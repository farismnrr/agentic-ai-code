import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js'
import { SSEClientTransport } from '@modelcontextprotocol/sdk/client/sse.js'
import { lookup } from 'node:dns/promises'
import { isIPv4, isIPv6 } from 'node:net'
import type { InferSelectModel } from 'drizzle-orm'
import type { mcpServers } from '../database/schema'

type McpServerConfig = InferSelectModel<typeof mcpServers>

/**
 * True for addresses that shouldn't be reachable from a server-side
 * "connect to whatever URL a user typed in" feature: loopback, private
 * (RFC1918), link-local (includes the 169.254.169.254 cloud-metadata
 * address), and IPv6 equivalents.
 */
function isDisallowedAddress(address: string) {
  if (isIPv4(address)) {
    const [a, b] = address.split('.').map(Number)
    return a === 127 || a === 10 || a === 0
      || (a === 169 && b === 254)
      || (a === 172 && b !== undefined && b >= 16 && b <= 31)
      || (a === 192 && b === 168)
  }
  if (isIPv6(address)) {
    const normalized = address.toLowerCase()
    return normalized === '::1' || normalized === '::'
      || normalized.startsWith('fe80:') // link-local, includes IPv6 metadata equivalents
      || normalized.startsWith('fc') || normalized.startsWith('fd') // unique local, fc00::/7
      || normalized.startsWith('::ffff:127.') || normalized.startsWith('::ffff:169.254.') // IPv4-mapped loopback/link-local
  }
  return true // unrecognized address shape — fail closed
}

/**
 * Rejects anything that isn't a plain http(s) URL resolving to a public
 * address. Re-resolves DNS right before connecting (not just at
 * mcp_servers create/update time) so a hostname that resolves safely at
 * registration and to an internal address later (DNS rebinding) is still
 * caught — this is a per-request connector, so the cost of a fresh lookup
 * here is negligible.
 */
async function assertSafeMcpUrl(url: URL, serverName: string) {
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new Error(`Server "${serverName}" has an unsupported URL scheme "${url.protocol}"`)
  }

  let addresses: string[]
  try {
    addresses = (await lookup(url.hostname, { all: true })).map(a => a.address)
  } catch {
    throw new Error(`Server "${serverName}" has a URL that could not be resolved`)
  }

  if (addresses.length === 0 || addresses.some(isDisallowedAddress)) {
    throw new Error(`Server "${serverName}" resolves to a disallowed address`)
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
 */
export async function createMcpClient(serverConfig: McpServerConfig) {
  const client = new Client({ name: 'ai-code', version: '1.0.0' }, { capabilities: {} })

  if (serverConfig.transport === 'stdio') {
    throw new Error(`Server "${serverConfig.name}" uses the stdio transport, which is not enabled for outbound connections (see server/utils/mcp-client.ts)`)
  }

  if (!serverConfig.url) {
    throw new Error(`Server "${serverConfig.name}" is missing a url for the ${serverConfig.transport} transport`)
  }

  const url = new URL(serverConfig.url)
  await assertSafeMcpUrl(url, serverConfig.name)

  const transport = serverConfig.transport === 'sse'
    ? new SSEClientTransport(url)
    : new StreamableHTTPClientTransport(url)

  await client.connect(transport)
  return client
}
