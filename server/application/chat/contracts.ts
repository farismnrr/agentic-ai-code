import type { UIMessage } from '#shared/types/chat'

export interface ChatProviderContext { id: string, type: 'openai_compatible' | 'anthropic_compatible' | 'vertex_ai' }
export interface ChatModelContext { id: string, modelId: string, contextWindow: number | null | undefined, maxOutputTokens: number | null | undefined, thinkingEnabled: boolean | null }
export interface ResolvedChatModelConfig { contextWindow: number | undefined, maxOutputTokens: number | undefined, thinkingEnabled: boolean }
export type ChatModelHandle = unknown
export type ChatMessageHandle = unknown
export type ChatToolSet = Record<string, unknown>
export type ChatToolApproval = Record<string, unknown>
export type ChatStreamResult = unknown

export interface ChatTurnDependencies {
  resolveModelConfig(model: ChatModelContext): ResolvedChatModelConfig
  getChatModel(provider: ChatProviderContext, modelId: string): ChatModelHandle
  getLanggraphModel(provider: ChatProviderContext, modelId: string, maxOutputTokens?: number): ChatModelHandle
  resolveMessagesForModel(input: { messages: UIMessage[], conv: { id: string, contextSummary: string | null, contextSummaryUpToMessageId: string | null, lastMeasuredTokens: number | null, lastMeasuredMessageId: string | null }, contextWindow: number | null | undefined, maxOutputTokens: number | null | undefined, getSummarizerModel: () => ChatModelHandle }): Promise<UIMessage[]>
  buildMcpTools(userId: string, enabledToolIds: string[], approvals: Record<string, string>): Promise<{ tools: ChatToolSet, toolApproval?: ChatToolApproval, close: () => Promise<void> }>
  convertTurnMessages(messages: UIMessage[], tools: ChatToolSet): ChatMessageHandle
  prepareAiSdkModel(model: ChatModelHandle, thinkingEnabled: boolean): ChatModelHandle
  streamAiSdkAgent(input: Record<string, unknown>): ChatStreamResult
  streamLangGraphChat(input: Record<string, unknown>): ChatStreamResult
}
