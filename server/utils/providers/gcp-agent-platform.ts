import { createGoogleGenerativeAI } from '@ai-sdk/google'
import { decryptSecret } from '../crypto'

export function getGcpAgentPlatformModel(modelId: string, encryptedApiKey: string) {
  const provider = createGoogleGenerativeAI({
    apiKey: decryptSecret(encryptedApiKey)
  })
  return provider(modelId)
}

/**
 * The Gemini API's own ListModels endpoint — filtered to models that
 * actually support `generateContent`, since the response also includes
 * embedding-only and other non-chat models that would just error if picked.
 */
export async function listGcpAgentPlatformModels(encryptedApiKey: string) {
  const apiKey = decryptSecret(encryptedApiKey)
  const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models?key=${apiKey}`)
  if (!response.ok) {
    throw new Error(`GCP Agent Platform model list request failed: ${response.status} ${response.statusText}`)
  }
  const body = await response.json() as { models?: { name: string, supportedGenerationMethods?: string[] }[] }
  return (body.models ?? [])
    .filter(m => m.supportedGenerationMethods?.includes('generateContent'))
    .map(m => m.name.replace(/^models\//, ''))
    .sort()
}
