import { getOpenAiCompatibleModel, listOpenAiCompatibleModels } from './openai-compatible'
import { getAnthropicCompatibleModel, listAnthropicCompatibleModels } from './anthropic-compatible'
import { getVertexAiModel } from './vertex-ai'
import type { modelProviders, ModelProviderType } from '../../database/schema'
import type { InferSelectModel } from 'drizzle-orm'
import type { ChatModel } from '#shared/types/chat'
import type { ConcreteLanguageModel } from '../../infrastructure/ai/ai-sdk-stream'

type ModelProviderRow = InferSelectModel<typeof modelProviders>

type ProviderAdapter = {
  createModel: (modelId: string, provider: ModelProviderRow) => ConcreteLanguageModel
  listModels: (provider: ModelProviderRow) => Promise<{ label: string, value: string }[]>
}

function requireBaseUrl(provider: ModelProviderRow) {
  if (!provider.baseUrl) throw new Error(`${provider.type === 'openai_compatible' ? 'OpenAI' : 'Anthropic'} Compatible providers require a base URL`)
  return provider.baseUrl
}

const providerAdapters: Record<ModelProviderType, ProviderAdapter> = {
  openai_compatible: {
    createModel: (modelId, provider) => getOpenAiCompatibleModel(modelId, requireBaseUrl(provider), provider.apiKeyEncrypted, provider.customHeaders),
    listModels: provider => listOpenAiCompatibleModels(requireBaseUrl(provider), provider.apiKeyEncrypted, provider.customHeaders)
  },
  anthropic_compatible: {
    createModel: (modelId, provider) => getAnthropicCompatibleModel(modelId, requireBaseUrl(provider), provider.apiKeyEncrypted, provider.customHeaders),
    listModels: provider => listAnthropicCompatibleModels(requireBaseUrl(provider), provider.apiKeyEncrypted, provider.customHeaders)
  },
  vertex_ai: {
    createModel: (modelId, provider) => getVertexAiModel(modelId, provider.apiKeyEncrypted),
    listModels: async () => { throw new Error('Vertex AI Express Mode has no model-listing endpoint') }
  }
}

export function getChatModel(provider: ModelProviderRow, modelId: string) {
  return providerAdapters[provider.type].createModel(modelId, provider)
}

export function listProviderModels(provider: ModelProviderRow) {
  return providerAdapters[provider.type].listModels(provider)
}

export function resolveModelConfig(model: ChatModel) {
  return {
    contextWindow: model.contextWindow ?? undefined,
    maxOutputTokens: model.maxOutputTokens ?? undefined,
    thinkingEnabled: model.thinkingEnabled ?? false,
    thinkingMinTokens: model.thinkingMinTokens ?? undefined,
    thinkingMaxTokens: model.thinkingMaxTokens ?? undefined
  }
}
