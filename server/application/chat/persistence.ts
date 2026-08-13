import type { UIMessage } from '#shared/types/chat'
import type { ChatPersistencePort } from './contracts'

export function createAssistantPersister({ conversationId, modelId, providerType, close, persistence }: { conversationId: string, modelId: string, providerType: string, close: () => Promise<void>, persistence: ChatPersistencePort }) {
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
        const last = await persistence.findLast(conversationId)
        if (last?.role === 'assistant') {
          await persistence.updateAssistant(last.id, parts, totalTokens)
          if (totalTokens != null) await persistence.cacheTokens(conversationId, last.id, totalTokens)
          return
        }
      }
      const inserted = await persistence.insertAssistant(conversationId, parts, totalTokens)
      if (totalTokens != null && inserted) await persistence.cacheTokens(conversationId, inserted.id, totalTokens)
    } catch (err) {
      logger.error('[chat onEnd] failed to persist assistant message', err)
    }
  }
  return { persist, cleanup: closeOnce }
}
