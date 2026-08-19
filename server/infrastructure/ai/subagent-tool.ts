import { generateText, stepCountIs, tool, type LanguageModel, type ToolSet } from 'ai'
import { z } from 'zod'
import type { SubagentToolPort } from '../../application/chat/contracts'
import type { SubagentBudget, SubagentResult } from '#shared/types/subagents'
import { SubagentRuntime } from '../../application/subagents/runtime'
import { loadAgentProfile, nativeToolMatchesProfile } from '../../application/subagents/profiles'
import { intersectSubagentAuthority } from '../../application/subagents/policy'
import { OrchestratorScheduler, ORCHESTRATOR_BUDGETS, ORCHESTRATOR_ROLE_PROFILE, requirementsFitAuthority } from '../../application/orchestration/scheduler'
import { getOrchestratorGraph } from '../../application/orchestration/task-graph'
import { advanceWriter, markDelivered, reconcileChildren } from '../../application/orchestration/reconciliation'
import { buildMcpTools, scopeMcpTools } from '../mcp/mcp-tools'
import { logger } from '../observability/logger'
import { BackgroundTaskManager } from '../../application/subagents/background'
import { classifyOutput, putResultRef } from '../../application/task-context-output'
import { parsePresentationSafeSubagentResult, presentationSafeBackgroundTask } from './subagent-result'
import { readRuntimeInstruction } from './runtime-instructions'

function readProfileInstruction(name: string) {
  const loaded = readRuntimeInstruction(process.cwd(), ['.agents', 'agents'], [`${name}.md`])
  if (!loaded) throw new Error('configured profile is unavailable')
  return loaded.text
}

function readSkillInstruction(name: string) {
  const found = new Map<string, string>()
  for (const root of [['ai-self', 'skills'], ['.agents', 'skills']]) {
    const loaded = readRuntimeInstruction(process.cwd(), root, [name, 'SKILL.md'], { optional: true })
    if (loaded && !found.has(loaded.canonical)) found.set(loaded.canonical, loaded.text)
  }
  if (found.size > 1) throw new Error('configured profile skill is ambiguous across approved roots')
  return found.values().next().value
}

const runtime = new SubagentRuntime({
  readProfile: readProfileInstruction,
  readSkill: readSkillInstruction,
  lifecycle: {
    event: (name, payload) => logger.info(`chat.subagent.${name}`, { 'operation': `chat.subagent.${name}`, 'outcome': payload.status ?? 'started', 'agent.profile': payload.profile, 'agent.state': payload.status ?? 'started', 'agent.depth': payload.depth })
  },
  // The parent-resolved model is the only model available at this composition
  // edge. Profile fast/default/strong values therefore remain vendor-neutral,
  // advisory hints; they never claim a provider/model switch that was not
  // resolved from the user's configured model set.
  execution: {
    async execute({ userId, parentSessionId, profile, authority, context, budget, abortSignal, sessionId, model, approvals, permissionMode }) {
      const mcp = await buildMcpTools(userId, authority.tools, approvals ?? {}, permissionMode ?? (authority.working_mode === 'read-only' ? 'plan' : 'manual'), { allowedEffects: authority.effects, maxToolCalls: budget.max_tool_calls, abortSignal })
      const scopedMcp = scopeMcpTools(mcp, new Set(authority.tools))
      const tools = Object.fromEntries(Object.entries(scopedMcp.tools).filter(([name]) => scopedMcp.toolOwners.has(name) || nativeToolMatchesProfile(name, profile))) as ToolSet
      try {
        const response = await generateText({
          model: model as LanguageModel,
          system: `${profile.instructions}\n${(context.skill_instructions ?? []).join('\n')}\nReturn JSON with keys status, summary, findings, evidence, validation, remaining_risks. Never include hidden reasoning.`,
          prompt: JSON.stringify({ ...context, skill_instructions: undefined }),
          tools,
          toolApproval: scopedMcp.toolApproval,
          toolChoice: 'auto',
          stopWhen: stepCountIs(budget.max_turns),
          maxOutputTokens: budget.max_output_tokens,
          timeout: { totalMs: budget.max_wall_time_ms },
          abortSignal
        })
        const approvalPending = Array.isArray(response.content) && response.content.some(part => typeof part === 'object' && part !== null && ['tool-approval-request', 'tool-output-denied'].includes((part as { type?: unknown }).type as string))
        const result = parseResult(response.text, sessionId, parentSessionId, profile.name, budget, abortSignal.aborted, response.steps?.length ?? 0, mcp.toolCallCount(), response.usage, approvalPending)
        return { ...result, allowStop: (status: string) => mcp.subagentStop(parentSessionId, sessionId, status) }
      } catch (error) {
        if (error instanceof Error && error.message === 'subagent tool-call budget exhausted') return { status: 'budget_exhausted', summary: 'Child tool-call budget exhausted.', usage: { tool_calls: mcp.toolCallCount() }, allowStop: (status: string) => mcp.subagentStop(parentSessionId, sessionId, status) }
        return { status: abortSignal.aborted ? 'cancelled' : 'failed', summary: abortSignal.aborted ? 'Child execution was cancelled.' : 'Child execution failed.', usage: { tool_calls: mcp.toolCallCount() }, allowStop: (status: string) => mcp.subagentStop(parentSessionId, sessionId, status) }
      } finally {
        await mcp.close()
      }
    }
  }
})

