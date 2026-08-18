import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { generateText, stepCountIs, tool, type LanguageModel, type ToolSet } from 'ai'
import { z } from 'zod'
import type { SubagentToolPort } from '../../application/chat/contracts'
import type { SubagentBudget, SubagentResult } from '#shared/types/subagents'
import { SubagentRuntime } from '../../application/subagents/runtime'
import { toolMatchesProfile } from '../../application/subagents/profiles'
import { buildMcpTools } from '../mcp/mcp-tools'
import { logger } from '../observability/logger'

const runtime = new SubagentRuntime({
  readProfile: name => readFileSync(join(process.cwd(), '.agents', 'agents', `${name}.md`), 'utf8'),
  readSkill: (name) => {
    const candidates = [join(process.cwd(), 'ai-self', 'skills', name, 'SKILL.md'), join(process.cwd(), '.agents', 'skills', name, 'SKILL.md')]
    for (const candidate of candidates) {
      try {
        return readFileSync(candidate, 'utf8')
      } catch {
        // Try the next approved repository location.
      }
    }
    return undefined
  },
  lifecycle: {
    event: (name, payload) => logger.info(`chat.subagent.${name}`, { operation: `chat.subagent.${name}`, outcome: payload.status ?? 'started', profile: payload.profile, depth: payload.depth })
  },
  // The parent-resolved model is the only model available at this composition
  // edge. Profile fast/default/strong values therefore remain vendor-neutral,
  // advisory hints; they never claim a provider/model switch that was not
  // resolved from the user's configured model set.
  execution: {
    async execute({ userId, parentSessionId, profile, authority, context, budget, abortSignal, sessionId, model, approvals, permissionMode }) {
      const mcp = await buildMcpTools(userId, authority.tools, approvals ?? {}, permissionMode ?? (authority.working_mode === 'read-only' ? 'plan' : 'manual'), { allowedEffects: authority.effects, maxToolCalls: budget.max_tool_calls })
      const tools = Object.fromEntries(Object.entries(mcp.tools).filter(([name]) => toolMatchesProfile(name, profile))) as ToolSet
      try {
        const response = await generateText({
          model: model as LanguageModel,
          system: `${profile.instructions}\n${(context.skill_instructions ?? []).join('\n')}\nReturn JSON with keys status, summary, findings, evidence, validation, remaining_risks. Never include hidden reasoning.`,
          prompt: JSON.stringify({ ...context, skill_instructions: undefined }),
          tools,
          toolApproval: mcp.toolApproval,
          toolChoice: 'auto',
          stopWhen: stepCountIs(budget.max_turns),
          maxOutputTokens: budget.max_output_tokens,
          timeout: { totalMs: budget.max_wall_time_ms },
          abortSignal
        })
        const approvalPending = Array.isArray(response.content) && response.content.some(part => typeof part === 'object' && part !== null && ['tool-approval-request', 'tool-output-denied'].includes((part as { type?: unknown }).type as string))
        const result = parseResult(response.text, sessionId, profile.name, budget, abortSignal.aborted, response.steps?.length ?? 0, mcp.toolCallCount(), response.usage, approvalPending)
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

function parseResult(text: string, sessionId: string, profile: SubagentResult['profile'], budget: SubagentBudget, cancelled: boolean, steps: number, toolCalls: number, providerUsage?: { inputTokens?: number, outputTokens?: number, totalTokens?: number }, approvalPending = false): SubagentResult {
  const value = (() => {
    try {
      return JSON.parse(text) as Partial<SubagentResult>
    } catch {
      return { summary: text }
    }
  })()
  return {
    status: cancelled ? 'cancelled' : approvalPending ? 'blocked' : steps >= budget.max_turns ? 'budget_exhausted' : value.status && ['completed', 'blocked', 'cancelled', 'budget_exhausted', 'failed', 'invalid'].includes(value.status) ? value.status : 'completed',
    summary: approvalPending ? 'Child tool call requires approval before execution.' : typeof value.summary === 'string' ? value.summary : 'Child completed without a summary.',
    findings: Array.isArray(value.findings) ? value.findings : [],
    evidence: Array.isArray(value.evidence) ? value.evidence : [],
    validation: Array.isArray(value.validation) ? value.validation : [],
    remaining_risks: Array.isArray(value.remaining_risks) ? value.remaining_risks : [],
    session_id: sessionId,
    profile,
    usage: { turns: Math.min(steps, budget.max_turns), tool_calls: Math.min(toolCalls, budget.max_tool_calls), output_tokens: Math.min(providerUsage?.outputTokens ?? 0, budget.max_output_tokens), context_tokens: Math.min(providerUsage?.inputTokens ?? 0, budget.max_context_tokens), wall_time_ms: 0, depth: 0 }
  }
}
