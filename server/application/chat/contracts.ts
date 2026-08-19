import type { UIMessage } from '#shared/types/chat'
import type { SubagentAuthority } from '#shared/types/subagents'

export interface ChatProviderContext { id: string, type: 'openai_compatible' | 'anthropic_compatible' | 'vertex_ai' }
export interface ChatModelContext { id: string, modelId: string, contextWindow: number | null | undefined, maxOutputTokens: number | null | undefined, thinkingEnabled: boolean | null }
export interface ResolvedChatModelConfig { contextWindow: number | undefined, maxOutputTokens: number | undefined, thinkingEnabled: boolean }
export type ChatModelHandle = unknown
export type ChatMessageHandle = unknown
export type ChatToolSet = Record<string, unknown>
export type ChatToolApproval = Record<string, unknown>
export type ChatStreamResult = unknown
export interface ChatConversation { id: string, modelId: string, workspaceId: string | null, mode: 'chat' | 'agent', permissionMode: 'plan' | 'workspace' | 'autonomous' | 'manual', enabledToolIds: string[], approvals: Record<string, string>, reasoningEffort: string | null, contextSummary: string | null, contextSummaryUpToMessageId: string | null, lastMeasuredTokens: number | null, lastMeasuredMessageId: string | null }
export interface ChatModelRecord extends ChatModelContext { providerId: string }
export interface ChatProviderRecord extends ChatProviderContext { baseUrl?: string | null }
export interface ChatHistoryPort { load(conversation: ChatConversation): Promise<UIMessage[]>, insertUser(conversationId: string, message: UIMessage): Promise<{ id: string }> }
export interface ChatPersistencePort { findLast(conversationId: string): Promise<{ id: string, role: string } | undefined>, updateAssistant(messageId: string, parts: UIMessage['parts'], totalTokens?: number | null): Promise<void>, insertAssistant(conversationId: string, parts: UIMessage['parts'], totalTokens?: number | null): Promise<{ id: string }>, cacheTokens(conversationId: string, messageId: string, totalTokens: number): Promise<void> }
export interface ChatOwnershipPort { findConversation(userId: string, conversationId: string): Promise<ChatConversation | undefined>, findModel(userId: string, modelId: string): Promise<ChatModelRecord>, findProvider(userId: string, providerId: string): Promise<ChatProviderRecord>, findWorkspace(userId: string, workspaceId: string): Promise<{ name: string, path: string }> }
export interface LocalTerminalPort { hasPairedDevice(userId: string): Promise<boolean>, buildTool(): unknown }
export interface SubagentToolInput { userId: string, parentSessionId: string, authority: SubagentAuthority, model: ChatModelHandle, enabledToolIds: string[], approvals: Record<string, string>, permissionMode: ChatConversation['permissionMode'], abortSignal: AbortSignal }
export interface SubagentToolPort { build(input: SubagentToolInput): unknown, buildBackground(input: SubagentToolInput): Record<string, unknown>, buildOrchestration(input: SubagentToolInput): Record<string, unknown> }

export interface ChatTurnDependencies {
  ownership: ChatOwnershipPort
  history: ChatHistoryPort
  persistence: ChatPersistencePort
  localTerminal: LocalTerminalPort
  subagent: SubagentToolPort
  resolveWorkspacePath(path: string): Promise<string>
  resolveModelConfig(model: ChatModelContext): ResolvedChatModelConfig
  getChatModel(provider: ChatProviderContext, modelId: string): ChatModelHandle
  getLanggraphModel(provider: ChatProviderContext, modelId: string, maxOutputTokens?: number): ChatModelHandle
  resolveMessagesForModel(input: { messages: UIMessage[], conv: { id: string, contextSummary: string | null, contextSummaryUpToMessageId: string | null, lastMeasuredTokens: number | null, lastMeasuredMessageId: string | null }, contextWindow: number | null | undefined, maxOutputTokens: number | null | undefined, getSummarizerModel: () => ChatModelHandle }): Promise<UIMessage[]>
  buildMcpTools(userId: string, enabledToolIds: string[], approvals: Record<string, string>, permissionMode: ChatConversation['permissionMode']): Promise<{ tools: ChatToolSet, toolApproval?: ChatToolApproval, close: () => Promise<void> }>
  convertTurnMessages(messages: UIMessage[], tools: ChatToolSet): ChatMessageHandle
  prepareAiSdkModel(model: ChatModelHandle, thinkingEnabled: boolean): ChatModelHandle
  streamAiSdkAgent(input: Record<string, unknown>): ChatStreamResult
  streamLangGraphChat(input: Record<string, unknown>): ChatStreamResult
}