const backgroundTasks = new BackgroundTaskManager(runtime)
const orchestratorScheduler = new OrchestratorScheduler()

export function buildSubagentTool(input: Parameters<SubagentToolPort['build']>[0]) {
  return tool({
    description: 'Delegate one focused task to a named, bounded child profile. Parent-only; sequential; returns a structured evidence summary.',
    inputSchema: z.object({
      agent: z.enum(['explore', 'plan', 'review', 'verify', 'general-purpose']),
      task: z.string().min(1).max(8192),
      cwd: z.string().max(4096).optional(),
      context_refs: z.array(z.string().max(512)).max(32).optional(),
      budget: z.object({ max_turns: z.number().int().positive().optional(), max_tool_calls: z.number().int().positive().optional(), max_output_tokens: z.number().int().positive().optional(), max_context_tokens: z.number().int().positive().optional(), max_wall_time_ms: z.number().int().positive().optional() }).optional()
    }),
    execute: async ({ agent, task, cwd, context_refs, budget }) => runtime.run({ user_id: input.userId, parent_session_id: input.parentSessionId, parent_authority: input.authority, profile: agent, task, cwd, context_refs, budget, depth: 0, abort_signal: input.abortSignal, model: input.model, approvals: input.approvals, permission_mode: input.permissionMode })
  })
}

export function buildBackgroundTaskTools(input: Parameters<SubagentToolPort['build']>[0]) {
  const common = { user_id: input.userId, parent_session_id: input.parentSessionId, parent_authority: input.authority, profile: 'explore' as const, task: 'background task', isolation: 'shared_read' as const }
  return {
    agent_task_start: tool({
      description: 'Start a bounded parent-managed background agent. Read-only tasks use shared_read; writers require a dedicated worktree.',
      inputSchema: z.object({ agent: z.enum(['explore', 'review', 'plan', 'general-purpose']), task: z.string().min(1).max(8192), isolation: z.enum(['shared_read', 'worktree']), context_refs: z.array(z.string().max(512)).max(32).optional() }),
      execute: async ({ agent, task, isolation, context_refs }) => {
        const result = backgroundTasks.start({ ...common, profile: agent, task, isolation, context_refs, repositoryRoot: input.authority.workspace_root, worktreeRoot: `${input.authority.workspace_root}/.agents/worktrees`, model: input.model, approvals: input.approvals, permission_mode: input.permissionMode })
        logger.info('chat.background.start', { 'operation': 'chat.background.start', 'outcome': result.state === 'rejected' ? 'denied' : 'ok', 'agent.profile': agent, 'background.isolation': isolation, 'background.state': result.state })
        return result
      }
    }),
    agent_task_get: tool({
      description: 'Get bounded status and result for a parent-owned background agent task.',
      inputSchema: z.object({ task_id: z.string().uuid() }),
      execute: async ({ task_id }) => {
        const raw = backgroundTasks.get(task_id, input.userId, input.parentSessionId)
        const result = raw ? presentationSafeBackgroundTask(raw) : { state: 'not_found' as const }
        logger.info('chat.background.get', { 'operation': 'chat.background.get', 'outcome': result.state === 'not_found' ? 'error' : 'ok', 'background.state': result.state })
        return result
      }
    }),
    agent_task_cancel: tool({
      description: 'Cancel a parent-owned background agent task. Dirty writer worktrees remain available for inspection.',
      inputSchema: z.object({ task_id: z.string().uuid() }),
      execute: async ({ task_id }) => {
        const cancelled = backgroundTasks.cancel(task_id, input.userId, input.parentSessionId)
        logger.info('chat.background.cancel', { 'operation': 'chat.background.cancel', 'outcome': cancelled ? 'cancelled' : 'error', 'background.state': cancelled ? 'cancelling' : 'not_found', 'cancel.reason': cancelled ? 'user-request' : 'not-found' })
        return { task_id, cancelled }
      }
    })
  }
}

