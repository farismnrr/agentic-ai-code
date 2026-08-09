import { getOpenAiCompatibleModel, listOpenAiCompatibleModels } from './openai-compatible'
import { getAnthropicCompatibleModel, listAnthropicCompatibleModels } from './anthropic-compatible'
import { getVertexAiModel } from './vertex-ai'
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
    // Vertex AI Express Mode has no ListModels/discovery endpoint (confirmed
    // against Google's own Express Mode REST reference) — providers.ts's
    // listProviderModelIds() short-circuits before ever calling this for a
    // vertex_ai row, so this only fires if listProviderModels() is ever
    // called directly, bypassing that guard.
    throw new Error('Vertex AI Express Mode has no model-listing endpoint')
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
