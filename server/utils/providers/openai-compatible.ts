import { createOpenAICompatible } from '@ai-sdk/openai-compatible'
import { decryptHeaders, decryptSecret } from '../crypto'
import { assertSafeUrl, createSsrfSafeFetch } from '../ssrf-guard'

export function getOpenAiCompatibleModel(modelId: string, baseUrl: string, encryptedApiKey: string, encryptedCustomHeaders: Record<string, string>) {
  const provider = createOpenAICompatible({
    name: 'openai-compatible',
    baseURL: baseUrl,
    apiKey: decryptSecret(encryptedApiKey),
    headers: decryptHeaders(encryptedCustomHeaders),
    // Same SSRF policy already used for outbound MCP connections — see
    // `createSsrfSafeFetch` for the exact guarantees/residual risk.
    fetch: createSsrfSafeFetch('OpenAI-compatible provider base URL')
  })
  return provider.chatModel(modelId)
}

/**
 * OpenAI-compatible services (9Router included) implement the standard
 * `GET /models` list endpoint — used here instead of a hand-maintained
 * model list so "which models can I pick" always matches what the
 * provider actually has configured.
 */
export async function listOpenAiCompatibleModels(baseUrl: string, encryptedApiKey: string, encryptedCustomHeaders: Record<string, string>) {
  const apiKey = decryptSecret(encryptedApiKey)
  const customHeaders = decryptHeaders(encryptedCustomHeaders)
  const url = new URL(`${baseUrl.replace(/\/$/, '')}/models`)
  await assertSafeUrl(url, 'OpenAI-compatible provider base URL')
  const response = await fetch(url, {
    headers: { Authorization: `Bearer ${apiKey}`, ...customHeaders }
  })
  if (!response.ok) {
    throw new Error(`Model list request failed: ${response.status} ${response.statusText}`)
  }
  const body = await response.json() as { data?: { id: string }[] }
  return (body.data ?? []).map(m => m.id).sort().map(id => ({ label: id, value: id }))
}
