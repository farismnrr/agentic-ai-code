/* eslint-disable @stylistic/max-statements-per-line */
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { WorktreeAllocator } from '../server/application/subagents/worktree.ts'
import { BackgroundTaskManager, BACKGROUND_CAPS } from '../server/application/subagents/background.ts'
import type { SubagentAuthority } from '../shared/types/subagents.ts'

const exec = promisify(execFile)
const git = async (args: string[], cwd: string) => (await exec('git', args, { cwd, env: { ...process.env, GIT_DIR: undefined, GIT_WORK_TREE: undefined, GIT_INDEX_FILE: undefined, GIT_COMMON_DIR: undefined } })).stdout.trim()
const root = await mkdtemp(join(tmpdir(), 'ai-code-039g-'))
await git(['init', '-b', 'main'], root)
await git(['remote', 'add', 'origin', 'https://github.com/farismnrr/ai-code.git'], root)
await git(['config', 'user.email', '039g@example.invalid'], root)
await git(['config', 'user.name', '039G acceptance'], root)
await writeFile(join(root, 'README.md'), 'fixture\n')
await mkdir(join(root, '.agents'), { recursive: true })
await writeFile(join(root, '.gitignore'), '.agents/worktrees/\n')
await git(['add', 'README.md', '.gitignore'], root)
await git(['commit', '-m', 'fixture'], root)
const allocator = new WorktreeAllocator()
const worktreeRoot = join(root, '.agents', 'worktrees')
const ownerInput = { repositoryRoot: root, worktreeRoot, taskId: 'task-one', parentSessionId: 'parent-one', userId: 'user-one' }
const one = await allocator.allocate(ownerInput)
const two = await allocator.allocate({ ...ownerInput, taskId: 'task-two' })
if (one.root === two.root || one.owner.branch === two.owner.branch) throw new Error('writer worktrees were not unique')
if (one.root === root || !one.root.startsWith(resolve(worktreeRoot) + '/')) throw new Error('writer worktree escaped configured root')
await writeFile(join(one.root, 'child.txt'), 'child only\n')
if (await readFile(join(root, 'README.md'), 'utf8') !== 'fixture\n') throw new Error('writer changed parent checkout')
if (await allocator.dispose(one.owner)) throw new Error('dirty writer worktree was removed')
if (!await allocator.dispose(two.owner)) throw new Error('clean disposable writer worktree was not removed')
const outside = await mkdtemp(join(tmpdir(), 'ai-code-039g-outside-'))
const link = join(root, '.agents', 'escape')
await mkdir(join(root, '.agents'), { recursive: true })
await symlink(outside, link)
try { await allocator.allocate({ ...ownerInput, taskId: 'escape', worktreeRoot: link }); throw new Error('symlink worktree root was accepted') } catch (error) { if (!(error instanceof Error) || !error.message.includes('escapes')) throw error }
await writeFile(join(root, 'dirty.txt'), 'do not transfer\n')
try { await allocator.allocate({ ...ownerInput, taskId: 'dirty-parent' }); throw new Error('dirty parent was accepted') } catch (error) { if (!(error instanceof Error) || !error.message.includes('clean parent')) throw error }

const authority: SubagentAuthority = { tools: ['file_read', 'file_write', 'apply_patch', 'terminal_exec'], effects: ['workspace_read', 'workspace_write', 'process_exec'], working_mode: 'workspace', model_policy: 'default', workspace_root: root }
const fakeRuntime = { run: async (request: { abort_signal: AbortSignal, profile: string, task?: string }) => {
  if (request.task === 'hold') await new Promise<void>(resolve => request.abort_signal.addEventListener('abort', () => resolve(), { once: true }))
  else await new Promise(resolve => setTimeout(resolve, 5))
  return { status: request.abort_signal.aborted ? 'cancelled' : 'completed', summary: 'fixture complete', findings: [], evidence: [], validation: [], remaining_risks: [], session_id: 'fixture', profile: request.profile, usage: { turns: 1, tool_calls: 1, output_tokens: 1, context_tokens: 1, wall_time_ms: 5, depth: 0 } }
} }
const manager = new BackgroundTaskManager(fakeRuntime as unknown as ConstructorParameters<typeof BackgroundTaskManager>[0], allocator)
const base = { user_id: 'user-one', parent_session_id: 'parent-one', parent_authority: authority, profile: 'explore' as const, task: 'read', isolation: 'shared_read' as const }
const a = manager.start(base); const b = manager.start({ ...base, task: 'read two' })
if (!a.task_id || !b.task_id || a.state !== 'queued' || b.state !== 'queued') throw new Error('background task IDs were not returned immediately')
if (manager.start({ ...base, task: 'over cap' }).state !== 'rejected') throw new Error('parent cap was not enforced')
if (manager.get(a.task_id, 'other-user', 'parent-one')) throw new Error('cross-user task access succeeded')
if (!manager.cancel(a.task_id, 'user-one', 'parent-one')) throw new Error('task cancellation failed')
await new Promise(resolve => setTimeout(resolve, 45))
if (manager.get(a.task_id, 'user-one', 'parent-one')?.state !== 'cancelled') throw new Error('cancellation was not retained')
if (manager.get(b.task_id, 'user-one', 'parent-one')?.state === 'cancelled') throw new Error('cancellation reset sibling task')
if (BACKGROUND_CAPS.global !== 4 || BACKGROUND_CAPS.perParent !== 2) throw new Error('unexpected bounded caps')

