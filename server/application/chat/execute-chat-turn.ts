import type { ToolApprovalConfiguration, ToolSet } from 'ai'
import type { UIMessage } from '#shared/types/chat'
import { NATIVE_LOCAL_TERMINAL_TOOL_ID } from '#shared/utils/native-tools'
import { getChatModel, resolveModelConfig } from '../../utils/providers/index'
import { getLanggraphModel } from '../../utils/providers/langgraph-model'
import { resolveMessagesForModel } from '../../utils/context-compaction'
import { buildMcpTools } from '../../utils/mcp-tools'
import { convertTurnMessages, prepareAiSdkModel, streamAiSdkAgent } from '../../infrastructure/ai/ai-sdk-stream'
import { streamLangGraphChat } from '../../infrastructure/ai/langgraph-stream'
import { loadAuthorizedChatContext } from './ownership'
import { buildChatWorkspaceSystemPrompt, resolveChatWorkspaceContext } from './workspace-context'
import { buildTurnMessages } from './history'
import { createAssistantPersister } from './persistence'
import { createLocalTerminalPolicy } from './local-terminal-policy'

export interface ExecuteChatTurnInput {
  userId: string
  conversationId: string
  trigger: string | undefined
  message: UIMessage | undefined
  /**
   * Cancellation is received explicitly rather than derived from an H3
   * event — the caller (the HTTP adapter) is responsible for wiring this
   * to request-close/abort. This is what keeps `executeChatTurn` callable
   * with plain input and no H3 event object (Plan 031A finding H).
   */
  abortSignal: AbortSignal
}

/**
 * The single authoritative, H3-independent chat-turn use case (Plan 031A
 * finding H). Coordinates authorized context/history mutation/compaction,
 * workspace resolution, tool/approval composition, chat vs agent stream
 * selection, and assistant persistence/cleanup. `server/api/chat.post.ts`
 * is reduced to auth/input/event-cancellation/response adaptation around
 * this function.
 *
 * Depends only on narrow capability functions (ownership/workspace/history/
 * persistence/tool-policy from this same application layer, plus the
 * existing `server/utils/**` and `server/infrastructure/**` integration
 * boundaries) — no Drizzle schema, no H3 event, and no direct AI SDK/
 * LangGraph stream construction happens in this file itself.
 */
