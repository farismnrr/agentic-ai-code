import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import type { BackgroundTaskMetadata, BackgroundTaskState, SubagentAuthority, SubagentEffect } from '../../shared/types/subagents.ts'
import type { OrchestratorNode } from '../../shared/types/orchestration.ts'
import { loadAgentProfile } from '../../server/application/subagents/profiles.ts'
import { intersectSubagentAuthority } from '../../server/application/subagents/policy.ts'
import { getOrchestratorGraph, replaceOrchestratorGraph, resetOrchestratorStoresForTests } from '../../server/application/orchestration/task-graph.ts'
import { OrchestratorScheduler, ORCHESTRATOR_BUDGETS, ORCHESTRATOR_ROLE_PROFILE, requirementsFitAuthority, type OrchestratorChildPort } from '../../server/application/orchestration/scheduler.ts'

const root = resolve(import.meta.dirname, '../..')
const readProfile = (name: string) => readFileSync(resolve(root, '.agents/agents', `${name}.md`), 'utf8')
const scheduler = new OrchestratorScheduler()
const owner = { userId: 'user-042b', conversationId: 'conv-042b', parentSessionId: 'conv-042b' }
const fullAuthority: SubagentAuthority = {
  tools: ['file_read', 'file_search', 'text_search', 'git_status', 'git_diff', 'file_write', 'file_edit', 'apply_patch', 'http_fetch', 'change_request_merge'],
  effects: ['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation'],
  working_mode: 'workspace',
  model_policy: 'default',
  workspace_root: root
}
const readAuthority: SubagentAuthority = {
  tools: ['file_read', 'file_search', 'text_search', 'git_status', 'git_diff'],
  effects: ['workspace_read', 'git_read'],
  working_mode: 'read-only',
  model_policy: 'default',
  workspace_root: root
}
const node = (id: string, role: OrchestratorNode['role'] = 'researcher', depends_on: string[] = [], required_tools: string[] = ['file_read'], required_effects: SubagentEffect[] = ['workspace_read']): Omit<OrchestratorNode, 'status' | 'attempt' | 'evidence_refs' | 'blocked_by' | 'updated_at'> => ({
  id,
  role,
  objective: `Bounded scheduler task ${id}`,
  depends_on,
  budget_class: 'medium',
  required_tools,
  required_effects
})

class FakePort implements OrchestratorChildPort {
  readonly tasks = new Map<string, BackgroundTaskMetadata>()
  readonly starts: Array<{ taskId: string, nodeId: string, profile: string, isolation: string }> = []
  readonly cancelled: string[] = []

  capacity(parentSessionId: string) {
    const active = [...this.tasks.values()].filter(task => task.parent_session_id === parentSessionId && ['queued', 'starting', 'running', 'cancelling'].includes(task.state))
    return { global: Math.max(0, 4 - active.length), parent: Math.max(0, 2 - active.length) }
  }

  prepare(taskNode: OrchestratorNode, parentAuthority: SubagentAuthority) {
    const profileName = ORCHESTRATOR_ROLE_PROFILE[taskNode.role]
    if (taskNode.profile && taskNode.profile !== profileName) throw new Error('role/profile mismatch')
    const profile = loadAgentProfile(profileName, readProfile)
    const authority = intersectSubagentAuthority(parentAuthority, profile)
    if (!requirementsFitAuthority(taskNode, authority)) throw new Error('insufficient child authority')
    if (taskNode.role === 'writer' && authority.working_mode !== 'workspace') throw new Error('writer is read-only')
    if (taskNode.role !== 'writer' && authority.working_mode !== 'read-only') throw new Error('non-writer can mutate')
    return { profile: profileName, isolation: taskNode.role === 'writer' ? 'worktree' as const : 'shared_read' as const, budget: ORCHESTRATOR_BUDGETS[taskNode.budget_class] }
  }

