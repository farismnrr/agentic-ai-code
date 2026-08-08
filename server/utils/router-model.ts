import { createOpenAICompatible } from '@ai-sdk/openai-compatible'

/**
 * 9Router speaks the OpenAI chat-completions wire format (see
 * .agents/memories/007-9router-config.md), so it's addressed through the AI
 * SDK's own OpenAI-compatible provider rather than hand-rolled SSE parsing —
 * that's what makes streamText's tool-call loop, multi-tool-call handling,
 * and tool-approval flow work for free instead of being reimplemented here.
 */
export function getRouterModel(modelId: string) {
  const config = useRuntimeConfig()
  const provider = createOpenAICompatible({
    name: '9router',
    baseURL: config.routerBaseUrl,
    apiKey: config.routerApiKey
  })
  return provider.chatModel(modelId)
}
