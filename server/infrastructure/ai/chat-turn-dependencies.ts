import { getChatModel, resolveModelConfig } from './providers/index'
import { getLanggraphModel } from './providers/langgraph-model'
import { resolveMessagesForModel } from './context-compaction'
import { buildMcpTools } from '../mcp/mcp-tools'
import { convertTurnMessages, prepareAiSdkModel, streamAiSdkAgent } from './ai-sdk-stream'
import { streamLangGraphChat } from './langgraph-stream'
import type { ChatTurnDependencies } from '../../application/chat/contracts'

/**
 * The narrow, explicit dependency contract `executeChatTurn` orchestrates
 * through (Plan 031A finding S) — provider/model resolution, context
 * compaction, MCP tool wiring, and AI SDK/LangGraph stream construction are
 * all concrete infrastructure integrations. `server/application/chat`
 * depends only on this interface's shape, never on the modules it is built
 * from here.
 */
/**
 * Composition-edge factory: builds the concrete dependency object that
 * `server/api/chat.post.ts` hands to `executeChatTurn`. Plain object of
 * functions — not a DI framework/container/service-locator.
 */
export function createChatTurnDependencies(): ChatTurnDependencies {
  return {
    resolveModelConfig: model => resolveModelConfig(model as Parameters<typeof resolveModelConfig>[0]),
    getChatModel: (provider, modelId) => getChatModel(provider as Parameters<typeof getChatModel>[0], modelId),
    getLanggraphModel: (provider, modelId, maxOutputTokens) => getLanggraphModel(provider as Parameters<typeof getLanggraphModel>[0], modelId, maxOutputTokens),
    resolveMessagesForModel: input => resolveMessagesForModel({ ...input, getSummarizerModel: input.getSummarizerModel as Parameters<typeof resolveMessagesForModel>[0]['getSummarizerModel'] }),
    buildMcpTools: async (userId, enabledToolIds, approvals) => buildMcpTools(userId, enabledToolIds, approvals as Parameters<typeof buildMcpTools>[2]),
    convertTurnMessages: (messages, tools) => convertTurnMessages(messages, tools as Parameters<typeof convertTurnMessages>[1]),
    prepareAiSdkModel: (model, thinkingEnabled) => prepareAiSdkModel(model as Parameters<typeof prepareAiSdkModel>[0], thinkingEnabled),
    streamAiSdkAgent: input => streamAiSdkAgent(input as Parameters<typeof streamAiSdkAgent>[0]),
    streamLangGraphChat: input => streamLangGraphChat(input as Parameters<typeof streamLangGraphChat>[0])
  }
}
