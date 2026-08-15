import { createUIMessageStreamResponse } from 'ai'
import { runLanggraphChat } from './langgraph/langgraph-chat'
import type { UIMessage } from '#shared/types/chat'
import type { getLanggraphModel } from './providers/langgraph-model'
import type { RequestTelemetryContext } from '../../application/observability/contracts'

/**
 * LangGraph stream construction is an infrastructure integration concern
 * per Plan 031A finding G — application orchestration decides which model,
 * messages, and system prompt to use, but never touches LangGraph's own
 * run/stream wiring directly.
 */
export function streamLangGraphChat({
  messages,
  model,
  system,
  abortSignal,
  cleanup,
  persistAssistantMessage,
  telemetry
}: {
  messages: UIMessage[]
  model: ReturnType<typeof getLanggraphModel>
  system?: string
  abortSignal: AbortSignal
  cleanup: () => Promise<void>
  persistAssistantMessage: (parts: UIMessage['parts'], isContinuation: boolean, totalTokens?: number) => Promise<void>
  telemetry?: RequestTelemetryContext
}) {
  const stream = runLanggraphChat({
    uiMessages: messages,
    baseModel: model,
    systemPrompt: system,
    abortSignal,
    cleanup,
    onEnd: async (parts, totalTokens) => {
      await persistAssistantMessage(parts, false, totalTokens)
    },
    telemetry
  })
  return createUIMessageStreamResponse({ stream })
}