const waitForState = async (taskManager: BackgroundTaskManager, taskId: string, parentSessionId: string, state: string) => {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (taskManager.get(taskId, 'user-one', parentSessionId)?.state === state) return
    await new Promise(resolve => setTimeout(resolve, 5))
  }
  throw new Error(`task ${taskId} did not reach ${state}`)
}

// Keep one active entry alive while repeatedly filling the terminal set. This
// proves cardinality eviction never satisfies the cap by evicting active work.
const held = manager.start({ ...base, parent_session_id: 'active-parent', task: 'hold' })
if (!held.task_id) throw new Error('active retention fixture did not start')
const completedIds: string[] = []
for (let wave = 0; wave < 13; wave++) {
  const waveIds = [0, 1, 2].map(index => manager.start({ ...base, parent_session_id: `wave-${wave}-${index}`, task: `wave-${wave}-${index}` }).task_id)
  if (waveIds.some(id => !id)) throw new Error(`terminal wave ${wave} was rejected unexpectedly`)
  completedIds.push(...waveIds)
  await Promise.all(waveIds.map(id => waitForState(manager, id, `wave-${wave}-${waveIds.indexOf(id)}`, 'completed')))
  if (!manager.get(held.task_id, 'user-one', 'active-parent') || manager.get(held.task_id, 'user-one', 'active-parent')?.state === 'cancelled') throw new Error('active entry was evicted by terminal retention')
}
const retainedCount = completedIds.filter((id, index) => manager.get(id, 'user-one', `wave-${Math.floor(index / 3)}-${index % 3}`)).length
if (retainedCount > BACKGROUND_CAPS.retainedTerminal) throw new Error(`terminal retention exceeded cap: ${retainedCount}`)
if (manager.get(held.task_id, 'user-one', 'active-parent')?.state !== 'running') throw new Error('active task was not retained during terminal eviction')

// Deterministic TTL enforcement remains active independently of cardinality.
const clock = { value: Date.now() }
const ttlManager = new BackgroundTaskManager(fakeRuntime as unknown as ConstructorParameters<typeof BackgroundTaskManager>[0], allocator, () => clock.value)
const ttlTask = ttlManager.start({ ...base, parent_session_id: 'ttl-parent', task: 'ttl' })
await waitForState(ttlManager, ttlTask.task_id, 'ttl-parent', 'completed')
if (!ttlManager.get(ttlTask.task_id, 'user-one', 'ttl-parent')) throw new Error('fresh terminal task was not retained')
clock.value += BACKGROUND_CAPS.terminalTtlMs + 1
const ttlTrigger = ttlManager.start({ ...base, parent_session_id: 'ttl-trigger', task: 'ttl-trigger' })
if (ttlManager.get(ttlTask.task_id, 'user-one', 'ttl-parent')) throw new Error('stale terminal task survived TTL eviction')
await waitForState(ttlManager, ttlTrigger.task_id, 'ttl-trigger', 'completed')

await manager.cancel(held.task_id, 'user-one', 'active-parent')
await waitForState(manager, held.task_id, 'active-parent', 'cancelled')
await rm(outside, { recursive: true, force: true })
await rm(root, { recursive: true, force: true })
console.log('background/worktree behavioral acceptance: PASS')
