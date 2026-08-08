import { eq, and } from 'drizzle-orm'
import { mcpServers } from '../../../database/schema'
import type { McpTool } from '#shared/types/chat'

/**
 * "Test connection" — connects to a stored server, lists its tools, and
 * persists status/tools on success (or 'error' on failure) so the settings
 * UI reflects reality instead of the optimistic 'connected' default set at
 * creation. Opportunistic refresh only, per plan 012's scope boundary — no
 * polling daemon.
 */
export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing server ID')

  const db = useDb()

  const [server] = await db
    .select()
    .from(mcpServers)
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, session.user.id)))
    .limit(1)

  if (!server) throw notFound('Server not found')

  try {
    const client = await createMcpClient(server)
    let listed
    try {
      listed = await client.listTools()
    } finally {
      await client.close().catch((err: unknown) => console.error('[mcp test] error closing client', err))
    }

    const tools: McpTool[] = listed.tools.map(t => ({
      id: `${server.id}.${t.name}`,
      serverId: server.id,
      name: t.name,
      description: t.description ?? '',
      sampleInput: {}
    }))

    const [updated] = await db
      .update(mcpServers)
      .set({ status: 'connected', tools, updatedAt: new Date() })
      .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, session.user.id)))
      .returning()

    return { id: updated!.id, status: updated!.status, tools }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Unknown error connecting to MCP server'

    await db
      .update(mcpServers)
      .set({ status: 'error', updatedAt: new Date() })
      .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, session.user.id)))

    throw badRequest(message)
  }
})
