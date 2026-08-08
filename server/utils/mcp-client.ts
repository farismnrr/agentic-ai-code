import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js'
import { SSEClientTransport } from '@modelcontextprotocol/sdk/client/sse.js'
import type { InferSelectModel } from 'drizzle-orm'
import type { mcpServers } from '../database/schema'

type McpServerConfig = InferSelectModel<typeof mcpServers>

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
  const transport = serverConfig.transport === 'sse'
    ? new SSEClientTransport(url)
    : new StreamableHTTPClientTransport(url)

  await client.connect(transport)
  return client
}
