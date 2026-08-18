import { generateText, stepCountIs, tool, type LanguageModel, type ToolSet } from 'ai'
import { z } from 'zod'
import type { SubagentToolPort } from '../../application/chat/contracts'
import type { SubagentBudget, SubagentResult } from '#shared/types/subagents'
import { SubagentRuntime } from '../../application/subagents/runtime'
import { nativeToolMatchesProfile } from '../../application/subagents/profiles'
import { buildMcpTools, scopeMcpTools } from '../mcp/mcp-tools'
import { logger } from '../observability/logger'
import { BackgroundTaskManager } from '../../application/subagents/background'
import { classifyOutput, putResultRef } from '../../application/task-context-output'
import { parsePresentationSafeSubagentResult, presentationSafeBackgroundTask } from './subagent-result'
import { readRuntimeInstruction } from './runtime-instructions'

const runtime = new SubagentRuntime({
  readProfile: (name) => {
    const loaded = readRuntimeInstruction(process.cwd(), ['.agents', 'agents'], [`${name}.md`])
    if (!loaded) throw new Error('configured profile is unavailable')
    return loaded.text
  },
  readSkill: (name) => {
    const found = new Map<string, string>()
    for (const root of [['ai-self', 'skills'], ['.agents', 'skills']]) {
      const loaded = readRuntimeInstruction(process.cwd(), root, [name, 'SKILL.md'], { optional: true })
      if (loaded && !found.has(loaded.canonical)) found.set(loaded.canonical, loaded.text)
    }
    if (found.size > 1) throw new Error('configured profile skill is ambiguous across approved roots')
    return found.values().next().value
  },
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
