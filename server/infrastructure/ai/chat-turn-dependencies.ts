import { getChatModel, resolveModelConfig } from './providers/index'
import { getLanggraphModel } from './providers/langgraph-model'
import { resolveMessagesForModel } from './context-compaction'
import { buildMcpTools } from '../mcp/mcp-tools'
import { convertTurnMessages, prepareAiSdkModel, streamAiSdkAgent } from './ai-sdk-stream'
import { streamLangGraphChat } from './langgraph-stream'
import type { ChatTurnDependencies } from '../../application/chat/contracts'
import { findUserConversation, loadHistoryMessages, insertUserMessage, findLastMessage, updateAssistantMessage, insertAssistantMessage, cacheLastMeasuredTokens } from '../database/chat'
import { findUserModel } from '../database/models'
import { findUserProvider } from '../database/providers'
import { findUserWorkspace } from '../../infrastructure/database/workspaces'
import { resolveWorkspacePath } from '../filesystem/browse'
import { hasActivePairedDevice } from '../database/devices'
import { buildLocalTerminalTool } from './local-terminal-tool'
import { buildBackgroundTaskTools, buildOrchestratorTools, buildSubagentTool } from './subagent-tool'

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
    ownership: { findConversation: async (userId, id) => findUserConversation(userId, id) as never, findModel: async (userId, id) => findUserModel(userId, id) as never, findProvider: async (userId, id) => findUserProvider(userId, id) as never, findWorkspace: async (userId, id) => findUserWorkspace(userId, id) as never },
    history: { load: async conversation => loadHistoryMessages(conversation as never), insertUser: insertUserMessage },
    persistence: { findLast: async id => findLastMessage(id), updateAssistant: updateAssistantMessage, insertAssistant: insertAssistantMessage, cacheTokens: cacheLastMeasuredTokens },
    localTerminal: { hasPairedDevice: hasActivePairedDevice, buildTool: buildLocalTerminalTool },
    subagent: { build: input => buildSubagentTool(input), buildBackground: input => buildBackgroundTaskTools(input), buildOrchestration: input => buildOrchestratorTools(input) },
    resolveWorkspacePath,
    resolveModelConfig: model => resolveModelConfig(model as Parameters<typeof resolveModelConfig>[0]),
    getChatModel: (provider, modelId) => getChatModel(provider as Parameters<typeof getChatModel>[0], modelId),
    getLanggraphModel: (provider, modelId, maxOutputTokens) => getLanggraphModel(provider as Parameters<typeof getLanggraphModel>[0], modelId, maxOutputTokens),
    resolveMessagesForModel: input => resolveMessagesForModel({ ...input, getSummarizerModel: input.getSummarizerModel as Parameters<typeof resolveMessagesForModel>[0]['getSummarizerModel'] }),
    buildMcpTools: async (userId, enabledToolIds, approvals, permissionMode) => buildMcpTools(userId, enabledToolIds, approvals as Parameters<typeof buildMcpTools>[2], permissionMode as Parameters<typeof buildMcpTools>[3]),
    convertTurnMessages: (messages, tools) => convertTurnMessages(messages, tools as Parameters<typeof convertTurnMessages>[1]),
    prepareAiSdkModel: (model, thinkingEnabled) => prepareAiSdkModel(model as Parameters<typeof prepareAiSdkModel>[0], thinkingEnabled),
    streamAiSdkAgent: input => streamAiSdkAgent(input as Parameters<typeof streamAiSdkAgent>[0]),
    streamLangGraphChat: input => streamLangGraphChat(input as Parameters<typeof streamLangGraphChat>[0])
  }
}
