import { eq, and } from 'drizzle-orm'
import { conversations } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw createError({ statusCode: 400, message: 'Missing conversation ID' })

  const db = useDb()

  const [deleted] = await db
    .delete(conversations)
    .where(and(eq(conversations.id, id), eq(conversations.userId, session.user.id)))
    .returning()

  if (!deleted) {
    throw createError({ statusCode: 404, message: 'Conversation not found' })
  }

  return { ok: true }
})
