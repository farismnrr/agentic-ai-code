import { strict as assert } from 'node:assert'
import { ModernHttpMcpClient } from '../../server/infrastructure/mcp/modern-http-client.ts'

function response(id: string, result: unknown) {
  return new Response(JSON.stringify({ jsonrpc: '2.0', id, result }), {
    status: 200,
    headers: { 'content-type': 'application/json' }
  })
}

async function requestBody(init: RequestInit | undefined) {
  return JSON.parse(String(init?.body)) as { id: string, method: string }
}

const outputSchema = {
  type: 'object',
  properties: {
    path: { type: 'string' }
  },
  required: ['path'],
  additionalProperties: false
}

const structuredContent = { path: 'README.md' }

const fetchImpl: typeof fetch = async (_input, init) => {
  const body = await requestBody(init)
  if (body.method === 'server/discover') {
    return response(body.id, { supportedVersions: ['2026-07-28'], capabilities: {} })
  }
  if (body.method === 'tools/list') {
    return response(body.id, {
      tools: [{
        name: 'file_read',
        description: 'fixture',
        inputSchema: { type: 'object', properties: {} },
        outputSchema
      }]
    })
  }
  if (body.method === 'tools/call') {
    return response(body.id, {
      content: [{ type: 'text', text: JSON.stringify(structuredContent) }],
      structuredContent,
      isError: false
    })
  }
  throw new Error(`unexpected method ${body.method}`)
}

const client = new ModernHttpMcpClient(
  new URL('https://relay.example.test/mcp'),
  'test-token',
  fetchImpl,
  1_000,
  'first-party-relay'
)

await client.connect()
const tools = await client.listTools()
assert.deepEqual(tools.tools[0]?.outputSchema, outputSchema)

const result = await client.callTool({ name: 'file_read', arguments: {} })
assert.deepEqual(result.structuredContent, structuredContent)
assert.deepEqual(JSON.parse(String((result.content[0] as { text: string }).text)), structuredContent)

console.log('modern MCP structured output acceptance: PASS')
