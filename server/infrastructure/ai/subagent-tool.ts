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
  lifecycle: {
    event: (name, payload) => logger.info(`chat.subagent.${name}`, { operation: `chat.subagent.${name}`, outcome: payload.status ?? 'started', profile: payload.profile, depth: payload.depth })
  },
  execution: {
    async execute({ userId, profile, authority, context, budget, abortSignal, sessionId, model, approvals, permissionMode }) {
      const mcp = await buildMcpTools(userId, authority.tools, approvals ?? {}, permissionMode ?? (authority.working_mode === 'read-only' ? 'plan' : 'manual'))
      const tools = Object.fromEntries(Object.entries(mcp.tools).filter(([name]) => toolMatchesProfile(name, profile))) as ToolSet
      try {
        const response = await generateText({
          model: model as LanguageModel,
          system: `${profile.instructions}\nReturn JSON with keys status, summary, findings, evidence, validation, remaining_risks. Never include hidden reasoning.`,
          prompt: JSON.stringify(context),
          tools,
          toolChoice: 'auto',
          stopWhen: stepCountIs(budget.max_turns),
          maxOutputTokens: budget.max_output_tokens,
          timeout: { totalMs: budget.max_wall_time_ms },
          abortSignal
        })
        const result = parseResult(response.text, sessionId, profile.name, budget, abortSignal.aborted, response.steps?.length ?? 0)
        return result
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

function parseResult(text: string, sessionId: string, profile: SubagentResult['profile'], budget: SubagentBudget, cancelled: boolean, steps: number): SubagentResult {
  const value = (() => {
    try {
      return JSON.parse(text) as Partial<SubagentResult>
    } catch {
      return { summary: text }
    }
  })()
  return {
    status: cancelled ? 'cancelled' : steps >= budget.max_turns ? 'budget_exhausted' : value.status && ['completed', 'blocked', 'cancelled', 'budget_exhausted', 'failed', 'invalid'].includes(value.status) ? value.status : 'completed',
    summary: typeof value.summary === 'string' ? value.summary : 'Child completed without a summary.',
    findings: Array.isArray(value.findings) ? value.findings : [],
    evidence: Array.isArray(value.evidence) ? value.evidence : [],
    validation: Array.isArray(value.validation) ? value.validation : [],
    remaining_risks: Array.isArray(value.remaining_risks) ? value.remaining_risks : [],
    session_id: sessionId,
    profile,
    usage: { turns: steps, tool_calls: 0, output_tokens: budget.max_output_tokens, context_tokens: budget.max_context_tokens, wall_time_ms: 0, depth: 0 }
  }
}
