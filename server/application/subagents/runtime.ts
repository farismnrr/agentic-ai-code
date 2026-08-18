import { randomUUID } from 'node:crypto'
import { relative, resolve } from 'node:path'
import type { SubagentAuthority, SubagentBudget, SubagentContextPackage, SubagentProfile, SubagentRequest, SubagentResult, SubagentUsage } from '../../../shared/types/subagents.ts'
import { assertContainedChildPath, intersectSubagentAuthority, narrowBudget } from './policy.ts'
import { loadAgentProfile } from './profiles.ts'
import { getResultRef } from '../task-context-output.ts'

const MAX_TASK = 8192
const MAX_REFS = 32
const MAX_REF = 512
const MAX_SUMMARY = 4096
const MAX_SKILL_BYTES = 8192
const MAX_SKILLS = 16

export interface SubagentExecutionPort {
  execute(input: { userId: string, sessionId: string, parentSessionId: string, profile: SubagentProfile, authority: SubagentAuthority, context: SubagentContextPackage, budget: SubagentBudget, abortSignal: AbortSignal, model?: unknown, approvals?: Record<string, string>, permissionMode?: SubagentRequest['permission_mode'] }): Promise<Partial<SubagentResult> & { usage?: Partial<SubagentUsage>, allowStop?: (status: string) => Promise<boolean> }>
}

export interface SubagentLifecyclePort {
  event(name: 'subagent_start' | 'subagent_stop', payload: { session_id: string, parent_session_id: string, profile: string, status?: string, depth: number }): void
  allowStop?(payload: { session_id: string, parent_session_id: string, status: string }): Promise<boolean>
}

export interface SubagentRuntimeOptions {
  readProfile: (name: string) => string
  readSkill?: (name: string) => string | undefined
  execution: SubagentExecutionPort
  lifecycle?: SubagentLifecyclePort
  now?: () => number
}

export class SubagentRuntime {
  private readonly activeParents = new Set<string>()
  private readonly options: SubagentRuntimeOptions

  constructor(options: SubagentRuntimeOptions) { this.options = options }

  async run(request: SubagentRequest): Promise<SubagentResult> {
    const sessionId = randomUUID()
    const usage: SubagentUsage = { turns: 0, tool_calls: 0, output_tokens: 0, context_tokens: 0, wall_time_ms: 0, depth: request.depth ?? 0 }
    const invalid = (summary: string): SubagentResult => ({ status: 'invalid', summary, findings: [], evidence: [], validation: [], remaining_risks: [], session_id: sessionId, profile: request.profile, usage })
    if (!request.parent_session_id || ((request.depth !== 0) && (request.depth !== undefined)) || (!request.allow_concurrent_parent && this.activeParents.has(request.parent_session_id))) return invalid('Child delegation is unavailable for this parent state.')
    if (request.task.length === 0 || request.task.length > MAX_TASK) return invalid('Child task exceeds the bounded request limit.')
    if (!request.allow_concurrent_parent) this.activeParents.add(request.parent_session_id)
    const started = this.options.now?.() ?? Date.now()
    let profile: SubagentProfile | undefined
    try {
      profile = loadAgentProfile(request.profile, this.options.readProfile)
      const budget = narrowBudget(profile, request.budget)
      if (request.depth !== undefined && request.depth >= budget.max_depth) return invalid('Child recursion depth is denied.')
      const cwd = assertContainedChildPath(request.parent_authority.workspace_root, request.cwd ?? request.parent_authority.workspace_root)
      const skillInstructions = loadSkills(profile.skills, this.options.readSkill)
      const context = makeContext(request, cwd, budget, skillInstructions)
      const authority = intersectSubagentAuthority(request.parent_authority, profile)
      this.options.lifecycle?.event('subagent_start', { session_id: sessionId, parent_session_id: request.parent_session_id, profile: profile.name, depth: request.depth ?? 0 })
      const localAbort = new AbortController()
      const forwardAbort = () => localAbort.abort()
      request.abort_signal?.addEventListener('abort', forwardAbort, { once: true })
      const timeout = setTimeout(() => localAbort.abort(), budget.max_wall_time_ms)
      try {
        const raw = await this.options.execution.execute({ userId: request.user_id, sessionId, parentSessionId: request.parent_session_id, profile, authority, context, budget, abortSignal: localAbort.signal, model: request.model, approvals: request.approvals, permissionMode: request.permission_mode })
        usage.wall_time_ms = Math.min(Date.now() - started, budget.max_wall_time_ms)
        const observed = raw.usage ?? {}
        usage.turns = clampUsage(observed.turns, budget.max_turns)
        usage.tool_calls = clampUsage(observed.tool_calls, budget.max_tool_calls)
        usage.output_tokens = clampUsage(observed.output_tokens, budget.max_output_tokens)
        usage.context_tokens = clampUsage(observed.context_tokens, budget.max_context_tokens)
        usage.depth = Math.min(Math.max(0, request.depth ?? 0), budget.max_depth)
        const status = localAbort.signal.aborted ? 'cancelled' : (raw.status ?? 'completed')
        const result: SubagentResult = { status, summary: bound(raw.summary ?? 'Child completed without a summary.'), findings: boundList(raw.findings), evidence: boundEvidence(raw.evidence), validation: boundList(raw.validation), remaining_risks: boundList(raw.remaining_risks), session_id: sessionId, profile: profile.name, usage, summary_ref: typeof raw.summary_ref === 'string' ? raw.summary_ref : undefined }
        const lifecycleAllowed = raw.allowStop ? await raw.allowStop(result.status) : true
        const policyAllowed = this.options.lifecycle?.allowStop ? await this.options.lifecycle.allowStop({ session_id: sessionId, parent_session_id: request.parent_session_id, status: result.status }) : true
        if (!lifecycleAllowed || !policyAllowed) return { ...result, status: 'blocked', summary: 'Child completion was blocked by lifecycle policy.' }
        this.options.lifecycle?.event('subagent_stop', { session_id: sessionId, parent_session_id: request.parent_session_id, profile: profile.name, status: result.status, depth: request.depth ?? 0 })
        return result
      } finally {
        clearTimeout(timeout)
        request.abort_signal?.removeEventListener('abort', forwardAbort)
      }
    } catch {
      usage.wall_time_ms = Math.min(Date.now() - started, 180000)
      this.options.lifecycle?.event('subagent_stop', { session_id: sessionId, parent_session_id: request.parent_session_id, profile: profile?.name ?? request.profile, status: request.abort_signal?.aborted ? 'cancelled' : 'failed', depth: request.depth ?? 0 })
      return { ...invalid(request.abort_signal?.aborted ? 'Child execution was cancelled.' : 'Child execution failed.'), status: request.abort_signal?.aborted ? 'cancelled' : 'failed' }
    } finally {
      if (!request.allow_concurrent_parent) this.activeParents.delete(request.parent_session_id)
    }
  }
}