export function buildOrchestratorTools(input: Parameters<SubagentToolPort['build']>[0]) {
  const port = {
    capacity: (parentSessionId: string) => backgroundTasks.capacity(parentSessionId),
    prepare: (node: Parameters<typeof requirementsFitAuthority>[0], parentAuthority: Parameters<typeof requirementsFitAuthority>[1]) => {
      const profileName = ORCHESTRATOR_ROLE_PROFILE[node.role]
      if (node.profile && node.profile !== profileName) throw new Error('orchestrator role/profile mismatch')
      const profile = loadAgentProfile(profileName, readProfileInstruction)
      const childAuthority = intersectSubagentAuthority(parentAuthority, profile)
      if (!requirementsFitAuthority(node, childAuthority)) throw new Error('orchestrator child authority is insufficient')
      if (node.role === 'writer' && childAuthority.working_mode !== 'workspace') throw new Error('writer authority is read-only')
      if (node.role !== 'writer' && childAuthority.working_mode !== 'read-only') throw new Error('non-writer authority is not read-only')
      return { profile: profileName, isolation: node.role === 'writer' ? 'worktree' as const : 'shared_read' as const, budget: ORCHESTRATOR_BUDGETS[node.budget_class] }
    },
    start: ({ taskId, node, prepared }: { taskId: string, node: Parameters<typeof requirementsFitAuthority>[0], prepared: { profile: SubagentResult['profile'], isolation: 'shared_read' | 'worktree', budget: Partial<SubagentBudget> } }) => backgroundTasks.start({
      taskId,
      user_id: input.userId,
      parent_session_id: input.parentSessionId,
      parent_authority: input.authority,
      profile: prepared.profile,
      task: node.objective,
      budget: prepared.budget,
      isolation: prepared.isolation,
      repositoryRoot: input.authority.workspace_root,
      worktreeRoot: `${input.authority.workspace_root}/.agents/worktrees`,
      model: input.model,
      approvals: input.approvals,
      permission_mode: input.permissionMode
    }),
    get: (taskId: string) => backgroundTasks.get(taskId, input.userId, input.parentSessionId),
    cancel: (taskId: string) => backgroundTasks.cancel(taskId, input.userId, input.parentSessionId)
  }

  const cancelActiveRun = () => {
    const graph = getOrchestratorGraph(input.userId, input.parentSessionId)
    if (!graph || graph.status !== 'active') return
    orchestratorScheduler.cancelRun({ userId: input.userId, conversationId: input.parentSessionId, generation: graph.generation, port })
  }
  if (input.abortSignal.aborted) cancelActiveRun()
  else input.abortSignal.addEventListener('abort', cancelActiveRun, { once: true })

  return {
    orchestrator_dispatch: tool({
      description: 'Dispatch dependency-ready orchestration nodes through the existing bounded background/subagent runtime. Writer nodes use isolated worktrees; other roles stay read-only.',
      inputSchema: z.object({ generation: z.string().uuid() }),
      execute: async ({ generation }) => {
        orchestratorScheduler.poll({ userId: input.userId, conversationId: input.parentSessionId, generation, port })
        const result = orchestratorScheduler.dispatchReady({ userId: input.userId, conversationId: input.parentSessionId, generation, parentSessionId: input.parentSessionId, parentAuthority: input.authority, port })
        logger.info('chat.orchestrator.dispatch', { 'operation': 'chat.orchestrator.dispatch', 'outcome': 'ok', 'orchestration.run_id': generation, 'orchestration.started.count': result.started.length, 'orchestration.denied.count': result.denied.length, 'orchestration.ready.count': result.graph.ready.length })
        return result
      }
    }),
    orchestrator_poll: tool({
      description: 'Refresh parent-owned orchestration state from bounded child task results without starting new work.',
      inputSchema: z.object({ generation: z.string().uuid() }),
      execute: async ({ generation }) => {
        const graph = orchestratorScheduler.poll({ userId: input.userId, conversationId: input.parentSessionId, generation, port })
        logger.info('chat.orchestrator.poll', { 'operation': 'chat.orchestrator.poll', 'outcome': graph.status, 'orchestration.run_id': generation, 'orchestration.state': graph.status, 'orchestration.ready.count': graph.ready.length, 'orchestration.running.count': graph.nodes.filter(node => node.status === 'running').length })
        return graph
      }
    }),
    orchestrator_cancel: tool({
      description: 'Cancel one orchestration node, its dependency subtree, or the whole parent-owned run. Running child process trees receive cancellation through the existing background runtime.',
      inputSchema: z.object({ generation: z.string().uuid(), scope: z.enum(['node', 'subtree', 'run']), node_id: z.string().min(1).max(64).optional() }),
      execute: async ({ generation, scope, node_id }) => {
        if (scope !== 'run' && !node_id) throw new Error('orchestrator cancellation target is required')
        const graph = scope === 'run'
          ? orchestratorScheduler.cancelRun({ userId: input.userId, conversationId: input.parentSessionId, generation, port })
          : orchestratorScheduler.cancelNode({ userId: input.userId, conversationId: input.parentSessionId, generation, nodeId: node_id!, subtree: scope === 'subtree', port })
        logger.info('chat.orchestrator.cancel', { 'operation': 'chat.orchestrator.cancel', 'outcome': 'cancelled', 'cancel.reason': scope, 'orchestration.run_id': generation, 'orchestration.state': graph.status, 'orchestration.running.count': graph.nodes.filter(node => node.status === 'running').length })
        return graph
      }
    }),
    orchestrator_reconcile: tool({
      description: 'Collect terminal child evidence into a bounded parent-owned reconciliation ledger. Duplicate findings are deduplicated; disagreements and P0/P1 findings block delivery.',
      inputSchema: z.object({ generation: z.string().uuid(), task_ids: z.array(z.string().uuid()).min(1).max(24) }),
      execute: async ({ generation, task_ids }) => {
        const unique = [...new Set(task_ids)]
        if (unique.length !== task_ids.length) throw new Error('duplicate reconciliation task id')
        const children = []
        for (const taskId of unique) {
          const child = await backgroundTasks.reconciliation(taskId, input.userId, input.parentSessionId)
          if (!child) throw new Error('reconciliation child is unavailable')
          children.push(child)
        }
        const result = reconcileChildren({ userId: input.userId, conversationId: input.parentSessionId, generation, children })
        logger.info('chat.orchestrator.reconcile', { 'operation': 'chat.orchestrator.reconcile', 'outcome': result.blockers.length ? 'blocked' : 'ok', 'orchestration.run_id': generation, 'orchestration.reconciliation_outcome': result.blockers.length ? 'blocked' : 'clear', 'orchestration.issue.count': result.issues.length, 'orchestration.blocker.count': result.blockers.length })
        return result
      }
    }),
    orchestrator_writer_transition: tool({
      description: 'Advance one writer through reviewed, accepted, then integrated states only when its bounded worktree HEAD evidence still matches.',
      inputSchema: z.object({ generation: z.string().uuid(), task_id: z.string().uuid(), expected_head: z.string().regex(/^[0-9a-f]{40}$/i), action: z.enum(['review', 'accept', 'integrate']) }),
      execute: async ({ generation, task_id, expected_head, action }) => advanceWriter({ userId: input.userId, conversationId: input.parentSessionId, generation, taskId: task_id, expectedHead: expected_head, action })
    }),
    orchestrator_mark_delivered: tool({
      description: 'Mark reconciliation delivered only after all writer work is integrated and no high-severity finding or reviewer disagreement remains. Actual delivery must already use Plan-040 Git/forge tools.',
      inputSchema: z.object({ generation: z.string().uuid() }),
      execute: async ({ generation }) => markDelivered({ userId: input.userId, conversationId: input.parentSessionId, generation })
    })
  }
}

