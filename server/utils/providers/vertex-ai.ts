import { createVertex } from '@ai-sdk/google-vertex'
import { decryptSecret } from '../crypto'

/**
 * Real Vertex AI (aiplatform.googleapis.com), not the Gemini Developer API
 * (generativelanguage.googleapis.com) — those are two separate Google
 * products with incompatible API keys, which is why an actual Vertex AI
 * key thrown at the Gemini API 403s. Express Mode is what makes this work
 * with just an API key: no project ID, no location, no service account.
 * See https://ai-sdk.dev/providers/ai-sdk-providers/google-vertex and
 * https://cloud.google.com/vertex-ai/generative-ai/docs/start/express-mode/overview
 */
export function getVertexAiModel(modelId: string, encryptedApiKey: string) {
  const provider = createVertex({
    apiKey: decryptSecret(encryptedApiKey)
  })
  return provider.languageModel(modelId)
}