function makeContext(request: SubagentRequest, cwd: string, budget: SubagentBudget, skillInstructions: string[]): SubagentContextPackage {
  const references = [...new Set((request.context_refs ?? []).map(ref => ref.trim()).filter(Boolean))]
  if (references.length > MAX_REFS || references.some(ref => ref.length > MAX_REF)) throw new Error('child context references exceed bounds')
  const root = resolve(request.parent_authority.workspace_root)
  if (relative(root, cwd).startsWith('..')) throw new Error('invalid child scope')
  const resolvedReferences = references.map(reference => getResultRef(request.parent_session_id, reference) ? `${reference}: ${getResultRef(request.parent_session_id, reference)?.slice(0, 4096)}` : reference)
  const context = { task: request.task.slice(0, MAX_TASK), repository_identity: root, workspace_root: root, cwd, references: resolvedReferences, parent_summary: undefined, skill_instructions: skillInstructions }
  const encoded = JSON.stringify(context)
  if (Buffer.byteLength(encoded, 'utf8') > budget.max_context_tokens * 4) throw new Error('child context exceeds effective token bound')
  return context
}

function loadSkills(names: string[], readSkill?: (name: string) => string | undefined) {
  if (names.length === 0) return []
  if (names.length > MAX_SKILLS || names.some(name => !/^[a-z][a-z0-9-]{0,63}$/.test(name))) throw new Error('invalid profile skill reference')
  if (!readSkill) throw new Error('profile skills are unavailable')
  const loaded = names.map((name) => {
    const text = readSkill(name)
    if (!text) throw new Error('configured profile skill is unavailable')
    if (Buffer.byteLength(text, 'utf8') > MAX_SKILL_BYTES) throw new Error('configured profile skill exceeds size limit')
    return text.replaceAll(/\p{Cc}/gu, ' ')
  })
  if (loaded.reduce((total, text) => total + Buffer.byteLength(text, 'utf8'), 0) > MAX_SKILLS * MAX_SKILL_BYTES) throw new Error('configured profile skills exceed size limit')
  return loaded
}

function clampUsage(value: unknown, limit: number) {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? Math.min(Math.floor(value), limit) : 0
}

function bound(value: string): string {
  return value.replaceAll(/\p{Cc}/gu, ' ').slice(0, MAX_SUMMARY)
}
function boundList(values: unknown): string[] {
  if (!Array.isArray(values)) return []
  return values.filter((value): value is string => typeof value === 'string').slice(0, 32).map(bound)
}
function boundEvidence(values: unknown) {
  if (!Array.isArray(values)) return []
  return values.slice(0, 32).flatMap(value => typeof value === 'object' && value !== null && typeof (value as { reference?: unknown }).reference === 'string' && typeof (value as { detail?: unknown }).detail === 'string'
    ? [{ reference: bound((value as { reference: string }).reference), detail: bound((value as { detail: string }).detail) }]
    : [])
}
