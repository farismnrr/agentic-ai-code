import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { presentToolOutput, safeInputSummary, toolCategory } from '../../app/utils/tool-presentation.ts'
import { sanitizeAttributes } from '../../server/infrastructure/observability/sanitize.ts'

assert.equal(toolCategory('orchestrator_plan'), 'subagent')
assert.equal(toolCategory('orchestrator_dispatch'), 'subagent')
assert.equal(toolCategory('orchestrator_reconcile'), 'subagent')

const graph = presentToolOutput({
  generation: '11111111-1111-4111-8111-111111111111',
  status: 'active',
  nodes: [
    { id: 'read-a', status: 'completed' },
    { id: 'write-b', status: 'running' },
    { id: 'review-c', status: 'blocked' }
  ],
  ready: []
})
assert.equal(graph?.summary, 'Orchestration · 3 nodes · 1 running · 1 blocked · 1 completed · state active')
assert.equal(graph?.preview, undefined)

const reconciliation = presentToolOutput({ issues: [{ statement: 'P1 secret=client-secret /home/private/repo' }], writers: [{}], blockers: ['high_severity_finding'], delivery_ready: false })
assert.equal(reconciliation?.summary, 'Reconciliation · 1 issues · 1 writers · 1 blockers · delivery blocked')
assert.equal(reconciliation?.preview, undefined)

const safeInput = safeInputSummary({ generation: '11111111-1111-4111-8111-111111111111', node_id: 'review-c', scope: 'subtree', task_id: 'hidden-task-id', expected_head: 'a'.repeat(40), prompt: 'secret prompt' })
assert(safeInput.rows.some(row => row.label === 'node id' && row.value === 'review-c'))
assert(safeInput.rows.some(row => row.label === 'scope' && row.value === 'subtree'))
assert(!safeInput.rows.some(row => row.label.includes('task')))
assert(!safeInput.rows.some(row => row.label.includes('expected head')))
assert(!safeInput.rows.some(row => row.value.includes('secret prompt')))

const telemetry = sanitizeAttributes({
  'operation': 'chat.orchestrator.poll',
  'orchestration.run_id': '11111111-1111-4111-8111-111111111111',
  'orchestration.state': 'running',
  'orchestration.running.count': 2,
  'orchestration.blocker.count': 1,
  'orchestration.node_id': 'node-a',
  'prompt': 'Bearer secret-token',
  'private.path': '/home/private/repo'
})
assert.equal(telemetry['orchestration.running.count'], 2)
assert.equal(telemetry['orchestration.node_id'], 'node-a')
assert.equal(telemetry.prompt, undefined)
assert.equal(telemetry['private.path'], undefined)

const sourceRoot = resolve(import.meta.dirname, '../..')
const subagentSource = readFileSync(resolve(sourceRoot, 'server/infrastructure/ai/subagent-tool.ts'), 'utf8')
const schedulerSource = readFileSync(resolve(sourceRoot, 'server/application/orchestration/scheduler.ts'), 'utf8')
const reconcileSource = readFileSync(resolve(sourceRoot, 'server/application/orchestration/reconciliation.ts'), 'utf8')
assert.equal((subagentSource.match(/new SubagentRuntime\(/g) ?? []).length, 1)
assert.equal((subagentSource.match(/new BackgroundTaskManager\(runtime\)/g) ?? []).length, 1)
assert(!schedulerSource.includes('change_request_merge'))
assert(!reconcileSource.includes('change_request_merge'))
assert(!reconcileSource.includes('chain_of_thought'))
assert(!reconcileSource.includes('worktree_path'))
assert(subagentSource.includes('Actual delivery must already use Plan-040 Git/forge tools.'))

console.log('042D orchestration UX/observability closure acceptance: PASS')
