import { and, eq, inArray } from 'drizzle-orm'
import { useDb } from '../database/connection'
import { mcpServers } from '../../database/schema'

export async function loadEnabledMcpServers(userId: string, serverIds: string[]) {
  if (serverIds.length === 0) return []
  return useDb().select().from(mcpServers).where(and(eq(mcpServers.userId, userId), eq(mcpServers.enabled, true), inArray(mcpServers.id, serverIds)))
}
