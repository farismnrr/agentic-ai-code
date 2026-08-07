import { eq, and } from 'drizzle-orm'
import { mcpServers } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw createError({ statusCode: 400, message: 'Missing server ID' })

  const db = useDb()

  const [deleted] = await db
    .delete(mcpServers)
    .where(and(eq(mcpServers.id, id), eq(mcpServers.userId, session.user.id)))
    .returning()

  if (!deleted) {
    throw createError({ statusCode: 404, message: 'Server not found' })
  }

  return { ok: true }
})
