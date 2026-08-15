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
    const headers = new Headers(input instanceof Request ? input.headers : init?.headers)
    for (const [name, value] of Object.entries(traceHeaders)) {
      headers.set(name, value)
    }
    return fetchImpl(input, { ...init, headers })
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
  const transport = new StreamableHTTPClientTransport(url, {
    fetch: withFirstPartyTrace(guardedFetch, firstParty),
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
