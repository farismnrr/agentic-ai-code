import { getRouter9Model } from './router9'
import { getGcpAgentPlatformModel } from './gcp-agent-platform'
import type { modelProviders } from '../../database/schema'
import type { InferSelectModel } from 'drizzle-orm'
import type { ChatModel } from '#shared/types/chat'

type ModelProviderRow = InferSelectModel<typeof modelProviders>

export function getChatModel(provider: ModelProviderRow, modelId: string) {
  if (provider.type === '9router') {
    if (!provider.baseUrl) throw new Error('9Router provider requires a base URL')
    return getRouter9Model(modelId, provider.baseUrl, provider.apiKeyEncrypted)
  }
  if (provider.type === 'gcp_agent_platform') {
    return getGcpAgentPlatformModel(modelId, provider.apiKeyEncrypted)
  }
  throw new Error(`Unknown provider type: ${(provider as any).type}`)
}

export function resolveModelConfig(model: ChatModel, settings: any) {
  return {
    contextWindow: model.contextWindow ?? settings.defaultContextWindow,
    maxOutputTokens: model.maxOutputTokens ?? settings.defaultMaxOutputTokens,
    thinkingEnabled: model.thinkingEnabled ?? settings.defaultThinkingEnabled,
    thinkingMinTokens: model.thinkingMinTokens ?? settings.defaultThinkingMinTokens,
    thinkingMaxTokens: model.thinkingMaxTokens ?? settings.defaultThinkingMaxTokens
  }
}
