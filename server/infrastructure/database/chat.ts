import { and, asc, eq, gt } from 'drizzle-orm'
import { conversations, messages as messagesTable } from '../../database/schema'
import type { UIMessage } from '#shared/types/chat'
import { internal } from '../../utils/http-errors'

/**
 * Narrow ownership-scoped lookup: the conversation row itself, no model or
 * provider resolution. Callers that also need model/provider ownership
 * reassertion should compose this with
 * `server/application/chat/ownership.ts#resolveOwnedModelContext` rather
 * than resolving the model/provider by ID here — that keeps this module a
 * plain persistence lookup instead of pulling application-layer
 * authorization logic into infrastructure.
 */
export async function findUserConversation(userId: string, conversationId: string) {
  const db = useDb()
  const [conversation] = await db.select().from(conversations).where(and(eq(conversations.id, conversationId), eq(conversations.userId, userId))).limit(1)
  return conversation
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
