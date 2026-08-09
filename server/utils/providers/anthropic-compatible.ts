import { createAnthropic } from '@ai-sdk/anthropic'
import { decryptSecret } from '../crypto'

export function getAnthropicCompatibleModel(modelId: string, baseUrl: string, encryptedApiKey: string, customHeaders: Record<string, string>) {
  const provider = createAnthropic({
    baseURL: baseUrl,
    apiKey: decryptSecret(encryptedApiKey),
    headers: customHeaders
  })
  return provider(modelId)
}

/**
 * The Anthropic Messages API's own `GET /v1/models` — self-hosted
 * Anthropic-compatible proxies generally mirror this same shape.
 */
export async function listAnthropicCompatibleModels(baseUrl: string, encryptedApiKey: string, customHeaders: Record<string, string>) {
  const apiKey = decryptSecret(encryptedApiKey)
  const response = await fetch(`${baseUrl.replace(/\/$/, '')}/models`, {
    headers: { 'x-api-key': apiKey, 'anthropic-version': '2023-06-01', ...customHeaders }
  })
  if (!response.ok) {
    throw new Error(`Model list request failed: ${response.status} ${response.statusText}`)
  }
  const body = await response.json() as { data?: { id: string }[] }
  return (body.data ?? []).map(m => m.id).sort()
}
