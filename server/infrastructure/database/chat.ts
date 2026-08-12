import { and, asc, eq, gt } from 'drizzle-orm'
import { conversations, messages as messagesTable, modelProviders, models } from '../../database/schema'
import type { UIMessage } from '#shared/types/chat'
import { badRequest, internal, notFound } from '../../utils/http-errors'

export async function loadAuthorizedChatContext(userId: string, conversationId: string) {
  if (!conversationId) throw badRequest('Missing conversationId')
  const db = useDb()
  const [conversation] = await db.select().from(conversations).where(and(eq(conversations.id, conversationId), eq(conversations.userId, userId))).limit(1)
  if (!conversation) throw notFound('Conversation not found')
  const [model] = await db.select().from(models).where(eq(models.id, conversation.modelId)).limit(1)
  if (!model) throw notFound('Model not found')
  const [provider] = await db.select().from(modelProviders).where(eq(modelProviders.id, model.providerId)).limit(1)
  if (!provider) throw notFound('Model provider not found')
  return { conversation, model, provider }
}

export async function loadChatHistory(conversation: typeof conversations.$inferSelect, trigger: string | undefined, message: UIMessage | undefined) {
  const db = useDb()
  const historyWhere = conversation.contextSummaryUpToCreatedAt
    ? and(eq(messagesTable.conversationId, conversation.id), gt(messagesTable.createdAt, conversation.contextSummaryUpToCreatedAt))
    : eq(messagesTable.conversationId, conversation.id)
  const rows = await db.select().from(messagesTable).where(historyWhere).orderBy(asc(messagesTable.createdAt))
  let messages: UIMessage[] = rows.map(row => ({ id: row.id, role: row.role as UIMessage['role'], parts: row.parts as UIMessage['parts'] }))
  if (trigger === 'submit-message' && message?.role === 'user') {
    const [inserted] = await db.insert(messagesTable).values({ conversationId: conversation.id, role: 'user', parts: message.parts }).returning({ id: messagesTable.id })
    if (!inserted) throw internal('Failed to insert user message')
    messages.push({ ...message, id: inserted.id })
  } else if (trigger === 'regenerate-message') {
    if (messages.at(-1)?.role === 'assistant') messages = messages.slice(0, -1)
  } else if (message && messages.length > 0) {
    messages[messages.length - 1] = message
  }
  return messages
}