  start(input: Parameters<OrchestratorChildPort['start']>[0]) {
    this.starts.push({ taskId: input.taskId, nodeId: input.node.id, profile: input.prepared.profile, isolation: input.prepared.isolation })
    const metadata: BackgroundTaskMetadata = {
      task_id: input.taskId,
      parent_session_id: owner.parentSessionId,
      user_id: owner.userId,
      agent_profile: input.prepared.profile,
      repository_identity: root,
      isolation: input.prepared.isolation,
      state: 'running',
      progress_summary: 'Running.',
      cleanup: input.prepared.isolation === 'worktree' ? 'preserved' : 'not_applicable'
    }
    this.tasks.set(input.taskId, metadata)
    return { task_id: input.taskId, state: 'queued' as const }
  }

  get(taskId: string) { return this.tasks.get(taskId) }
  cancel(taskId: string) {
    const task = this.tasks.get(taskId)
    if (!task || !['queued', 'starting', 'running', 'cancelling'].includes(task.state)) return false
    task.state = 'cancelled'
    this.cancelled.push(taskId)
    return true
  }

  settle(taskId: string, state: BackgroundTaskState) {
    const task = this.tasks.get(taskId)
    if (!task) throw new Error('missing fake task')
    task.state = state
    task.result = {
      status: state === 'completed' ? 'completed' : state === 'cancelled' ? 'cancelled' : state === 'blocked' ? 'blocked' : state === 'budget_exhausted' ? 'budget_exhausted' : 'failed',
      summary: `Result ${taskId}`,
      findings: [],
      evidence: [{ reference: `evidence/${taskId}`, detail: 'bounded' }],
      validation: [],
      remaining_risks: [],
      session_id: taskId,
      profile: task.agent_profile,
      usage: { turns: 1, tool_calls: 1, output_tokens: 1, context_tokens: 1, wall_time_ms: 1, depth: 0 },
      summary_ref: `result/${taskId}`
    }
  }
}

resetOrchestratorStoresForTests()

// Two independent read-only nodes run concurrently; third is queued deterministically by per-parent cap.
let graph = replaceOrchestratorGraph({ ...owner, nodes: [node('r1'), node('r2'), node('r3')], now: 1000 })
let port = new FakePort()
let dispatch = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: fullAuthority, port, now: 1001 })
assert.deepEqual(dispatch.started, ['r1', 'r2'])
assert.deepEqual(dispatch.queued, ['r3'])
assert.deepEqual(port.starts.map(item => item.profile), ['explore', 'explore'])
assert(port.starts.every(item => item.isolation === 'shared_read'))
assert.notEqual(port.starts[0]?.taskId, port.starts[1]?.taskId)

// Completion frees a slot; unrelated queued work proceeds without violating dependency ordering.
const r1Owner = dispatch.graph.nodes.find(item => item.id === 'r1')?.owner
assert(r1Owner)
port.settle(r1Owner, 'completed')
graph = scheduler.poll({ ...owner, generation: graph.generation, port, now: 1002 })
assert.equal(graph.nodes.find(item => item.id === 'r1')?.status, 'completed')
dispatch = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: fullAuthority, port, now: 1003 })
assert.deepEqual(dispatch.started, ['r3'])

// Dependency-constrained work waits while unrelated ready work proceeds.
resetOrchestratorStoresForTests()
port = new FakePort()
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('root'), node('dependent', 'researcher', ['root']), node('unrelated')], now: 2000 })
dispatch = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: fullAuthority, port, now: 2001 })
assert.deepEqual(dispatch.started, ['root', 'unrelated'])
assert.equal(dispatch.graph.nodes.find(item => item.id === 'dependent')?.status, 'pending')

// Writer routing is fixed to general-purpose + isolated worktree, with independent ownership IDs.
resetOrchestratorStoresForTests()
port = new FakePort()
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('w1', 'writer', [], ['file_write'], ['workspace_read', 'workspace_write']), node('w2', 'writer', [], ['file_edit'], ['workspace_read', 'workspace_write'])], now: 3000 })
dispatch = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: fullAuthority, port, now: 3001 })
assert.deepEqual(dispatch.started, ['w1', 'w2'])
assert(port.starts.every(item => item.profile === 'general-purpose' && item.isolation === 'worktree'))
assert.equal(new Set(port.starts.map(item => item.taskId)).size, 2)

