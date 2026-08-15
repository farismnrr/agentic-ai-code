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

function getRemoteMcpRuntimeConfig(): RemoteMcpRuntimeConfig {
  const config = useRuntimeConfig()
  return config.remoteMcp as RemoteMcpRuntimeConfig
}

/**
 * Returns the private bearer token only when the stored MCP URL is the exact
 * first-party remote resource configured by the operator. This avoids a
 * generic "send this secret to any user-provided URL" capability.
 */
function resolveFirstPartyBearer(url: URL, serverName: string) {
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

  return accessToken
}

/**
 * Add W3C trace context only for the first-party remote relay. Sending our
 * internal trace identity to arbitrary third-party MCP servers would broaden
 * the telemetry trust boundary unnecessarily.
 */
function withFirstPartyTrace(fetchImpl: typeof fetch, enabled: boolean): typeof fetch {
  if (!enabled) return fetchImpl

  return async (input, init) => {
    const traceHeaders: Record<string, string> = {}
    propagation.inject(context.active(), traceHeaders)

    // Fetch semantics let `init.headers` override headers carried by a Request.
    // Preserve that precedence before injecting trace context; otherwise a
    // custom transport that supplies Request + init could accidentally lose its
    // Authorization header when this wrapper is active.
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

/**
 * Temporary Plan 036 bridge for the monolithic MCP SDK v1 still used by the
 * repository. The Rust first-party relay is strict MCP 2026-07-28 for normal
 * tool RPCs, while the v1 Client performs a legacy `initialize` handshake and
 * then emits legacy-shaped `tools/list` / `tools/call` requests.
 *
 * The relay deliberately keeps its modern validation strict. Instead, only
 * requests to the exact first-party resource are upgraded at this outbound
 * infrastructure boundary by adding the 2026 routing headers and per-request
 * `_meta` envelope. `initialize` and notifications are left untouched so the
 * existing narrow legacy handshake compatibility continues to work.
 *
 * Remove this bridge when the outbound client is migrated to
 * `@modelcontextprotocol/client` v2 with `versionNegotiation: { mode: 'auto' }`
 * and the lockfile/local verification can be updated together.
 */
function withFirstPartyRelay2026ToolEnvelope(fetchImpl: typeof fetch): typeof fetch {
  return async (input, init) => {
    const method = init?.method ?? (input instanceof Request ? input.method : 'GET')
    if (method.toUpperCase() !== 'POST' || typeof init?.body !== 'string') {
      return fetchImpl(input, init)
    }

    let payload: unknown
    try {
      payload = JSON.parse(init.body)
    } catch {
      return fetchImpl(input, init)
    }
    if (!isJsonRecord(payload)) return fetchImpl(input, init)

    const rpcMethod = typeof payload.method === 'string' ? payload.method : undefined
    if (rpcMethod !== 'tools/list' && rpcMethod !== 'tools/call') {
      return fetchImpl(input, init)
    }

    const params = isJsonRecord(payload.params) ? { ...payload.params } : {}
    const existingMeta = isJsonRecord(params._meta) ? params._meta : {}
    params._meta = {
      ...existingMeta,
      'io.modelcontextprotocol/protocolVersion': MODERN_MCP_VERSION,
      'io.modelcontextprotocol/clientCapabilities': {}
    }

    const headers = new Headers(input instanceof Request ? input.headers : undefined)
    for (const [name, value] of new Headers(init.headers).entries()) {
      headers.set(name, value)
    }
    headers.set('MCP-Protocol-Version', MODERN_MCP_VERSION)
    headers.set('Mcp-Method', rpcMethod)

    if (rpcMethod === 'tools/call') {
      const toolName = typeof params.name === 'string' ? params.name : undefined
      if (toolName) headers.set('Mcp-Name', toolName)
    }

    return fetchImpl(input, {
      ...init,
      headers,
      body: JSON.stringify({ ...payload, params })
    })
  }
}

/**
 * Connects to a stored third-party MCP server, per request — no pooling, no
 * reconnect logic (see plan 012's "Scope boundary" decision). Callers must
 * `client.close()` when done.
 *
 * `stdio` transport is deliberately unsupported here: it would spawn
 * `mcpServers.command` — a value any authenticated user can set, including
 * through the inbound `create_mcp_server` MCP tool — as a server-side child
 * process. That's an RCE path in a multi-tenant app, flagged in the plan's
 * Decisions and left unresolved for Phase 1; resolving it (allow-listing,
 * admin gating, or a sandboxed runner) is separate work, not a shortcut to
 * take here. Rows with `transport: 'stdio'` fail closed instead.
 *
 * `http`/`sse` rows carry a user-supplied `url` with no upstream
 * validation (`mcpServers.url` is a bare optional string at the API layer)
 * — this is what actually makes the outbound connection, so the SSRF guard
 * belongs here: reject non-http(s) schemes and any hostname resolving to a
 * loopback/private/link-local address (including cloud metadata IPs),
 * re-checked on every connection rather than only at server registration.
 *
 * Plan 036 adds one deliberately narrow authenticated path: when an HTTP
 * server URL exactly matches private `runtimeConfig.remoteMcp.url`, the
 * corresponding private access token is attached as a Bearer credential.
 * The same SSRF-safe fetch used by provider traffic validates every redirect
 * hop and rejects cross-origin/downgraded redirects, so the credential cannot
 * be silently forwarded to another origin.
 */
export async function createMcpClient(serverConfig: McpServerConfig) {
  const client = new Client({ name: 'ai-code', version: '1.0.0' }, { capabilities: {} })

  if (serverConfig.transport === 'stdio') {
    throw new Error(`Server "${serverConfig.name}" uses the stdio transport, which is not enabled for outbound connections (see server/infrastructure/mcp/client.ts)`)
  }

  if (!serverConfig.url) {
    throw new Error(`Server "${serverConfig.name}" is missing a url for the ${serverConfig.transport} transport`)
  }

  const url = new URL(serverConfig.url)
  await assertSafeUrl(url, `Server "${serverConfig.name}"`)

  if (serverConfig.transport === 'sse') {
    // Legacy SSE remains supported for existing third-party integrations. The
    // Plan 036 first-party remote relay is Streamable HTTP only, so its OAuth
    // credential is never attached to this legacy transport.
    const transport = new SSEClientTransport(url)
    await client.connect(transport)
    return client
  }

  const accessToken = resolveFirstPartyBearer(url, serverConfig.name)
  const firstParty = Boolean(accessToken)
  const guardedFetch = createSsrfSafeFetch(`Server "${serverConfig.name}"`)
  const protocolFetch = firstParty
    ? withFirstPartyRelay2026ToolEnvelope(guardedFetch)
    : guardedFetch
  const transport = new StreamableHTTPClientTransport(url, {
    fetch: withFirstPartyTrace(protocolFetch, firstParty),
    ...(accessToken && {
      requestInit: {
        headers: {
          Authorization: `Bearer ${accessToken}`
        }
      }
    })
  })

  await client.connect(transport)
  return client
}
