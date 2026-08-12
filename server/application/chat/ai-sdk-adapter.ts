import { createUIMessageStreamResponse, stepCountIs, streamText, toUIMessageStream, type LanguageModel, type ToolApprovalConfiguration, type ToolSet } from 'ai'
import type { UIMessage } from '#shared/types/chat'

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
  messages: Awaited<ReturnType<typeof import('ai')['convertToModelMessages']>>
  originalMessages: UIMessage[]
  tools: ToolSet
  toolApproval?: ToolApprovalConfiguration<ToolSet, never>
  maxOutputTokens?: number
  abortSignal: AbortSignal
  providerOptions?: Record<string, unknown>
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
