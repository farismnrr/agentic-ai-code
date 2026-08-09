import { ChatOpenAI } from '@langchain/openai'
import { ChatAnthropic } from '@langchain/anthropic'
import { ChatVertexAI } from '@langchain/google-vertexai'
import { decryptSecret } from '../crypto'
import type { modelProviders } from '../../database/schema'
import type { InferSelectModel } from 'drizzle-orm'

type ModelProviderRow = InferSelectModel<typeof modelProviders>

export function getLanggraphModel(provider: ModelProviderRow, modelId: string, maxOutputTokens?: number) {
  const apiKey = decryptSecret(provider.apiKeyEncrypted)
  if (provider.type === 'openai_compatible') {
    if (!provider.baseUrl) throw new Error('OpenAI Compatible providers require a base URL')
    return new ChatOpenAI({
      modelName: modelId,
      maxTokens: maxOutputTokens,
      configuration: {
        baseURL: provider.baseUrl,
        apiKey,
        defaultHeaders: provider.customHeaders
      }
    })
  }
  if (provider.type === 'anthropic_compatible') {
    if (!provider.baseUrl) throw new Error('Anthropic Compatible providers require a base URL')
    return new ChatAnthropic({
      model: modelId,
      maxTokens: maxOutputTokens,
      anthropicApiKey: apiKey,
      anthropicApiUrl: provider.baseUrl,
      clientOptions: {
        defaultHeaders: provider.customHeaders
      }
    })
  }
  if (provider.type === 'vertex_ai') {
    // Express Mode: apiKey alone, no project/location/service-account —
    // see server/utils/providers/vertex-ai.ts for why this isn't
    // @langchain/google-genai (that's the Gemini Developer API, a
    // different Google product with incompatible API keys).
    return new ChatVertexAI({
      model: modelId,
      maxOutputTokens,
      apiKey
    })
  }
  throw new Error(`Unknown provider type: ${(provider as ModelProviderRow).type}`)
}
