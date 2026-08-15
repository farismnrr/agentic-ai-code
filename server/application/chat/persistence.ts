import type { UIMessage } from '#shared/types/chat'
import type { ChatPersistencePort } from './contracts'
import type { RequestTelemetryContext } from '../observability/contracts'

export function createAssistantPersister({ conversationId, providerType, close, persistence, telemetry }: { conversationId: string, modelId: string, providerType: string, close: () => Promise<void>, persistence: ChatPersistencePort, telemetry?: RequestTelemetryContext }) {
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
        telemetry?.event('chat.stream.tool_calls', 'ok', {
          'provider.type': providerType,
          'tool.name': 'assistant_tool_calls',
          'attempt': toolParts.length
        })
      }
      if (isContinuation) {
        const last = await persistence.findLast(conversationId)
        if (last?.role === 'assistant') {
          await persistence.updateAssistant(last.id, parts, totalTokens)
          if (totalTokens != null) await persistence.cacheTokens(conversationId, last.id, totalTokens)
          telemetry?.event('chat.stream.persist', 'ok', { 'provider.type': providerType })
          return
        }
      }
      const inserted = await persistence.insertAssistant(conversationId, parts, totalTokens)
      if (totalTokens != null && inserted) await persistence.cacheTokens(conversationId, inserted.id, totalTokens)
      telemetry?.event('chat.stream.persist', 'ok', { 'provider.type': providerType })
    } catch (err) {
      telemetry?.error('chat.stream.persist', 'chat_persist_failed', err, { 'provider.type': providerType })
    }
  }
  return { persist, cleanup: closeOnce }
}
