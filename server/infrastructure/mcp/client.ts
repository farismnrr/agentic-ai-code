import { context, propagation } from '@opentelemetry/api'
import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js'
import { SSEClientTransport } from '@modelcontextprotocol/sdk/client/sse.js'
import { assertSafeUrl, createSsrfSafeFetch } from '../security/ssrf-guard'
import type { InferSelectModel } from 'drizzle-orm'
import type { mcpServers } from '../../database/schema'

type McpServerConfig = InferSelectModel<typeof mcpServers>

type RemoteMcpRuntimeConfig = {
  url?: string
  accessToken?: string
}

const MODERN_MCP_VERSION = '2026-07-28'
const MCP_CLIENT_INFO = { name: 'ai-code', version: '1.0.0' } as const

export type McpClientTool = {
  name: string
  description?: string
  inputSchema: Record<string, unknown>
}

export type McpClientCallResult = {
  content: unknown[]
  isError?: boolean
  [key: string]: unknown
}

export interface McpClientLike {
  listTools(): Promise<{ tools: McpClientTool[], [key: string]: unknown }>
  callTool(params: { name: string, arguments?: Record<string, unknown> }): Promise<McpClientCallResult>
  close(): Promise<void>
}

function getRemoteMcpRuntimeConfig(): RemoteMcpRuntimeConfig {
  const config = useRuntimeConfig()
  return config.remoteMcp as RemoteMcpRuntimeConfig
}

/**
 * Returns the private first-party configuration only when the stored MCP URL
 * is the exact resource configured by the operator. This avoids creating a
 * generic "send this secret to any user-provided URL" capability.
 */
function resolveFirstPartyRemote(url: URL, serverName: string) {
  const configured = getRemoteMcpRuntimeConfig()
  const configuredUrlRaw = configured.url?.trim()
  if (!configuredUrlRaw) return undefined

  let configuredUrl: URL
  try {
    configuredUrl = new URL(configuredUrlRaw)
  } catch {
    throw new Error('Remote MCP runtime configuration has an invalid URL')
  }

  if (configuredUrl.protocol !== 'https:') {
    throw new Error('Remote MCP runtime configuration must use HTTPS')
  }
  if (configuredUrl.username || configuredUrl.password || configuredUrl.search || configuredUrl.hash) {
    throw new Error('Remote MCP runtime configuration must not contain credentials, query, or fragment')
  }

  // Exact resource identity is intentional: an access token configured for
  // https://host/mcp must never be attached to https://host/other-path.
  if (configuredUrl.href !== url.href) return undefined

  const accessToken = configured.accessToken?.trim()
  if (!accessToken) {
    throw new Error(`Server "${serverName}" matches the configured remote MCP resource but no access token is configured`)
  }

  return { accessToken }
}

/**
 * Add W3C trace context only for the first-party remote relay. Sending our
 * internal trace identity to arbitrary third-party MCP servers would broaden
 * the telemetry trust boundary unnecessarily.
 */
function withFirstPartyTrace(fetchImpl: typeof fetch): typeof fetch {
  return async (input, init) => {
    const traceHeaders: Record<string, string> = {}
    propagation.inject(context.active(), traceHeaders)

    // Fetch semantics let `init.headers` override headers carried by a Request.
    // Preserve that precedence before injecting trace context.
    const headers = new Headers(input instanceof Request ? input.headers : undefined)
    for (const [name, value] of new Headers(init?.headers).entries()) {
      headers.set(name, value)
    }
    for (const [name, value] of Object.entries(traceHeaders)) {
      headers.set(name, value)
    }
    return fetchImpl(input, { ...init, headers })
  }
}

function isJsonRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function encodeMcpHeaderValue(value: string) {
  // The public relay's current tool names are ASCII, but the 2026 transport
  // defines a Base64 sentinel for values that cannot be represented directly
  // as an HTTP header. Keep the adapter correct if a future tool name changes.
  return /^[\x20-\x7e]*$/.test(value)
    ? value
    : `=?base64?${Buffer.from(value, 'utf8').toString('base64')}?=`
}

/**
 * Small, first-party-only MCP 2026 client for the Rust relay.
 *
 * The repository still carries the monolithic MCP SDK v1 for unrelated
 * third-party integrations and the legacy inbound Nuxt MCP endpoint. Rather
 * than weaken the Rust relay or make its modern path depend on a legacy SDK
 * lifecycle, this adapter speaks only the three RPCs ai-code needs against its
 * own relay: `server/discover`, `tools/list`, and `tools/call`.
 *
 * It is intentionally not a generic replacement for the MCP SDK. When the
 * repository can migrate the outbound client to `@modelcontextprotocol/client`
 * v2 with an atomic lockfile update + local verification, this class should be
 * deleted in favor of the official modern client with version negotiation.
 */
class FirstPartyRelayMcpClient implements McpClientLike {
  private requestSequence = 0

  constructor(
    private readonly url: URL,
    private readonly accessToken: string,
    private readonly fetchImpl: typeof fetch
  ) {}

  async connect() {
    const result = await this.request('server/discover', {})
    if (!isJsonRecord(result)
      || !Array.isArray(result.supportedVersions)
      || !result.supportedVersions.includes(MODERN_MCP_VERSION)) {
      throw new Error('Remote MCP server does not advertise the required protocol version')
    }
  }