// Role/profile mismatch and authority widening are denied before child execution.
resetOrchestratorStoresForTests()
port = new FakePort()
graph = replaceOrchestratorGraph({ ...owner, nodes: [{ ...node('mismatch', 'reviewer'), profile: 'general-purpose' }, node('writer-denied', 'writer', [], ['file_write'], ['workspace_write']), node('delivery-denied', 'writer', [], ['change_request_merge'], ['external_mutation'])], now: 4000 })
dispatch = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: readAuthority, port, now: 4001 })
assert.deepEqual(dispatch.started, [])
assert.deepEqual(dispatch.denied, ['mismatch', 'writer-denied', 'delivery-denied'])
assert.equal(port.starts.length, 0)
assert(dispatch.graph.nodes.every(item => item.status === 'blocked'))
assert.equal(dispatch.graph.status, 'blocked')
const deniedAgain = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: readAuthority, port, now: 4002 })
assert.deepEqual(deniedAgain.denied, [])
assert.deepEqual(deniedAgain.started, [])

// Budget exhaustion settles truthfully and blocks dependents without deadlocking unrelated nodes.
resetOrchestratorStoresForTests()
port = new FakePort()
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('budget'), node('blocked-after-budget', 'researcher', ['budget']), node('free')], now: 5000 })
dispatch = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: fullAuthority, port, now: 5001 })
const budgetOwner = dispatch.graph.nodes.find(item => item.id === 'budget')?.owner
assert(budgetOwner)
port.settle(budgetOwner, 'budget_exhausted')
graph = scheduler.poll({ ...owner, generation: graph.generation, port, now: 5002 })
assert.equal(graph.nodes.find(item => item.id === 'budget')?.status, 'failed')
assert.equal(graph.nodes.find(item => item.id === 'blocked-after-budget')?.status, 'blocked')
assert.equal(graph.nodes.find(item => item.id === 'free')?.status, 'running')

// Subtree cancellation aborts the running root and dependent subtree while unrelated work remains truthful.
resetOrchestratorStoresForTests()
port = new FakePort()
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('cancel-root'), node('cancel-child', 'researcher', ['cancel-root']), node('keep')], now: 6000 })
dispatch = scheduler.dispatchReady({ ...owner, generation: graph.generation, parentAuthority: fullAuthority, port, now: 6001 })
const keepOwner = dispatch.graph.nodes.find(item => item.id === 'keep')?.owner
const cancelOwner = dispatch.graph.nodes.find(item => item.id === 'cancel-root')?.owner
assert(keepOwner && cancelOwner)
graph = scheduler.cancelNode({ ...owner, generation: graph.generation, nodeId: 'cancel-root', subtree: true, port, now: 6002 })
assert.equal(graph.nodes.find(item => item.id === 'cancel-root')?.status, 'cancelled')
assert.equal(graph.nodes.find(item => item.id === 'cancel-child')?.status, 'cancelled')
assert.equal(graph.nodes.find(item => item.id === 'keep')?.status, 'running')
assert(port.cancelled.includes(cancelOwner))
assert(!port.cancelled.includes(keepOwner))

// Parent-run cancellation aborts every owned running child before graph state closes.
graph = scheduler.cancelRun({ ...owner, generation: graph.generation, port, now: 6003 })
assert.equal(graph.status, 'cancelled')
assert.equal(graph.nodes.find(item => item.id === 'keep')?.status, 'cancelled')
assert(port.cancelled.includes(keepOwner))

// Source boundary: orchestration must reuse the existing BackgroundTaskManager/runtime and never instantiate a second runtime.
const subagentSource = readFileSync(resolve(root, 'server/infrastructure/ai/subagent-tool.ts'), 'utf8')
assert.equal((subagentSource.match(/new SubagentRuntime\(/g) ?? []).length, 1)
assert.equal((subagentSource.match(/new BackgroundTaskManager\(runtime\)/g) ?? []).length, 1)
assert(subagentSource.includes('buildOrchestratorTools'))
assert(subagentSource.includes(`node.role === 'writer' ? 'worktree'`))
assert(subagentSource.includes(`input.abortSignal.addEventListener('abort', cancelActiveRun`))

const current = getOrchestratorGraph(owner.userId, owner.conversationId, 6004)
assert.equal(current?.status, 'cancelled')
console.log('042B orchestrator scheduler acceptance: PASS')
