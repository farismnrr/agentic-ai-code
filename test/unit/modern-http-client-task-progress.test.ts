import { strict as assert } from 'node:assert'
import { ModernHttpMcpClient } from '../../server/infrastructure/mcp/modern-http-client.ts'

const task = (taskId: string, output = 'step 1\n') => ({
  resultType: 'task',
  taskId,
  status: 'working',
  createdAt: '2026-08-29T00:00:00Z',
  lastUpdatedAt: '2026-08-29T00:00:01Z',
  ttlMs: null,
  pollIntervalMs: 250,
  output: { stdout: output, stderr: '', omittedBytes: 0, exitCode: null }
})

function response(id: string, result: unknown) {
  return new Response(JSON.stringify({ jsonrpc: '2.0', id, result }), {
    status: 200,
    headers: { 'content-type': 'application/json' }
  })
}

function client(fetchImpl: typeof fetch, requestTimeoutMs = 1_000) {
  return new ModernHttpMcpClient(
    new URL('https://relay.example.test/mcp'),
    'test-token',
    fetchImpl,
    requestTimeoutMs,
    'first-party-relay'
  )
}

async function requestBody(init: RequestInit | undefined) {
  return JSON.parse(String(init?.body)) as { id: string, method: string }
}

{
  const methods: string[] = []
  const fetchImpl: typeof fetch = async (_input, init) => {
    const body = await requestBody(init)
    methods.push(body.method)
    if (body.method === 'server/discover') {
      return response(body.id, { supportedVersions: ['2026-07-28'], capabilities: {} })
    }
    if (body.method === 'tools/call') return response(body.id, task('async-task-1'))
    throw new Error(`unexpected method ${body.method}`)
  }

  const mcp = client(fetchImpl)
  await mcp.connect()
  const result = await mcp.callTool({
    name: 'terminal_exec',
    arguments: { execution_mode: 'async', idempotency_key: 'async-test-1' }
  })
  const progress = JSON.parse(result.content[0].text) as Record<string, unknown>

  assert.deepEqual(methods, ['server/discover', 'tools/call'])
  assert.equal(progress.resultType, 'task')
  assert.equal(progress.taskId, 'async-task-1')
  assert.match(String(progress.message), /terminal_job_get/)
  assert.match(JSON.stringify(progress.output), /step 1/)
}

{
  const methods: string[] = []
  const fetchImpl: typeof fetch = async (_input, init) => {
    const body = await requestBody(init)
    methods.push(body.method)
    if (body.method === 'server/discover') {
      return response(body.id, { supportedVersions: ['2026-07-28'], capabilities: {} })
    }
    if (body.method === 'tools/call') return response(body.id, task('cancel-safe-task'))
    if (body.method === 'tasks/get') return response(body.id, { ...task('cancel-safe-task', 'latest step\n'), status: 'working' })
    if (body.method === 'tasks/cancel') throw new Error('task cancellation must not be implicit')
    throw new Error(`unexpected method ${body.method}`)
  }

  const controller = new AbortController()
  const mcp = client(fetchImpl)
  await mcp.connect()
  const call = mcp.callTool({ name: 'terminal_exec', arguments: { execution_mode: 'auto' } }, controller.signal)
  await new Promise(resolve => setImmediate(resolve))
  controller.abort()
  const result = await call
  const progress = JSON.parse(result.content[0].text) as Record<string, unknown>

  assert.equal(progress.taskId, 'cancel-safe-task')
  assert.match(JSON.stringify(progress.output), /latest step/)
  assert.equal(methods.includes('tasks/cancel'), false)
}

{
  const methods: string[] = []
  const fetchImpl: typeof fetch = async (_input, init) => {
    const body = await requestBody(init)
    methods.push(body.method)
    if (body.method === 'server/discover') {
      return response(body.id, { supportedVersions: ['2026-07-28'], capabilities: {} })
    }
    if (body.method === 'tools/call') return response(body.id, task('poll-timeout-task', 'last known line\n'))
    if (body.method === 'tasks/get') {
      await new Promise<void>((_resolve, reject) => {
        init?.signal?.addEventListener('abort', () => reject(new Error('poll aborted')), { once: true })
      })
    }
    throw new Error(`unexpected method ${body.method}`)
  }

  const mcp = client(fetchImpl, 25)
  await mcp.connect()
  const result = await mcp.callTool({ name: 'terminal_exec', arguments: { execution_mode: 'auto' } })
  const progress = JSON.parse(result.content[0].text) as Record<string, unknown>

  assert.equal(progress.taskId, 'poll-timeout-task')
  assert.match(String(progress.message), /timed out/i)
  assert.match(JSON.stringify(progress.output), /last known line/)
  assert.deepEqual(methods, ['server/discover', 'tools/call', 'tasks/get'])
}

{
  const fetchImpl: typeof fetch = async (_input, init) => {
    const body = await requestBody(init)
    if (body.method === 'server/discover') {
      return response(body.id, { supportedVersions: ['2026-07-28'], capabilities: {} })
    }
    if (body.method === 'tools/call') return response(body.id, task('timed-out-task'))
    if (body.method === 'tasks/get') {
      return response(body.id, {
        resultType: 'complete',
        taskId: 'timed-out-task',
        status: 'completed',
        executionStatus: 'timed_out',
        output: { stdout: 'completed before timeout\n', stderr: '', omittedBytes: 0, exitCode: null },
        result: { content: [{ type: 'text', text: 'execution timed out' }], isError: true }
      })
    }
    throw new Error(`unexpected method ${body.method}`)
  }

  const mcp = client(fetchImpl)
  await mcp.connect()
  const result = await mcp.callTool({ name: 'terminal_exec', arguments: { execution_mode: 'auto' } })
  const progress = JSON.parse(result.content[0].text) as Record<string, unknown>

  assert.equal(progress.taskId, 'timed-out-task')
  assert.equal(progress.status, 'completed')
  assert.match(JSON.stringify(progress.output), /completed before timeout/)
  assert.equal(result.isError, true)
}

console.log('async task progress focused acceptance: PASS')