  async listTools() {
    const result = await this.request('tools/list', {})
    if (!isJsonRecord(result) || !Array.isArray(result.tools)) {
      throw new Error('Remote MCP server returned an invalid tools/list result')
    }

    const tools = result.tools.map((tool) => {
      if (!isJsonRecord(tool)
        || typeof tool.name !== 'string'
        || !isJsonRecord(tool.inputSchema)) {
        throw new Error('Remote MCP server returned an invalid tool definition')
      }
      return {
        ...tool,
        name: tool.name,
        description: typeof tool.description === 'string' ? tool.description : undefined,
        inputSchema: tool.inputSchema
      } satisfies McpClientTool
    })

    return { ...result, tools }
  }

  async callTool(params: { name: string, arguments?: Record<string, unknown> }) {
    const result = await this.request('tools/call', {
      name: params.name,
      arguments: params.arguments ?? {}
    })
    if (!isJsonRecord(result) || !Array.isArray(result.content)) {
      throw new Error('Remote MCP server returned an invalid tools/call result')
    }
    return {
      ...result,
      content: result.content,
      ...(typeof result.isError === 'boolean' && { isError: result.isError })
    }
  }

  close() {
    // MCP 2026-07-28 is stateless for this relay. There is no protocol session
    // or background SSE channel to terminate.
    return Promise.resolve()
  }

  private async request(method: 'server/discover' | 'tools/list' | 'tools/call', params: Record<string, unknown>) {
    const id = `ai-code-${++this.requestSequence}`
    const requestParams = {
      ...params,
      _meta: {
        'io.modelcontextprotocol/protocolVersion': MODERN_MCP_VERSION,
        'io.modelcontextprotocol/clientCapabilities': {},
        'io.modelcontextprotocol/clientInfo': MCP_CLIENT_INFO
      }
    }
    const headers = new Headers({
      Accept: 'application/json, text/event-stream',
      Authorization: `Bearer ${this.accessToken}`,
      'Content-Type': 'application/json',
      'MCP-Protocol-Version': MODERN_MCP_VERSION,
      'Mcp-Method': method
    })
    if (method === 'tools/call' && typeof params.name === 'string') {
      headers.set('Mcp-Name', encodeMcpHeaderValue(params.name))
    }

    const response = await this.fetchImpl(this.url, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        jsonrpc: '2.0',
        id,
        method,
        params: requestParams
      })
    })

    if (response.status === 401 || response.status === 403) {
      throw new Error('Remote MCP authorization failed')
    }
    if (!response.ok) {
      throw new Error(`Remote MCP request failed with HTTP ${response.status}`)
    }

    let payload: unknown
    try {
      payload = await response.json()
    } catch {
      throw new Error('Remote MCP server returned invalid JSON')
    }
    if (!isJsonRecord(payload)
      || payload.jsonrpc !== '2.0'
      || payload.id !== id) {
      throw new Error('Remote MCP server returned an invalid JSON-RPC response')
    }
    if ('error' in payload) {
      // Do not forward arbitrary remote error text into application logs/UI.
      throw new Error('Remote MCP request returned a protocol error')
    }
    if (!('result' in payload)) {
      throw new Error('Remote MCP response is missing a result')
    }
    return payload.result
  }
}

/**
 * Connects to a stored MCP server, per request — no pooling, no reconnect
 * logic (see plan 012's "Scope boundary" decision). Callers must `close()`
 * when done.
 *
 * `stdio` transport is deliberately unsupported here: it would spawn
 * `mcpServers.command` — a value any authenticated user can set — as a
 * server-side child process. Rows with `transport: 'stdio'` fail closed.
 *
 * Generic HTTP/SSE rows still use the SDK v1 client for existing third-party
 * integrations. Plan 036's exact first-party remote resource instead uses the
 * strict MCP 2026 adapter above, with an externally-issued private Bearer token
 * and the same redirect/DNS SSRF guard used by provider traffic.
 */
export async function createMcpClient(serverConfig: McpServerConfig): Promise<McpClientLike> {
  if (serverConfig.transport === 'stdio') {
    throw new Error(`Server "${serverConfig.name}" uses the stdio transport, which is not enabled for outbound connections (see server/infrastructure/mcp/client.ts)`)
  }

  if (!serverConfig.url) {
    throw new Error(`Server "${serverConfig.name}" is missing a url for the ${serverConfig.transport} transport`)
  }

  const url = new URL(serverConfig.url)
  await assertSafeUrl(url, `Server "${serverConfig.name}"`)
  const firstParty = resolveFirstPartyRemote(url, serverConfig.name)

  if (firstParty) {
    if (serverConfig.transport !== 'http') {
      throw new Error('The configured first-party remote MCP resource must use the http transport')
    }
    const guardedFetch = createSsrfSafeFetch(`Server "${serverConfig.name}"`)
    const client = new FirstPartyRelayMcpClient(
      url,
      firstParty.accessToken,
      withFirstPartyTrace(guardedFetch)
    )
    await client.connect()
    return client
  }

  const client = new Client({ name: MCP_CLIENT_INFO.name, version: MCP_CLIENT_INFO.version }, { capabilities: {} })
  const transport = serverConfig.transport === 'sse'
    ? new SSEClientTransport(url)
    : new StreamableHTTPClientTransport(url, {
        fetch: createSsrfSafeFetch(`Server "${serverConfig.name}"`)
      })

  await client.connect(transport)
  return client as unknown as McpClientLike
}
