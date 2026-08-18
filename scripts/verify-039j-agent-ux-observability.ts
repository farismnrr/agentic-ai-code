import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { categoryLabel, presentToolOutput, safeInputSummary, toolCategory } from '../app/utils/tool-presentation.ts'
import { sanitizeAttributes } from '../server/infrastructure/observability/sanitize.ts'

const secretInput = {
  path: '/home/alice/project/src/main.ts',
  command: '/usr/bin/bash',
  args: ['-lc', 'echo token=super-secret'],
  url: 'https://user:pass@example.test/private?q=secret',
  authorization: 'Bearer top-secret',
  patch: 'SECRET PATCH CONTENT',
  max_results: 12,
  flags: ['a', 'b']
}
const summary = safeInputSummary(secretInput)
assert(summary.rows.some(row => row.label === 'path' && row.value === 'project/src/main.ts'))
assert(summary.rows.some(row => row.label === 'command' && row.value === 'bash'))
assert(summary.rows.some(row => row.label === 'url' && row.value === 'example.test'))
assert(summary.rows.some(row => row.label === 'max results' && row.value === '12'))
assert(summary.hiddenFields >= 3)
const serializedSummary = JSON.stringify(summary)
for (const forbidden of ['super-secret', 'top-secret', 'SECRET PATCH CONTENT', 'user:pass']) assert(!serializedSummary.includes(forbidden), `summary leaked ${forbidden}`)

assert.equal(toolCategory('file_read'), 'read')
assert.equal(toolCategory('git_diff'), 'git')
assert.equal(toolCategory('file_edit'), 'mutation')
assert.equal(toolCategory('terminal_exec'), 'execution')
assert.equal(toolCategory('http_fetch'), 'network')
assert.equal(toolCategory('delegate_task'), 'subagent')
assert.equal(toolCategory('agent_task_start'), 'subagent')
assert.equal(toolCategory('code_diagnostics'), 'diagnostics')
assert.equal(categoryLabel('mutation'), 'File change')

const largeDiff = 'x'.repeat(7000)
const output = presentToolOutput({ diff: largeDiff, truncated: true, continuation: 'opaque-token' })
assert(output)
assert.equal(output.preview?.length, 6000)
assert.equal(output.truncated, true)
assert.equal(output.continuation, true)
assert(!JSON.stringify(output).includes('opaque-token'))

const safeTelemetry = sanitizeAttributes({
  'operation': 'chat.tool.action', 'outcome': 'ok', 'tool.name': 'file_read', 'tool.id': 'relay.file_read',
  'tool.effects': 'workspace_read', 'policy.outcome': 'approved', 'policy.source': 'runtime-policy',
  'result.classification': 'bounded', 'result.truncated': false, 'duration_ms': 12,
  'unauthorized_payload': 'must-drop', 'error.message': 'Bearer abc.def token=hello /home/alice/private/file'
})
assert.equal(safeTelemetry['tool.id'], 'relay.file_read')
assert.equal(safeTelemetry['policy.outcome'], 'approved')
assert(!('unauthorized_payload' in safeTelemetry))
assert(!String(safeTelemetry['error.message']).includes('abc.def'))
assert(!String(safeTelemetry['error.message']).includes('token=hello'))
assert(!String(safeTelemetry['error.message']).includes('/home/alice/private/file'))

const approvalSource = readFileSync(resolve(import.meta.dirname, '../app/components/chat/ChatToolApproval.vue'), 'utf8')
assert(!approvalSource.includes('JSON.stringify(props.hookApproval.input'))
assert(approvalSource.includes('safeInputSummary'))
const toolSource = readFileSync(resolve(import.meta.dirname, '../app/components/chat/ChatToolCall.vue'), 'utf8')
assert(!toolSource.includes('JSON.stringify(value'))
assert(toolSource.includes('presentToolOutput'))
assert(toolSource.includes('resolveMcpToolFromModelName'))
assert(toolSource.includes('value.length > 1000'))
const partsSource = readFileSync(resolve(import.meta.dirname, '../app/components/chat/ChatMessageParts.vue'), 'utf8')
assert(partsSource.includes('getToolName(part).startsWith(\'agent_task_\')'))

console.log('039J agent UX/observability acceptance: PASS')
