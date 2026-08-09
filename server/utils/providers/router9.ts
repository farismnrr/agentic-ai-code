import { createOpenAICompatible } from '@ai-sdk/openai-compatible'
import { decryptSecret } from '../crypto'

export function getRouter9Model(modelId: string, baseUrl: string, encryptedApiKey: string) {
  const provider = createOpenAICompatible({
    name: '9router',
    baseURL: baseUrl,
    apiKey: decryptSecret(encryptedApiKey)
  })
  return provider.chatModel(modelId)
}

/**
 * 9Router speaks the OpenAI chat-completions wire format, which includes
 * the standard `GET /models` list endpoint — used here instead of a
 * hand-maintained model list so "which models can I pick" always matches
 * what the router actually has configured.
 */
export async function listRouter9Models(baseUrl: string, encryptedApiKey: string) {
  const apiKey = decryptSecret(encryptedApiKey)
  const response = await fetch(`${baseUrl.replace(/\/$/, '')}/models`, {
    headers: { Authorization: `Bearer ${apiKey}` }
  })
  if (!response.ok) {
    throw new Error(`9Router model list request failed: ${response.status} ${response.statusText}`)
  }
  const body = await response.json() as { data?: { id: string }[] }
  return (body.data ?? []).map(m => m.id).sort()
}
