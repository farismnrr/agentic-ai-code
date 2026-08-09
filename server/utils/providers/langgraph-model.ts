import { ChatOpenAI } from '@langchain/openai'
import { ChatGoogleGenerativeAI } from '@langchain/google-genai'
import { decryptSecret } from '../crypto'
import type { modelProviders } from '../../database/schema'
import type { InferSelectModel } from 'drizzle-orm'

type ModelProviderRow = InferSelectModel<typeof modelProviders>

export function getLanggraphModel(provider: ModelProviderRow, modelId: string, maxOutputTokens?: number) {
  const apiKey = decryptSecret(provider.apiKeyEncrypted)
  if (provider.type === '9router') {
    if (!provider.baseUrl) throw new Error('9Router provider requires a base URL')
    return new ChatOpenAI({
      modelName: modelId,
      maxTokens: maxOutputTokens,
      configuration: {
        baseURL: provider.baseUrl,
        apiKey
      }
    })
  }
  if (provider.type === 'gcp_agent_platform') {
    return new ChatGoogleGenerativeAI({
      model: modelId,
      maxOutputTokens,
      apiKey
    })
  }
  throw new Error(`Unknown provider type: ${(provider as ModelProviderRow).type}`)
}
