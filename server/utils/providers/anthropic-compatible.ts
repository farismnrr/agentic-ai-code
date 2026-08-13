import { createAnthropic } from '@ai-sdk/anthropic'
import { decryptHeaders, decryptSecret } from '../crypto'
import { assertSafeUrl, createSsrfSafeFetch } from '../ssrf-guard'

export function getAnthropicCompatibleModel(modelId: string, baseUrl: string, encryptedApiKey: string, encryptedCustomHeaders: Record<string, string>) {
  const provider = createAnthropic({
    baseURL: baseUrl,
    apiKey: decryptSecret(encryptedApiKey),
    headers: decryptHeaders(encryptedCustomHeaders),
    // Same SSRF policy already used for outbound MCP connections — see
    // `createSsrfSafeFetch` for the exact guarantees/residual risk.
    fetch: createSsrfSafeFetch('Anthropic-compatible provider base URL')
  })
  return provider(modelId)
}

/**
 * The Anthropic Messages API's own `GET /v1/models` — self-hosted
 * Anthropic-compatible proxies generally mirror this same shape.
 */
export async function listAnthropicCompatibleModels(baseUrl: string, encryptedApiKey: string, encryptedCustomHeaders: Record<string, string>) {
  const apiKey = decryptSecret(encryptedApiKey)
  const customHeaders = decryptHeaders(encryptedCustomHeaders)
  const url = new URL(`${baseUrl.replace(/\/$/, '')}/models`)
  await assertSafeUrl(url, 'Anthropic-compatible provider base URL')
  const response = await fetch(url, {
    headers: { 'x-api-key': apiKey, 'anthropic-version': '2023-06-01', ...customHeaders }
  })
  if (!response.ok) {
    throw new Error(`Model list request failed: ${response.status} ${response.statusText}`)
  }
  const body = await response.json() as { data?: { id: string }[] }
  return (body.data ?? []).map(m => m.id).sort()
}