export async function executeChatTurn({ userId, conversationId, trigger, message, abortSignal }: ExecuteChatTurnInput) {
  const { conversation: conv, model: modelInfo, provider } = await loadAuthorizedChatContext(userId, conversationId)

  // Bound the query with the compaction cutoff (once one exists) instead of
  // fetching every message in the conversation on every single turn — see
  // server/utils/context-compaction.ts for where this cached timestamp is
  // written (alongside contextSummaryUpToMessageId, only on an actual
  // compaction event, not the per-turn hot path).
  const messages = await buildTurnMessages(conv, trigger, message)

  const resolvedConfig = resolveModelConfig(modelInfo)

  const resolvedMessages = await resolveMessagesForModel({
    messages,
    conv,
    contextWindow: resolvedConfig.contextWindow,
    maxOutputTokens: resolvedConfig.maxOutputTokens,
    getSummarizerModel: () => getChatModel(provider, modelInfo.modelId)
  })

  const { path: workspacePath, name: workspaceName } = await resolveChatWorkspaceContext(userId, conv.workspaceId)

  // The server itself no longer has any file/shell access to offer the
  // model — that tool (`native.terminal`, workspace-sandboxed, server-side)
  // was removed by deliberate decision; the only execution path left is
  // `local_terminal`, which runs on the user's own machine via their paired
  // relay-agent CLI (see plan 026) and is opt-in per conversation. This is
  // now just location context, not a capability description — the model
  // learns what tools it actually has (if any) from the tools/approvals the
  // SDK gives it directly, not from this prompt.
  const buildWorkspaceSystemPrompt = () => buildChatWorkspaceSystemPrompt(workspacePath, workspaceName)

  // Resolves conv.enabledToolIds (McpTool ids, `${serverId}.${toolName}`)
  // against the user's stored mcp_servers rows into real ai@7 tools, and
  // conv.approvals into streamText's toolApproval map — see plan 012 Phase 2
  // and .agents/memories/012-mcp-inbound-sse-transport.md's sibling decision
  // record for why this goes through the SDK's own tool-approval mechanism
  // instead of a hand-rolled one (.agents/memories/ai-sdk-native-features.md).
  // Plain map, not `ToolApprovalConfiguration<ToolSet, never>` — see
  // server/utils/mcp-tools.ts's comment on why key-by-key assignment
  // (`toolApproval['local_terminal'] = ...`, for tools mcp-tools.ts doesn't
  // know about) doesn't typecheck against that union. This is cast to
  // `ToolApprovalConfiguration` only at the `streamAiSdkAgent` call site,
  // once nothing further mutates it.
  type ToolApprovalValue = 'approved' | 'denied' | 'user-approval'
    | ((input: never) => 'approved' | 'denied' | 'user-approval' | Promise<'approved' | 'denied' | 'user-approval'>)

  let tools: ToolSet = {}
  let toolApproval: Record<string, ToolApprovalValue> | undefined
  let close: () => Promise<void> = async () => {}

  if (conv.mode === 'agent') {
    const mcp = await buildMcpTools(userId, conv.enabledToolIds, conv.approvals)
    tools = mcp.tools
    toolApproval = mcp.toolApproval
    close = mcp.close

    // Not gated by `conv.enabledToolIds` (no picker toggle for this one —
    // the Settings → Local Terminal page is already where a user manages
    // this, so a second on/off switch in the chat Tool Picker was
    // redundant). Instead: available in every agent-mode conversation the
    // moment the user has at least one non-revoked paired device, and never
    // otherwise — the per-call approval gate below is what actually decides
    // whether any given command runs, same as before. If the paired CLI
    // happens to be offline right now, the tool still shows up here (the
    // server has no way to know live connection state, only pairing
    // metadata) — the client-side error path in
    // app/composables/useConversationChat.ts's `runApprovedLocalTerminalCall`
    // already reports "not connected" back to the model in that case.
    //
    // Wrapped defensively — a real incident (missing migration, see plan
    // 026 Phase 9) had this exact query throw because `user_devices` didn't
    // exist yet, taking down the *entire* chat request (including MCP tools
    // that had nothing to do with this). A hiccup here should degrade to
    // "no local terminal this turn", not break agent mode outright.
    const localTerminalPolicy = await createLocalTerminalPolicy({ userId, approvals: conv.approvals, toolId: NATIVE_LOCAL_TERMINAL_TOOL_ID })
    if (localTerminalPolicy.paired) {
      // No `execute` here — this makes it a client-executed tool in the AI
      // SDK's own sense (see node_modules/ai/dist/index.js's onToolCall /
      // addToolOutput pair). Once approved, streamText has nothing to call
      // server-side, so it stops the step and streams the tool call to the
      // client as-is; app/composables/useConversationChat.ts's watcher on
      // `chat.messages` is what actually runs it (not `onToolCall` — see
      // that file's comments for why), over the loopback WebSocket to the
      // user's local relay-agent CLI. This server has no shell-execution
      // tool of its own at all (the old workspace-sandboxed `terminal` tool
      // was deliberately removed) — `local_terminal` is the only path, and
      // it never touches this server: the whole point of plan 026.
      tools['local_terminal'] = localTerminalPolicy.tool
      toolApproval = toolApproval ?? {}
      toolApproval['local_terminal'] = localTerminalPolicy.approval
    }
  }

  const assistantLifecycle = createAssistantPersister({ conversationId: conv.id, modelId: modelInfo.modelId, providerType: provider.type, close })
  const persistAssistantMessage = assistantLifecycle.persist

  if (conv.mode === 'chat') {
    // Chat mode has no shell/file-access tool of its own (curl + search
    // only, see server/utils/langgraph-tools.ts) — the workspace-sandboxed
    // `terminal` tool it used to always wire in was removed; `local_terminal`
    // is agent-mode-only.
    const systemPrompt = buildWorkspaceSystemPrompt()
    const langgraphModel = getLanggraphModel(provider, modelInfo.modelId, resolvedConfig.maxOutputTokens)
    return streamLangGraphChat({
      messages: resolvedMessages,
      model: langgraphModel,
      system: systemPrompt,
      abortSignal,
      cleanup: assistantLifecycle.cleanup,
      persistAssistantMessage
    })
  }

  const baseModel = prepareAiSdkModel(getChatModel(provider, modelInfo.modelId), resolvedConfig.thinkingEnabled)

  return streamAiSdkAgent({
    model: baseModel,
    system: buildWorkspaceSystemPrompt(),
    messages: await convertTurnMessages(resolvedMessages, tools),
    originalMessages: messages,
    tools,
    toolApproval: toolApproval as ToolApprovalConfiguration<ToolSet, never> | undefined,
    maxOutputTokens: resolvedConfig.maxOutputTokens,
    abortSignal,
    providerOptions: resolvedConfig.thinkingEnabled ? { [provider.type]: { reasoningEffort: conv.reasoningEffort ?? 'medium' } } : undefined,
    cleanup: assistantLifecycle.cleanup,
    persistAssistantMessage
  })
}
