import { and, asc, desc, eq, gt } from 'drizzle-orm'
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

/**
 * Plain persistence read: every message row after the conversation's
 * context-summary cutoff (or all of them if no compaction has happened
 * yet). This module intentionally does not decide what the caller does
 * with the result — submit/regenerate/resume trigger semantics are an
 * application-layer business decision (Plan 031A finding G) implemented in
 * `server/application/chat/history.ts#buildTurnMessages`, not here.
 */
export async function loadHistoryMessages(conversation: typeof conversations.$inferSelect): Promise<UIMessage[]> {
  const db = useDb()
  const historyWhere = conversation.contextSummaryUpToCreatedAt
    ? and(eq(messagesTable.conversationId, conversation.id), gt(messagesTable.createdAt, conversation.contextSummaryUpToCreatedAt))
    : eq(messagesTable.conversationId, conversation.id)
  const rows = await db.select().from(messagesTable).where(historyWhere).orderBy(asc(messagesTable.createdAt))
  return rows.map(row => ({ id: row.id, role: row.role as UIMessage['role'], parts: row.parts as UIMessage['parts'] }))
}

export async function insertUserMessage(conversationId: string, message: UIMessage) {
  const db = useDb()
  const [inserted] = await db.insert(messagesTable).values({ conversationId, role: 'user', parts: message.parts }).returning({ id: messagesTable.id })
  if (!inserted) throw internal('Failed to insert user message')
  return inserted
}

export async function findLastMessage(conversationId: string) {
  const db = useDb()
  const [last] = await db.select().from(messagesTable).where(eq(messagesTable.conversationId, conversationId)).orderBy(desc(messagesTable.createdAt)).limit(1)
  return last
}

export async function updateAssistantMessage(messageId: string, parts: UIMessage['parts'], totalTokens?: number | null) {
  const db = useDb()
  const updateData: { parts: UIMessage['parts'], totalTokens?: number } = { parts }
  if (totalTokens != null) updateData.totalTokens = totalTokens
  await db.update(messagesTable).set(updateData).where(eq(messagesTable.id, messageId))
}

export async function insertAssistantMessage(conversationId: string, parts: UIMessage['parts'], totalTokens?: number | null) {
  const db = useDb()
  const [inserted] = await db.insert(messagesTable).values({ conversationId, role: 'assistant', parts, totalTokens }).returning({ id: messagesTable.id })
  if (!inserted) throw internal('Failed to insert assistant message')
  return inserted
}

export async function cacheLastMeasuredTokens(conversationId: string, messageId: string, totalTokens: number) {
  const db = useDb()
  await db.update(conversations).set({ lastMeasuredTokens: totalTokens, lastMeasuredMessageId: messageId }).where(eq(conversations.id, conversationId))
}
