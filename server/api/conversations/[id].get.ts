import { eq, and, asc } from 'drizzle-orm'
import { conversations, messages as messagesTable } from '../../database/schema'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')

  const db = useDb()

  const [conversation] = await db
    .select()
    .from(conversations)
    .where(and(eq(conversations.id, id), eq(conversations.userId, session.user.id)))
    .limit(1)

  if (!conversation) {
    throw notFound('Conversation not found')
  }

  const msgs = await db
    .select()
    .from(messagesTable)
    .where(eq(messagesTable.conversationId, conversation.id))
    .orderBy(asc(messagesTable.createdAt))

  return {
    id: conversation.id,
    title: conversation.title,
    modelId: conversation.modelId,
    enabledToolIds: conversation.enabledToolIds,
    approvals: conversation.approvals,
    createdAt: conversation.createdAt.getTime(),
    updatedAt: conversation.updatedAt.getTime(),
    messages: msgs.map(m => ({
      id: m.id,
      role: m.role,
      createdAt: m.createdAt,
      // parts is JSON parsed by the db driver or stored as jsonb
      parts: Array.isArray(m.parts) ? m.parts : (typeof m.parts === 'string' ? JSON.parse(m.parts) : m.parts)
    }))
  }
})
