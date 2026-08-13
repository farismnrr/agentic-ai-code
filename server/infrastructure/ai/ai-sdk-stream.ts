import { createUIMessageStreamResponse, convertToModelMessages, extractReasoningMiddleware, stepCountIs, streamText, toUIMessageStream, wrapLanguageModel, type LanguageModel, type ToolApprovalConfiguration, type ToolSet } from 'ai'
import type { ProviderOptions } from '@ai-sdk/provider-utils'
import type { UIMessage } from '#shared/types/chat'

/** A concrete language-model instance, as opposed to the `LanguageModel`
 * union's global-registry-id string form — every provider adapter under
 * `server/utils/providers/**` returns one of these, never a bare id. */
export type ConcreteLanguageModel = Exclude<LanguageModel, string>

/**
 * AI SDK model/stream construction (reasoning-middleware wrapping, model
 * message conversion, `streamText`/UI-stream wiring) is an infrastructure
 * integration concern per Plan 031A finding G — application orchestration
 * decides *whether* thinking is enabled and *which* messages/tools to send,
 * but never touches the AI SDK's own construction APIs directly.
 */
export function prepareAiSdkModel(model: ConcreteLanguageModel, thinkingEnabled: boolean): ConcreteLanguageModel {
  if (!thinkingEnabled) return model
  return wrapLanguageModel({ model, middleware: extractReasoningMiddleware({ tagName: 'think' }) })
}

export function convertTurnMessages(messages: UIMessage[], tools: ToolSet) {
  return convertToModelMessages(messages, { tools })
}

export async function streamAiSdkAgent({
  model,
  system,
  messages,
  originalMessages,
  tools,
  toolApproval,
  maxOutputTokens,
  abortSignal,
  providerOptions,
  cleanup,
  persistAssistantMessage
}: {
  model: LanguageModel
  system?: string
  messages: Awaited<ReturnType<typeof convertToModelMessages>>
  originalMessages: UIMessage[]
  tools: ToolSet
  toolApproval?: ToolApprovalConfiguration<ToolSet, never>
  maxOutputTokens?: number
  abortSignal: AbortSignal
  providerOptions?: ProviderOptions
  cleanup: () => Promise<void>
  persistAssistantMessage: (parts: UIMessage['parts'], isContinuation: boolean, totalTokens?: number) => Promise<void>
}) {
  const result = streamText({
    model,
    system,
    messages,
    tools,
    toolApproval,
    stopWhen: stepCountIs(20),
    timeout: { totalMs: 180_000, stepMs: 60_000 },
    maxOutputTokens,
    abortSignal,
    providerOptions,
    onError: ({ error }) => {
      logger.error('[chat stream]', error)
    }
  })

  const uiStream = toUIMessageStream({
    stream: result.stream,
    tools,
    originalMessages,
    onEnd: async ({ responseMessage, isContinuation }) => {
      await cleanup()
      let totalTokens: number | undefined
      try {
        const step = await result.finalStep
        if (step?.usage?.totalTokens) totalTokens = step.usage.totalTokens
      } catch {
        // Preserve the existing behavior: persistence still runs without usage.
      }
      await persistAssistantMessage(responseMessage.parts, isContinuation, totalTokens)
    }
  })

  return createUIMessageStreamResponse({ stream: uiStream })
}
