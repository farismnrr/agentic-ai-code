import { strict as assert } from 'node:assert'
import {
  completionTransitionWasNewlyReached,
  formatTaskCompletionMessage,
  sanitizeTaskCompletion
} from '../../server/application/task-notifications.ts'
import { ModernHttpMcpClient } from '../../server/infrastructure/mcp/modern-http-client.ts'
import { approvalForCapability, capabilityFactsForToolCall } from '../../shared/utils/capability-policy.ts'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const valid = {
  source: 'external-mcp' as const,
  taskId: 'og_123',
  title: 'Ship Telegram completion notice',
  summary: 'Implemented the feature and ran the focused tests.',
  completedAt: '2026-08-28T16:00:00.000Z',
  resultUrl: 'https://ai-code.example/tasks/og_123'
}

const payload = sanitizeTaskCompletion(valid)
assert.equal(payload.contractVersion, '1')
assert.equal(payload.taskId, valid.taskId)
assert.equal(payload.source, valid.source)
assert.equal(formatTaskCompletionMessage(payload), '✅ Ship Telegram completion notice\nImplemented the feature and ran the focused tests.\nResult: https://ai-code.example/tasks/og_123')

const unsafe = sanitizeTaskCompletion({
  ...valid,
  title: '\u001b[31mDone\u001b[0m\n',
  summary: 'Authorization: Bearer super-secret password=hidden\nNEXT\tline'
})
assert.equal(unsafe.title, 'Done')
assert.match(unsafe.summary, /Authorization: Bearer \[REDACTED\]/)
assert.match(unsafe.summary, /password=\[REDACTED\]/)
assert.doesNotMatch(unsafe.summary, /super-secret|hidden/)
assert.equal(unsafe.summary.includes('\u001b'), false)
assert.doesNotMatch(unsafe.summary, /\nNEXT\t/)

assert.throws(() => sanitizeTaskCompletion({ ...valid, taskId: '' }), /taskId/)
assert.throws(() => sanitizeTaskCompletion({ ...valid, title: 'x'.repeat(161) }), /title/)
assert.throws(() => sanitizeTaskCompletion({ ...valid, summary: 'x'.repeat(2001) }), /summary/)
assert.throws(() => sanitizeTaskCompletion({ ...valid, resultUrl: 'http://ai-code.example/task' }), /resultUrl/)
assert.throws(() => sanitizeTaskCompletion({ ...valid, resultUrl: 'https://ai-code.example/task?token=secret' }), /resultUrl/)
assert.equal(completionTransitionWasNewlyReached('active', 'completed'), true)
assert.equal(completionTransitionWasNewlyReached('completed', 'completed'), false)
assert.equal(completionTransitionWasNewlyReached('blocked', 'completed'), false)
assert.equal(completionTransitionWasNewlyReached('active', 'failed'), false)

const serialized = JSON.stringify(payload)
assert.doesNotMatch(serialized, /token|chatId|chat_id|bot/i)
assert(Buffer.byteLength(formatTaskCompletionMessage(payload), 'utf8') <= 4096)

const requests: Array<{ method: string, params: Record<string, unknown> }> = []
const fetchImpl: typeof fetch = async (_input, init) => {
  const request = JSON.parse(String(init?.body)) as { method: string, params: Record<string, unknown> }
  requests.push(request)
  const result = request.method === 'server/discover'
    ? { supportedVersions: ['2026-07-28'], capabilities: { extensions: { 'io.masihawam/task-completion-notifications': { version: '1', method: 'server/task_completed' } } } }
    : { status: 'queued' }
  return new Response(JSON.stringify({ jsonrpc: '2.0', id: (request as unknown as { id: string }).id, result }), { status: 200, headers: { 'content-type': 'application/json' } })
}
const client = new ModernHttpMcpClient(new URL('https://relay.example/mcp'), 'opaque-test-token', fetchImpl, 1000, 'first-party-relay')
await client.connect()
assert.equal(client.supportsTaskCompletion(), true)
assert.deepEqual(await client.taskCompleted(payload), { status: 'queued' })
assert.equal(requests[1]?.method, 'server/task_completed')
assert.equal(requests[1]?.params.taskId, payload.taskId)
assert.equal((requests[1]?.params as { _meta?: Record<string, unknown> })._meta?.['io.modelcontextprotocol/clientCapabilities'] !== undefined, true)
await client.close()

const facts = capabilityFactsForToolCall({
  toolId: 'relay.task_completed',
  toolName: 'task_completed',
  input: { taskId: 'og_123', title: 'done', summary: 'done' },
  annotations: { destructiveHint: false, openWorldHint: true },
  trustedProvenance: 'first-party-relay'
})
assert.deepEqual(facts.effects, ['network_write', 'external_mutation'])
assert.equal(approvalForCapability(facts, undefined, 'plan').outcome, 'denied')
assert.equal(approvalForCapability(facts, undefined, 'bypass').outcome, 'approved')

const relayCatalog = readFileSync(resolve(import.meta.dirname, '../../packages/rust-tools/interfaces/src/mcp/catalog.rs'), 'utf8')
const taskCompletionCatalog = readFileSync(resolve(import.meta.dirname, '../../packages/rust-tools/interfaces/src/mcp/catalog/task_completion.rs'), 'utf8')
assert.match(taskCompletionCatalog, /name: "task_completed"/)
assert.doesNotMatch(`${relayCatalog}\n${taskCompletionCatalog}`, /name: "telegram_send"/)
assert.doesNotMatch(taskCompletionCatalog, /botToken|chatId|chat_id/)

console.log('task completion notification contract: PASS')
