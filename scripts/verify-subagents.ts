import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { intersectSubagentAuthority, narrowBudget } from '../server/application/subagents/policy.ts'
import { parseAgentProfile } from '../server/application/subagents/profiles.ts'
import { SubagentRuntime } from '../server/application/subagents/runtime.ts'
import type { SubagentAuthority } from '../shared/types/subagents.ts'

const root = process.cwd()
const parent: SubagentAuthority = { tools: ['file_read', 'file_write', 'local_terminal'], effects: ['workspace_read', 'workspace_write', 'process_exec'], working_mode: 'workspace', model_policy: 'default', workspace_root: root }
const profile = parseAgentProfile(readFileSync(join(root, '.agents/agents/explore.md'), 'utf8'))
if (intersectSubagentAuthority(parent, profile).effects.includes('workspace_write')) throw new Error('explore widened write authority')
if (intersectSubagentAuthority({ ...parent, effects: ['workspace_read'] }, parseAgentProfile(readFileSync(join(root, '.agents/agents/general-purpose.md'), 'utf8'))).effects.includes('workspace_write')) throw new Error('profile widened parent deny')
if (narrowBudget(profile, { max_turns: 999 }).max_turns !== profile.max_turns) throw new Error('budget override widened profile')
try {
  parseAgentProfile('---\nname: explore\ntools: {allow: [unknown], deny: []}\n---\nno')
  throw new Error('unknown capability accepted')
} catch (error) {
  if ((error as Error).message === 'unknown capability accepted') throw error
}

let active = 0
const runtime = new SubagentRuntime({
  readProfile: name => readFileSync(join(root, `.agents/agents/${name}.md`), 'utf8'),
  execution: { execute: async ({ abortSignal }) => {
    active++
    if (active > 1) throw new Error('concurrent child')
    await new Promise(resolve => setTimeout(resolve, 20))
    active--
    return { status: abortSignal.aborted ? 'cancelled' : 'completed', summary: 'bounded', findings: [], evidence: [], validation: [], remaining_risks: [], usage: { turns: 1, tool_calls: 0 } }
  } }
})
const request = { user_id: 'test', parent_session_id: 'parent', parent_authority: parent, profile: 'explore' as const, task: 'inspect', depth: 0 }
const [first, second] = await Promise.all([runtime.run(request), runtime.run(request)])
if (![first.status, second.status].includes('invalid')) throw new Error('parent child slot was not exclusive')
if ((await runtime.run({ ...request, parent_session_id: 'cancelled', abort_signal: AbortSignal.timeout(1) })).status !== 'cancelled') throw new Error('cancellation was not represented')
console.log('subagent behavioral acceptance: PASS')
