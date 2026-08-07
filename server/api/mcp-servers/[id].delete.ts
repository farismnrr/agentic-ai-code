import { eq, and } from 'drizzle-orm'
import { mcpServers } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing server ID')

  const db = useDb()

  const [deleted] = await db
    .delete(mcpServers)
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, session.user.id)))
    .returning()

  if (!deleted) {
    throw notFound('Server not found')
  }

  return { ok: true }
})
