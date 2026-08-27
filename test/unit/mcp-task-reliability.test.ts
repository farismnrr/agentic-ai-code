import { strict as assert } from 'node:assert'
import { classifyRawCause } from '../../server/core/errors/classify.ts'
import {
  asMcpTaskEnvelope,
  fetchWithMcpDeadline,
  mcpRoutingName,
  McpRoundTripTimeoutError,
  resolveMcpRequestTimeoutMs,
  taskPollDelayMs
} from '../../server/infrastructure/mcp/task-reliability.ts'

assert.equal(resolveMcpRequestTimeoutMs(undefined), 45_000)
assert.equal(resolveMcpRequestTimeoutMs('25000'), 25_000)
assert.throws(() => resolveMcpRequestTimeoutMs(999), /configuration is invalid/)
assert.throws(() => resolveMcpRequestTimeoutMs(120_001), /configuration is invalid/)
assert.throws(() => resolveMcpRequestTimeoutMs('not-a-number'), /configuration is invalid/)

assert.deepEqual(asMcpTaskEnvelope({ resultType: 'task', taskId: 'job-1' }), { resultType: 'task', taskId: 'job-1' })
assert.equal(asMcpTaskEnvelope({ resultType: 'complete', taskId: 'job-1' }), undefined)
assert.equal(asMcpTaskEnvelope({ resultType: 'task', taskId: '' }), undefined)
assert.equal(asMcpTaskEnvelope({ resultType: 'task', taskId: 'x'.repeat(257) }), undefined)

assert.equal(mcpRoutingName('tools/call', { name: 'http_fetch' }), 'http_fetch')
assert.equal(mcpRoutingName('tasks/get', { taskId: 'job-1' }), 'job-1')
assert.equal(mcpRoutingName('tasks/cancel', { taskId: 'job-1' }), 'job-1')
assert.equal(mcpRoutingName('resources/read', { uri: 'workspace://manifest' }), undefined)

assert.equal(taskPollDelayMs({ pollIntervalMs: 1000 }), 1000)
assert.equal(taskPollDelayMs({ pollIntervalMs: 10 }), 250)
assert.equal(taskPollDelayMs({ pollIntervalMs: 50_000 }), 5000)
assert.equal(taskPollDelayMs({}, 500), 750)
assert.equal(taskPollDelayMs({}, 4000), 5000)

const timeoutFetch: typeof fetch = async (_input, init) => {
  await new Promise<void>((_resolve, reject) => {
    init?.signal?.addEventListener('abort', () => reject(init.signal?.reason ?? new Error('aborted')), { once: true })
  })
  throw new Error('unreachable')
}

const timeoutError = new McpRoundTripTimeoutError()
assert.equal(classifyRawCause(timeoutError), 'timeout')
await assert.rejects(
  fetchWithMcpDeadline(timeoutFetch, new URL('https://example.test/mcp'), { method: 'POST' }, 25),
  error => error instanceof McpRoundTripTimeoutError && error.message === 'Remote MCP request timed out'
)

const upstream = new AbortController()
const upstreamFetch: typeof fetch = async (_input, init) => {
  await new Promise<void>((_resolve, reject) => {
    init?.signal?.addEventListener('abort', () => reject(new Error('caller-aborted')), { once: true })
  })
  throw new Error('unreachable')
}
const upstreamPromise = fetchWithMcpDeadline(upstreamFetch, new URL('https://example.test/mcp'), { signal: upstream.signal }, 1000)
upstream.abort()
await assert.rejects(upstreamPromise, /caller-aborted/)

console.log('040A MCP task reliability focused acceptance: PASS')
