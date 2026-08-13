import { cacheLastMeasuredTokens, findLastMessage, insertAssistantMessage, updateAssistantMessage } from '../../infrastructure/database/chat'
import type { UIMessage } from '#shared/types/chat'

export function createAssistantPersister({ conversationId, modelId, providerType, close }: { conversationId: string, modelId: string, providerType: string, close: () => Promise<void> }) {
  let closed = false
  const closeOnce = async () => {
    if (closed) return
    closed = true
    await close()
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
        const last = await findLastMessage(conversationId)
        if (last?.role === 'assistant') {
          await updateAssistantMessage(last.id, parts, totalTokens)
          if (totalTokens != null) await cacheLastMeasuredTokens(conversationId, last.id, totalTokens)
          return
        }
      }
      const inserted = await insertAssistantMessage(conversationId, parts, totalTokens)
      if (totalTokens != null && inserted) await cacheLastMeasuredTokens(conversationId, inserted.id, totalTokens)
    } catch (err) {
      logger.error('[chat onEnd] failed to persist assistant message', err)
    }
  }
  return { persist, cleanup: closeOnce }
}
