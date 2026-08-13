import type { UIMessage } from '#shared/types/chat'
import { getChatModel, resolveModelConfig } from './providers/index'
import { getLanggraphModel } from './providers/langgraph-model'
import { resolveMessagesForModel } from './context-compaction'
import { buildMcpTools } from '../mcp/mcp-tools'
import { convertTurnMessages, prepareAiSdkModel, streamAiSdkAgent } from './ai-sdk-stream'
import { streamLangGraphChat } from './langgraph-stream'

/**
 * The narrow, explicit dependency contract `executeChatTurn` orchestrates
 * through (Plan 031A finding S) — provider/model resolution, context
 * compaction, MCP tool wiring, and AI SDK/LangGraph stream construction are
 * all concrete infrastructure integrations. `server/application/chat`
 * depends only on this interface's shape, never on the modules it is built
 * from here.
 */
export interface ChatTurnDependencies {
  resolveModelConfig: typeof resolveModelConfig
  getChatModel: typeof getChatModel
  getLanggraphModel: typeof getLanggraphModel
  resolveMessagesForModel: typeof resolveMessagesForModel
  buildMcpTools: typeof buildMcpTools
  convertTurnMessages: (messages: UIMessage[], tools: Parameters<typeof convertTurnMessages>[1]) => ReturnType<typeof convertTurnMessages>
  prepareAiSdkModel: typeof prepareAiSdkModel
  streamAiSdkAgent: typeof streamAiSdkAgent
  streamLangGraphChat: typeof streamLangGraphChat
}

/**
 * Composition-edge factory: builds the concrete dependency object that
 * `server/api/chat.post.ts` hands to `executeChatTurn`. Plain object of
 * functions — not a DI framework/container/service-locator.
 */
export function createChatTurnDependencies(): ChatTurnDependencies {
  return {
    resolveModelConfig,
    getChatModel,
    getLanggraphModel,
    resolveMessagesForModel,
    buildMcpTools,
    convertTurnMessages,
    prepareAiSdkModel,
    streamAiSdkAgent,
    streamLangGraphChat
  }
}
