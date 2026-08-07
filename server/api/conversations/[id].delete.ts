import { eq, and } from 'drizzle-orm'
import { conversations } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')

  const db = useDb()

  const [deleted] = await db
    .delete(conversations)
    .where(and(eq(conversations.id, id), eq(conversations.userId, session.user.id)))
    .returning()

  if (!deleted) {
    throw notFound('Conversation not found')
  }

  return { ok: true }
})
