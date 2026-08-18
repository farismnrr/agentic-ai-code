import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { categoryLabel, presentToolOutput, safeInputSummary, toolCategory } from '../app/utils/tool-presentation.ts'
import { sanitizeAttributes } from '../server/infrastructure/observability/sanitize.ts'
import { parsePresentationSafeSubagentResult, presentationSafeBackgroundTask } from '../server/infrastructure/ai/subagent-result.ts'

const secretInput = {
  path: '/home/alice/project/src/main.ts',
  command: '/usr/bin/bash',
  args: ['-lc', 'echo token=super-secret'],
  url: 'https://user:pass@example.test/private?q=secret',
  authorization: 'Bearer top-secret',
  patch: 'SECRET PATCH CONTENT',
  max_results: 12,
  flags: ['a', 'b'],
  note: 'top-secret-with-innocent-key',
  query: 'token=search-secret',
  pattern: 'Bearer search-secret',
  glob: 'source-secret/**'
}
const summary = safeInputSummary(secretInput)
assert(summary.rows.some(row => row.label === 'path' && row.value === '…/src/main.ts'))
assert(summary.rows.some(row => row.label === 'command' && row.value === 'bash'))
assert(summary.rows.some(row => row.label === 'url' && row.value === 'example.test'))
assert(summary.rows.some(row => row.label === 'max results' && row.value === '12'))
assert(summary.hiddenFields >= 3)
const serializedSummary = JSON.stringify(summary)
for (const forbidden of ['super-secret', 'top-secret', 'top-secret-with-innocent-key', 'search-secret', 'source-secret', 'SECRET PATCH CONTENT', 'user:pass']) assert(!serializedSummary.includes(forbidden), `summary leaked ${forbidden}`)

const windowsPathSummary = JSON.stringify(safeInputSummary({ path: 'C:\\Users\\alice\\private\\file.ts' }))
assert(!windowsPathSummary.includes('C:'))
assert(!windowsPathSummary.includes('Users'))
assert(windowsPathSummary.includes('…/private/file.ts'))
const uncPathSummary = JSON.stringify(safeInputSummary({ cwd: '\\\\server\\share\\private\\repo' }))
assert(!uncPathSummary.includes('server'))
assert(!uncPathSummary.includes('share'))
assert(uncPathSummary.includes('…/private/repo'))

const child = parsePresentationSafeSubagentResult(JSON.stringify({
  status: 'completed',
  summary: 'done token=child-secret /etc/passwd path=/home/alice/private/embedded.txt C:\\Users\\alice\\private.txt \\\\server\\share\\secret.txt /home/alice/private/project/file.ts',
  findings: ['Bearer child-bearer', 'safe finding'],
  evidence: [{ reference: '/home/alice/private/project/file.ts', detail: 'password=child-password' }],
  validation: ['safe validation'],
  remaining_risks: ['cookie=child-cookie']
}))
assert(child)
const serializedChild = JSON.stringify(child)
for (const forbidden of ['child-secret', 'child-bearer', 'child-password', 'child-cookie', '/etc/passwd', '/home/alice/private/embedded.txt', 'C:\\Users\\alice\\private.txt', '\\\\server\\share\\secret.txt', '/home/alice/private/project/file.ts']) assert(!serializedChild.includes(forbidden), `subagent result leaked ${forbidden}`)
assert.equal(parsePresentationSafeSubagentResult('not-json'), undefined)
assert.equal(parsePresentationSafeSubagentResult(JSON.stringify({ findings: ['missing summary'] })), undefined)
assert.equal(parsePresentationSafeSubagentResult(JSON.stringify({ status: 'bogus', summary: 'bounded' }))?.status, 'invalid')

const safeBackground = presentationSafeBackgroundTask({
  task_id: 'task', parent_session_id: 'parent', user_id: 'user', agent_profile: 'general-purpose', repository_identity: '/home/alice/private/repo', isolation: 'worktree', state: 'completed', progress_summary: 'done /etc/shadow', cleanup: 'preserved', worktree_path: '/home/alice/private/worktree',
  result: { status: 'completed', summary: 'ok', findings: [], evidence: [{ reference: 'git/status', detail: ' M /home/alice/private/repo/secret.ts token=background-secret' }], validation: [], remaining_risks: [], session_id: 'child', profile: 'general-purpose', usage: { turns: 1, tool_calls: 1, output_tokens: 1, context_tokens: 1, wall_time_ms: 1, depth: 0 } }
})
const serializedBackground = JSON.stringify(safeBackground)
for (const forbidden of ['/home/alice/private/repo', '/home/alice/private/worktree', '/etc/shadow', 'background-secret']) assert(!serializedBackground.includes(forbidden), `background result leaked ${forbidden}`)

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

const sensitiveOutput = presentToolOutput({
  content: 'token=client-secret Bearer abc.def /etc/passwd /home/alice/private/repo/src/file.ts C:\\Users\\alice\\private\\file.ts \\\\server\\share\\secret\\file.ts'
})
assert(sensitiveOutput)
for (const forbidden of ['client-secret', 'abc.def', '/etc/passwd', '/home/alice/private/repo', 'C:\\Users\\alice\\private\\file.ts', '\\\\server\\share\\secret\\file.ts']) assert(!JSON.stringify(sensitiveOutput).includes(forbidden), `tool output leaked ${forbidden}`)

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
assert(!toolSource.includes('{{ errorText }}'))
assert(toolSource.includes('Tool execution failed.'))
const subagentSource = readFileSync(resolve(import.meta.dirname, '../app/components/chat/ChatSubagentCall.vue'), 'utf8')
assert(!subagentSource.includes('{{ input.task }}'))
const partsSource = readFileSync(resolve(import.meta.dirname, '../app/components/chat/ChatMessageParts.vue'), 'utf8')
assert(partsSource.includes('getToolName(part).startsWith(\'agent_task_\')'))
const backgroundSource = readFileSync(resolve(import.meta.dirname, '../server/application/subagents/background.ts'), 'utf8')
assert(!backgroundSource.includes('error.message.slice'))
assert(backgroundSource.includes('Background task failed.'))

console.log('039J agent UX/observability acceptance: PASS')
