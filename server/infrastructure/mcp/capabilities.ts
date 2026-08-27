import { and, eq, ne } from 'drizzle-orm'
import { mcpServers } from '../../database/schema'
import { useDb } from '../database/connection'
import type { McpTool } from '#shared/types/chat'

export interface McpExecutionContext {
  terminalAvailable: boolean
  enabledToolIds: string[]
}

const TERMINAL_CAPABILITY_TOOL = 'terminal_exec'

function parseTools(value: unknown): McpTool[] {
  if (Array.isArray(value)) return value as McpTool[]
  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value)
      return Array.isArray(parsed) ? parsed as McpTool[] : []
    } catch {
      return []
    }
  }
  return []
}

/**
 * Resolve the current account-scoped MCP execution surface from Settings.
 *
 * Conversation-level tool IDs are deliberately not consulted here. Settings
 * is the canonical source for whether a server is enabled/connected, while
 * effect + provenance filtering in buildMcpTools remains the authority
 * boundary for what a model can actually call. Terminal availability is only
 * a product-mode gate: a matching external tool does not gain first-party
 * trust merely by using the `terminal_exec` name.
 */
export async function resolveMcpExecutionContext(userId: string): Promise<McpExecutionContext> {
  const servers = await useDb()
    .select()
    .from(mcpServers)
    .where(and(
      eq(mcpServers.userId, userId),
      eq(mcpServers.enabled, true),
      eq(mcpServers.status, 'connected'),
      ne(mcpServers.transport, 'stdio')
    ))

  // OAuth-backed rows are self-contained and can be executed directly from
  // stored, encrypted credentials. A legacy first-party-relay row, however,
  // may still carry a stale `connected` status from the former static-token
  // deployment. Exclude only that stale shape; ordinary external/public MCP
  // servers without OAuth remain valid.
  const executableInventories = servers
    .map(server => ({ server, tools: parseTools(server.tools) }))
    .filter(({ server, tools }) => Boolean(server.oauthTokensEncrypted) || !tools.some(tool => tool.trustedProvenance === 'first-party-relay'))
  const toolInventories = executableInventories.map(({ tools }) => tools)
  const enabledToolIds = [...new Set(toolInventories.flatMap(tools => tools.map(tool => tool.id)))]
  const terminalAvailable = toolInventories.some(tools => tools.some(tool => tool.name === TERMINAL_CAPABILITY_TOOL))

  return { terminalAvailable, enabledToolIds }
}
