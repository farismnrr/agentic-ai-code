import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  advanceWriter,
  markDelivered,
  reconcileChildren,
  resetReconciliationForTests
} from '../../server/application/orchestration/reconciliation.ts'
import type { BackgroundTaskMetadata, SubagentResult } from '../../shared/types/subagents.ts'

const generation = '11111111-1111-4111-8111-111111111111'
const owner = { userId: 'user-1', conversationId: 'conv-1', generation }
const result = (findings: string[], refs: string[]): SubagentResult => ({
  status: 'completed',
  summary: 'bounded summary',
  findings,
  evidence: refs.map(reference => ({ reference, detail: 'bounded evidence' })),
  validation: [],
  remaining_risks: [],
  session_id: crypto.randomUUID(),
  profile: 'review',
  usage: { turns: 1, tool_calls: 1, output_tokens: 10, context_tokens: 10, wall_time_ms: 1, depth: 0 }
})
const task = (task_id: string, childResult: SubagentResult): BackgroundTaskMetadata => ({
  task_id,
  parent_session_id: owner.conversationId,
  user_id: owner.userId,
  agent_profile: childResult.profile,
  repository_identity: 'repo',
  isolation: 'shared_read',
  state: 'completed',
  progress_summary: childResult.summary,
  result: childResult,
  cleanup: 'not_applicable'
})

resetReconciliationForTests()

// Duplicate reviewer findings deduplicate while preserving provenance.
let ledger = reconcileChildren({ ...owner, children: [
  { task: task('11111111-1111-4111-8111-111111111112', result(['P2 duplicate finding'], ['code/ref-1'])) },
  { task: task('11111111-1111-4111-8111-111111111113', result(['P2 duplicate finding'], ['code/ref-1'])) }
], now: 1000 })
assert.equal(ledger.issues.length, 1)
assert.equal(ledger.issues[0]?.source_task_ids.length, 2)
assert.equal(ledger.issues[0]?.conflict, false)

// Different conclusions on the same evidence surface a conflict instead of majority-vote selection.
ledger = reconcileChildren({ ...owner, children: [
  { task: task('11111111-1111-4111-8111-111111111114', result(['P2 behavior is safe'], ['code/ref-shared'])) },
  { task: task('11111111-1111-4111-8111-111111111115', result(['P2 behavior is unsafe'], ['code/ref-shared'])) }
], now: 2000 })
assert(ledger.issues.every(issue => issue.conflict))
assert(ledger.blockers.includes('reviewer_disagreement'))
assert.equal(ledger.delivery_ready, false)

// P0/P1 findings block delivery even without writer work.
ledger = reconcileChildren({ ...owner, children: [
  { task: task('11111111-1111-4111-8111-111111111116', result(['P1 credential exposure'], ['security/ref'])) }
], now: 3000 })
assert(ledger.blockers.includes('high_severity_finding'))
assert.throws(() => markDelivered(owner), /not delivery ready/)

// Writer lifecycle is explicit and stale/dirty identity fails closed.
const writerTask = task('11111111-1111-4111-8111-111111111117', result([], ['git/status']))
writerTask.agent_profile = 'general-purpose'
writerTask.isolation = 'worktree'
const head = 'a'.repeat(40)
ledger = reconcileChildren({ ...owner, children: [{ task: writerTask, writer: { branch: 'ai-code/background/task', base_commit: 'b'.repeat(40), head_commit: head, dirty: false } }], now: 4000 })
assert(ledger.blockers.includes('unintegrated_writer'))
assert.equal(ledger.writers[0]?.state, 'produced')
assert.throws(() => advanceWriter({ ...owner, taskId: writerTask.task_id, expectedHead: 'c'.repeat(40), action: 'review' }), /stale or dirty/)
assert.throws(() => advanceWriter({ ...owner, taskId: writerTask.task_id, expectedHead: head, action: 'review', currentWriter: { branch: 'ai-code/background/task', base_commit: 'b'.repeat(40), head_commit: 'c'.repeat(40), dirty: false } }), /stale or dirty/)
ledger = advanceWriter({ ...owner, taskId: writerTask.task_id, expectedHead: head, action: 'review', now: 4001 })
assert.equal(ledger.writers[0]?.state, 'reviewed')
ledger = advanceWriter({ ...owner, taskId: writerTask.task_id, expectedHead: head, action: 'accept', now: 4002 })
assert.equal(ledger.writers[0]?.state, 'accepted')
ledger = advanceWriter({ ...owner, taskId: writerTask.task_id, expectedHead: head, action: 'integrate', now: 4003 })
assert.equal(ledger.delivery_ready, true)
ledger = markDelivered({ ...owner, now: 4004 })
assert.equal(ledger.writers[0]?.state, 'delivered')

const dirtyTask = task('11111111-1111-4111-8111-111111111118', result([], ['git/status']))
dirtyTask.agent_profile = 'general-purpose'
dirtyTask.isolation = 'worktree'
ledger = reconcileChildren({ ...owner, children: [{ task: dirtyTask, writer: { branch: 'ai-code/background/dirty', base_commit: 'b'.repeat(40), head_commit: head, dirty: true } }], now: 5000 })
assert(ledger.blockers.includes('dirty_writer'))
assert.throws(() => advanceWriter({ ...owner, taskId: dirtyTask.task_id, expectedHead: head, action: 'review' }), /stale or dirty/)

// Presentation/state never includes private worktree paths or hidden reasoning fields.
const encoded = JSON.stringify(ledger)
assert(!encoded.includes('worktree_path'))
assert(!encoded.includes('chain_of_thought'))
assert(!encoded.includes('reasoning'))

const source = readFileSync(resolve(import.meta.dirname, '../../server/infrastructure/ai/subagent-tool.ts'), 'utf8')
assert(source.includes('orchestrator_reconcile'))
assert(source.includes('orchestrator_writer_transition'))
assert(source.includes('orchestrator_mark_delivered'))
assert(source.includes('Actual delivery must already use Plan-040 Git/forge tools.'))

console.log('042C reconciliation acceptance: PASS')
