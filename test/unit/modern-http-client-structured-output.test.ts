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
  $schema: 'https://json-schema.org/draft/2020-12/schema',
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
      content: [],
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
assert.deepEqual(result.content, [])

const invalidFetchImpl: typeof fetch = async (_input, init) => {
  const body = await requestBody(init)
  if (body.method === 'server/discover') {
    return response(body.id, { supportedVersions: ['2026-07-28'], capabilities: {} })
  }
  if (body.method === 'tools/list') {
    return response(body.id, {
      tools: [{
        name: 'file_read',
        inputSchema: { type: 'object', properties: {} },
        outputSchema
      }]
    })
  }
  if (body.method === 'tools/call') {
    return response(body.id, {
      content: [],
      structuredContent: { wrong: true },
      isError: false
    })
  }
  throw new Error(`unexpected method ${body.method}`)
}

const invalidClient = new ModernHttpMcpClient(
  new URL('https://relay.example.test/mcp'),
  'test-token',
  invalidFetchImpl,
  1_000,
  'first-party-relay'
)
await invalidClient.connect()
await invalidClient.listTools()
await assert.rejects(
  () => invalidClient.callTool({ name: 'file_read', arguments: {} }),
  /does not match its output schema/
)

const resourceFetchImpl: typeof fetch = async (_input, init) => {
  const body = await requestBody(init)
  if (body.method === 'server/discover') {
    return response(body.id, { supportedVersions: ['2026-07-28'], capabilities: { resources: {} } })
  }
  if (body.method === 'resources/read') {
    return response(body.id, {
      contents: [{
        uri: 'creative://asset/context/project_fixture/asset_fixture',
        blob: 'iVBORw0KGgo=',
        mimeType: 'image/png'
      }]
    })
  }
  throw new Error(`unexpected method ${body.method}`)
}

const resourceClient = new ModernHttpMcpClient(
  new URL('https://relay.example.test/mcp'),
  'test-token',
  resourceFetchImpl,
  1_000,
  'first-party-relay'
)
await resourceClient.connect()
const resource = await resourceClient.readResource('creative://asset/context/project_fixture/asset_fixture')
assert.deepEqual(resource.contents, [{
  uri: 'creative://asset/context/project_fixture/asset_fixture',
  text: undefined,
  blob: 'iVBORw0KGgo=',
  mimeType: 'image/png'
}])

console.log('modern MCP structured output acceptance: PASS')
