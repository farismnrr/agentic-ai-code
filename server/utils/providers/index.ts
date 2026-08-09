import { getOpenAiCompatibleModel, listOpenAiCompatibleModels } from './openai-compatible'
import { getAnthropicCompatibleModel, listAnthropicCompatibleModels } from './anthropic-compatible'
import { getVertexAiModel, listVertexAiModels } from './vertex-ai'
import type { modelProviders } from '../../database/schema'
import type { InferSelectModel } from 'drizzle-orm'
import type { ChatModel } from '#shared/types/chat'

type ModelProviderRow = InferSelectModel<typeof modelProviders>

export function getChatModel(provider: ModelProviderRow, modelId: string) {
  if (provider.type === 'openai_compatible') {
    if (!provider.baseUrl) throw new Error('OpenAI Compatible providers require a base URL')
    return getOpenAiCompatibleModel(modelId, provider.baseUrl, provider.apiKeyEncrypted, provider.customHeaders)
  }
  if (provider.type === 'anthropic_compatible') {
    if (!provider.baseUrl) throw new Error('Anthropic Compatible providers require a base URL')
    return getAnthropicCompatibleModel(modelId, provider.baseUrl, provider.apiKeyEncrypted, provider.customHeaders)
  }
  if (provider.type === 'vertex_ai') {
    return getVertexAiModel(modelId, provider.apiKeyEncrypted)
  }
  throw new Error(`Unknown provider type: ${(provider as ModelProviderRow).type}`)
}

export function listProviderModels(provider: ModelProviderRow) {
  if (provider.type === 'openai_compatible') {
    if (!provider.baseUrl) throw new Error('OpenAI Compatible providers require a base URL')
    return listOpenAiCompatibleModels(provider.baseUrl, provider.apiKeyEncrypted, provider.customHeaders)
  }
  if (provider.type === 'anthropic_compatible') {
    if (!provider.baseUrl) throw new Error('Anthropic Compatible providers require a base URL')
    return listAnthropicCompatibleModels(provider.baseUrl, provider.apiKeyEncrypted, provider.customHeaders)
  }
  if (provider.type === 'vertex_ai') {
    return listVertexAiModels(provider.apiKeyEncrypted)
  }
  throw new Error(`Unknown provider type: ${(provider as ModelProviderRow).type}`)
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
