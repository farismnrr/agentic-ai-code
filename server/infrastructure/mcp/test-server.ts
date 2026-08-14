import { logger } from '../observability/logger'
import { and, eq } from 'drizzle-orm'
import { mcpServers } from '../../database/schema'
import type { McpTool } from '#shared/types/chat'

export async function testMcpServer(userId: string, id: string) {
  const db = useDb()
  const [server] = await db.select().from(mcpServers).where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId))).limit(1)
  if (!server) return null
  try {
    const client = await createMcpClient(server)
    let listed
    try {
      listed = await client.listTools()
    } finally {
      await client.close().catch((err: unknown) => logger.error('[mcp test] error closing client', err))
    }
    const tools: McpTool[] = listed.tools.map(t => ({ id: `${server.id}.${t.name}`, serverId: server.id, name: t.name, description: t.description ?? '', sampleInput: {} }))
    const [updated] = await db.update(mcpServers).set({ status: 'connected', tools, updatedAt: new Date() }).where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId))).returning()
    return { id: updated!.id, status: updated!.status, tools }
  } catch (err: unknown) {
    await db.update(mcpServers).set({ status: 'error', updatedAt: new Date() }).where(and(eq(mcpServers.id, id), eq(mcpServers.userId, userId)))
    throw err
  }
}