function parseResult(text: string, sessionId: string, owner: string, profile: SubagentResult['profile'], budget: SubagentBudget, cancelled: boolean, steps: number, toolCalls: number, providerUsage?: { inputTokens?: number, outputTokens?: number, totalTokens?: number }, approvalPending = false): SubagentResult {
  const value = parsePresentationSafeSubagentResult(text)
  const summary = approvalPending ? 'Child tool call requires approval before execution.' : value?.summary ?? 'Child returned an invalid structured summary.'
  const summaryRef = classifyOutput(Buffer.byteLength(summary, 'utf8')) === 'summarized_large' ? putResultRef(owner, summary.slice(0, 32 * 1024)) : undefined
  return {
    status: cancelled ? 'cancelled' : approvalPending ? 'blocked' : steps >= budget.max_turns ? 'budget_exhausted' : value?.status ?? 'invalid',
    summary: summaryRef ? `${summary.slice(0, 512)} …[full bounded summary in result_ref]` : summary,
    findings: value?.findings ?? [],
    evidence: value?.evidence ?? [],
    validation: value?.validation ?? [],
    remaining_risks: value?.remaining_risks ?? [],
    session_id: sessionId,
    profile,
    usage: { turns: Math.min(steps, budget.max_turns), tool_calls: Math.min(toolCalls, budget.max_tool_calls), output_tokens: Math.min(providerUsage?.outputTokens ?? 0, budget.max_output_tokens), context_tokens: Math.min(providerUsage?.inputTokens ?? 0, budget.max_context_tokens), wall_time_ms: 0, depth: 0 },
    summary_ref: summaryRef
  }
}
