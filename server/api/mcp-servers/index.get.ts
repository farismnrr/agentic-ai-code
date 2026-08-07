import { eq } from 'drizzle-orm'
import { mcpServers } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const db = useDb()

  const servers = await db
    .select()
    .from(mcpServers)
    .where(eq(mcpServers.userId, session.user.id))

  return servers.map(s => ({
    id: s.id,
    name: s.name,
    description: s.description,
    transport: s.transport,
    url: s.url,
    command: s.command,
    status: s.status,
    enabled: s.enabled,
    tools: Array.isArray(s.tools) ? s.tools : (typeof s.tools === 'string' ? JSON.parse(s.tools) : s.tools)
  }))
})
