/* eslint-disable @stylistic/max-statements-per-line */
import { randomUUID } from 'node:crypto'
import { resolve } from 'node:path'
import type { BackgroundIsolation, BackgroundTaskMetadata, BackgroundTaskState, SubagentRequest } from '../../../shared/types/subagents.ts'
import type { SubagentRuntime } from './runtime.ts'
import { WorktreeAllocator, type WorktreeOwner } from './worktree.ts'

export const BACKGROUND_CAPS = { global: 4, perParent: 2, retainedTerminal: 32, terminalTtlMs: 15 * 60 * 1000, resultSummary: 4096 } as const
type Entry = BackgroundTaskMetadata & { controller: AbortController, owner?: WorktreeOwner }

export class BackgroundTaskManager {
  private readonly tasks = new Map<string, Entry>()
  private readonly runtime: SubagentRuntime
  private readonly worktrees: WorktreeAllocator
  private readonly now: () => number
  constructor(runtime: SubagentRuntime, worktrees = new WorktreeAllocator(), now = () => Date.now()) { this.runtime = runtime; this.worktrees = worktrees; this.now = now }

  start(input: Omit<SubagentRequest, 'abort_signal'> & { isolation: BackgroundIsolation, repositoryRoot?: string, worktreeRoot?: string, baseRef?: string }): { task_id: string, state: BackgroundTaskState } {
    this.evictTerminal()
    const active = [...this.tasks.values()].filter(task => !isTerminal(task.state))
    const parentActive = active.filter(task => task.parent_session_id === input.parent_session_id)
    if (active.length >= BACKGROUND_CAPS.global || parentActive.length >= BACKGROUND_CAPS.perParent) return { task_id: '', state: 'rejected' }
    if (input.isolation === 'worktree' && input.profile !== 'general-purpose') return { task_id: '', state: 'rejected' }
    if (input.isolation === 'shared_read' && input.profile === 'general-purpose') return { task_id: '', state: 'rejected' }
    const task_id = randomUUID()
    const entry: Entry = { task_id, parent_session_id: input.parent_session_id, user_id: input.user_id, agent_profile: input.profile, repository_identity: resolve(input.parent_authority.workspace_root), isolation: input.isolation, state: 'queued', progress_summary: 'Queued.', cleanup: input.isolation === 'worktree' ? 'preserved' : 'not_applicable', controller: new AbortController() }
    this.tasks.set(task_id, entry)
    void this.run(entry, input)
    return { task_id, state: 'queued' }
  }

  get(taskId: string, userId: string, parentSessionId: string): BackgroundTaskMetadata | undefined {
    const task = this.tasks.get(taskId)
    if (!task || task.user_id !== userId || task.parent_session_id !== parentSessionId) return undefined
    return publicTask(task)
  }

  cancel(taskId: string, userId: string, parentSessionId: string): boolean {
    const task = this.tasks.get(taskId)
    if (!task || task.user_id !== userId || task.parent_session_id !== parentSessionId || isTerminal(task.state)) return false
    task.state = 'cancelling'; task.progress_summary = 'Cancellation requested.'; task.controller.abort(); return true
  }

  private async run(entry: Entry, input: Parameters<BackgroundTaskManager['start']>[0]) {
    let request: SubagentRequest
    try {
      entry.state = 'starting'; entry.started_at = this.now()
      if (input.isolation === 'worktree') {
        if (!input.repositoryRoot || !input.worktreeRoot) throw new Error('writer worktree configuration is required')
        const allocation = await this.worktrees.allocate({ repositoryRoot: input.repositoryRoot, worktreeRoot: input.worktreeRoot, taskId: entry.task_id, parentSessionId: entry.parent_session_id, userId: entry.user_id, baseRef: input.baseRef })
        entry.owner = allocation.owner; entry.branch = allocation.owner.branch; entry.worktree_path = allocation.root
        request = { ...input, parent_authority: { ...input.parent_authority, workspace_root: allocation.root, working_mode: 'workspace' }, cwd: allocation.root }
      } else {
        request = { ...input, parent_authority: { ...input.parent_authority, effects: ['workspace_read', 'git_read'], tools: input.parent_authority.tools.filter(tool => !['file_write', 'file_edit', 'apply_patch', 'terminal_exec', 'local_terminal'].includes(tool)), working_mode: 'read-only' }, permission_mode: 'plan' }
      }
      entry.state = 'running'; entry.progress_summary = 'Child is running.'
      const result = await this.runtime.run({ ...request, abort_signal: entry.controller.signal, allow_concurrent_parent: true })
      if (entry.owner) {
        const evidence = await this.worktrees.evidence(entry.owner)
        result.evidence = [...result.evidence, { reference: 'git/status', detail: evidence.status }, { reference: 'git/diff-stat', detail: evidence.diff }, { reference: 'git/commits', detail: evidence.commits }].slice(0, 32)
        result.validation = [...result.validation, 'Writer worktree evidence collected; parent integration remains explicit.'].slice(0, 32)
      }
      entry.result = result; entry.state = result.status === 'completed' ? 'completed' : result.status === 'cancelled' ? 'cancelled' : result.status === 'budget_exhausted' ? 'budget_exhausted' : result.status === 'blocked' ? 'blocked' : 'failed'
      entry.progress_summary = result.summary
      if (entry.owner) entry.cleanup = 'preserved'
    } catch {
      entry.state = entry.controller.signal.aborted ? 'cancelled' : 'failed'; entry.progress_summary = entry.controller.signal.aborted ? 'Background task was cancelled.' : 'Background task failed.'
    } finally {
      entry.completed_at = this.now()
      // Terminal retention is a transition invariant, not a side effect of a
      // later task start. Keep active entries untouched while enforcing both
      // TTL and cardinality as soon as this child settles.
      this.evictTerminal()
    }
  }

  private evictTerminal() {
    const cutoff = this.now() - BACKGROUND_CAPS.terminalTtlMs
    for (const [id, task] of this.tasks) if (isTerminal(task.state) && (task.completed_at ?? 0) < cutoff) this.tasks.delete(id)
    const terminal = [...this.tasks.values()].filter(task => isTerminal(task.state)).sort((a, b) => (a.completed_at ?? 0) - (b.completed_at ?? 0))
    for (const task of terminal.slice(0, Math.max(0, terminal.length - BACKGROUND_CAPS.retainedTerminal))) this.tasks.delete(task.task_id)
  }
}

function isTerminal(state: BackgroundTaskState) { return ['completed', 'failed', 'cancelled', 'blocked', 'rejected', 'budget_exhausted'].includes(state) }
function publicTask(task: Entry): BackgroundTaskMetadata { const { controller: _, owner, ...safe } = task; return { ...safe, branch: owner?.branch, worktree_path: owner?.worktree_path } }
