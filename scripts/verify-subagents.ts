import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { intersectSubagentAuthority, narrowBudget } from '../server/application/subagents/policy.ts'
import { parseAgentProfile } from '../server/application/subagents/profiles.ts'
import { SubagentRuntime } from '../server/application/subagents/runtime.ts'
import type { SubagentAuthority } from '../shared/types/subagents.ts'
import { toolRequiresEffects } from '../shared/utils/capability-policy.ts'

const root = process.cwd()
const parent: SubagentAuthority = { tools: ['file_read', 'file_write', 'local_terminal', 'terminal_exec'], effects: ['workspace_read', 'workspace_write', 'process_exec', 'network_read', 'external_mutation'], working_mode: 'workspace', model_policy: 'default', workspace_root: root }
const profile = parseAgentProfile(readFileSync(join(root, '.agents/agents/explore.md'), 'utf8'))
if (intersectSubagentAuthority(parent, profile).effects.includes('workspace_write')) throw new Error('explore widened write authority')
if (intersectSubagentAuthority({ ...parent, effects: ['workspace_read'] }, parseAgentProfile(readFileSync(join(root, '.agents/agents/general-purpose.md'), 'utf8'))).effects.includes('workspace_write')) throw new Error('profile widened parent deny')
if (narrowBudget(profile, { max_turns: 999 }).max_turns !== profile.max_turns) throw new Error('budget override widened profile')
try {
  narrowBudget(profile, { max_tool_calls: 0 })
  throw new Error('zero budget accepted')
} catch (error) {
  if ((error as Error).message === 'zero budget accepted') throw error
}
try {
  parseAgentProfile('---\nname: explore\ntools: {allow: [unknown], deny: []}\n---\nno')
  throw new Error('unknown capability accepted')
} catch (error) {
  if ((error as Error).message === 'unknown capability accepted') throw error
}

const writeDeniedProfile = parseAgentProfile(readFileSync(join(root, '.agents/agents/general-purpose.md'), 'utf8').replace('workspace_write, workspace_delete, git_read', 'workspace_delete, git_read'))
const narrowed = intersectSubagentAuthority(parent, writeDeniedProfile)
if (narrowed.tools.includes('file_write')) throw new Error('effect denial did not remove file_write')
if (!toolRequiresEffects('file_read').includes('workspace_read')) throw new Error('read effect classification missing')
if (!toolRequiresEffects('unknown_tool').includes('privileged_bridge')) throw new Error('unknown effect did not fail closed')
const verify = parseAgentProfile(readFileSync(join(root, '.agents/agents/verify.md'), 'utf8'))
const verifyAuthority = intersectSubagentAuthority(parent, verify)
if (!verifyAuthority.tools.includes('terminal_exec')) throw new Error('verify terminal execution path unavailable')
if (verifyAuthority.tools.some(tool => tool === 'file_write' || tool === 'file_edit' || tool === 'apply_patch')) throw new Error('verify gained source mutation tools')
const plan = parseAgentProfile(readFileSync(join(root, '.agents/agents/plan.md'), 'utf8'))
if (plan.skills.length !== 1 || plan.skills[0] !== 'implementation-planning') throw new Error('plan skill contract changed')

let active = 0
let stopCalls = 0
let stopAllowed = true
const runtime = new SubagentRuntime({
  readProfile: name => readFileSync(join(root, `.agents/agents/${name}.md`), 'utf8'),
  readSkill: name => name === 'implementation-planning' ? readFileSync(join(root, 'ai-self/skills/implementation-planning/SKILL.md'), 'utf8') : undefined,
  lifecycle: { event: () => {}, allowStop: async () => true },
  execution: { execute: async ({ abortSignal, context, budget }) => {
    active++
    if (active > 1) throw new Error('concurrent child')
    await new Promise(resolve => setTimeout(resolve, 20))
    active--
    if (context.skill_instructions?.length && context.skill_instructions[0].length === 0) throw new Error('skill was empty')
    return {
      status: abortSignal.aborted ? 'cancelled' : 'completed', summary: 'bounded', findings: [], evidence: [], validation: [], remaining_risks: [],
      usage: { turns: 1, tool_calls: Math.min(1, budget.max_tool_calls), output_tokens: 3, context_tokens: 4 },
      allowStop: async () => {
        stopCalls++
        return stopAllowed
      }
    }
  } }
})
const request = { user_id: 'test', parent_session_id: 'parent', parent_authority: parent, profile: 'explore' as const, task: 'inspect', depth: 0 }
const [first, second] = await Promise.all([runtime.run(request), runtime.run(request)])
if (![first.status, second.status].includes('invalid')) throw new Error('parent child slot was not exclusive')
if ((await runtime.run({ ...request, parent_session_id: 'cancelled', abort_signal: AbortSignal.timeout(1) })).status !== 'cancelled') throw new Error('cancellation was not represented')
const beforeStop = stopCalls
const stopped = await runtime.run({ ...request, parent_session_id: 'stop-blocked' })
if (stopped.status !== 'completed' || stopCalls !== beforeStop + 1) throw new Error('completion lifecycle did not run exactly once')
stopAllowed = false
const beforeBlocked = stopCalls
if ((await runtime.run({ ...request, parent_session_id: 'stop-denied' })).status !== 'blocked' || stopCalls !== beforeBlocked + 1) throw new Error('blocking completion lifecycle was not fail-closed')
if ((await runtime.run({ ...request, parent_session_id: 'planned', profile: 'plan', budget: { max_context_tokens: 1 } })).status !== 'failed') throw new Error('oversized context was not bounded')
console.log('subagent behavioral acceptance: PASS')
