import { desc, eq } from 'drizzle-orm'
import { conversations, messages as messagesTable } from '../../database/schema'
import type { UIMessage } from '#shared/types/chat'

type Db = ReturnType<typeof useDb>

export function createAssistantPersister({ db, conversationId, modelId, providerType, close }: { db: Db, conversationId: string, modelId: string, providerType: string, close: () => Promise<void> }) {
  let closed = false
  const closeOnce = async () => {
    if (closed) return
    closed = true
    await close()
  }
  const cacheLastMeasuredTokens = async (messageId: string, totalTokens: number) => {
    await db.update(conversations).set({ lastMeasuredTokens: totalTokens, lastMeasuredMessageId: messageId }).where(eq(conversations.id, conversationId))
  }
  const persist = async function persist(parts: UIMessage['parts'], isContinuation = false, totalTokens?: number | null) {
    try {
      await closeOnce()
      const toolParts = parts.filter(part => String(part.type).startsWith('tool-'))
      if (toolParts.length > 0) {
        logger.info('[chat persist] assistant message with tool calls', {
          conversationId,
          modelId,
          providerType,
          isContinuation,
          toolCallCount: toolParts.length,
          toolCallsMissingProviderMetadata: toolParts.filter(part => !('callProviderMetadata' in part) && !('resultProviderMetadata' in part)).length
        })
      }
      if (isContinuation) {
        const [last] = await db.select().from(messagesTable).where(eq(messagesTable.conversationId, conversationId)).orderBy(desc(messagesTable.createdAt)).limit(1)
        if (last?.role === 'assistant') {
          const updateData: { parts: UIMessage['parts'], totalTokens?: number } = { parts }
          if (totalTokens != null) updateData.totalTokens = totalTokens
          await db.update(messagesTable).set(updateData).where(eq(messagesTable.id, last.id))
          if (totalTokens != null) await cacheLastMeasuredTokens(last.id, totalTokens)
          return
        }
      }
      const [inserted] = await db.insert(messagesTable).values({ conversationId, role: 'assistant', parts, totalTokens }).returning({ id: messagesTable.id })
      if (totalTokens != null && inserted) await cacheLastMeasuredTokens(inserted.id, totalTokens)
    } catch (err) {
      logger.error('[chat onEnd] failed to persist assistant message', err)
    }
  }
  return { persist, cleanup: closeOnce }
}
