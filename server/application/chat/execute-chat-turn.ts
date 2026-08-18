import type { UIMessage } from '#shared/types/chat'
import { NATIVE_LOCAL_TERMINAL_TOOL_ID } from '#shared/utils/native-tools'
import type { ChatTurnDependencies } from './contracts'
import type { RequestTelemetryContext } from '../observability/contracts'
import { loadAuthorizedChatContext } from './ownership'
import { buildChatWorkspaceSystemPrompt, resolveChatWorkspaceContext } from './workspace-context'
import { buildTurnMessages } from './history'
import { createAssistantPersister } from './persistence'
import { createLocalTerminalPolicy } from './local-terminal-policy'
import { buildTaskUpdateTool } from '../task-context-output'

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
  /**
   * Concrete provider/AI SDK/LangGraph/MCP/context-compaction integrations,
   * supplied by the composition edge (`server/api/chat.post.ts` via
   * `createChatTurnDependencies()`) rather than imported directly here
   * (Plan 031A finding S) — this file depends only on the narrow
   * `ChatTurnDependencies` contract, never on `server/infrastructure/**`
   * modules themselves.
   */
  deps: ChatTurnDependencies
  /** Request-scoped telemetry context (Plan 035 Phase 6). Optional so this use case stays callable without a live request (tests/tools). */
  telemetry?: RequestTelemetryContext
  /** Bounded structured context returned by the first-party relay session hook. */
  agentContext?: { repository_identity?: string }
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
 * persistence/tool-policy from this same application layer) plus the
 * explicit `ChatTurnDependencies` contract for provider/AI SDK/LangGraph/
 * MCP/context-compaction integrations (Plan 031A finding S) — no Drizzle
 * schema, no H3 event, and no direct `server/infrastructure/**` import
 * happens in this file itself; the composition edge
 * (`server/api/chat.post.ts`) supplies the concrete `deps` object via
 * `createChatTurnDependencies()`.
 */
export async function executeChatTurn(input: ExecuteChatTurnInput) {
  const { telemetry } = input
  if (!telemetry) return executeChatTurnInner(input)
  return telemetry.withSpan('chat.execute', {}, () => executeChatTurnInner(input))
}

async function executeChatTurnInner({ userId, conversationId, trigger, message, abortSignal, deps, telemetry, agentContext }: ExecuteChatTurnInput) {
  const { conversation: conv, model: modelInfo, provider } = await loadAuthorizedChatContext(userId, conversationId, deps.ownership)

  // Bound the query with the compaction cutoff (once one exists) instead of
  // fetching every message in the conversation on every single turn — see
  // server/infrastructure/ai/context-compaction.ts for where this cached timestamp is
  // written (alongside contextSummaryUpToMessageId, only on an actual
  // compaction event, not the per-turn hot path).
  const messages = await buildTurnMessages(conv, trigger, message, deps.history)

  const resolvedConfig = deps.resolveModelConfig(modelInfo)

  const resolvedMessages = await deps.resolveMessagesForModel({
    messages,
    conv,
    contextWindow: resolvedConfig.contextWindow,
    maxOutputTokens: resolvedConfig.maxOutputTokens,
    getSummarizerModel: () => deps.getChatModel(provider, modelInfo.modelId)
  })

  const { path: workspacePath, name: workspaceName } = await resolveChatWorkspaceContext(userId, conv.workspaceId, deps.ownership, deps.resolveWorkspacePath, telemetry)

  // The server itself no longer has any file/shell access to offer the
  // model — that tool (`native.terminal`, workspace-sandboxed, server-side)
  // was removed by deliberate decision; the only execution path left is
  // `local_terminal`, which runs on the user's own machine via their paired
  // relay-agent CLI (see plan 026) and is opt-in per conversation. This is
  // now just location context, not a capability description — the model
  // learns what tools it actually has (if any) from the tools/approvals the
  // SDK gives it directly, not from this prompt.
  const boundedAgentContext = agentContext?.repository_identity
    ? { repository_identity: agentContext.repository_identity.slice(0, 512) }
    : undefined
  const buildWorkspaceSystemPrompt = () => {
    const workspacePrompt = buildChatWorkspaceSystemPrompt(workspacePath, workspaceName)
    if (!boundedAgentContext || conv.mode !== 'agent') return workspacePrompt
    const contextPrompt = `Bounded repository hook context: ${JSON.stringify(boundedAgentContext)}`
    return workspacePrompt ? `${workspacePrompt}\n${contextPrompt}` : contextPrompt
  }

  // Resolves conv.enabledToolIds (McpTool ids, `${serverId}.${toolName}`)
  // against the user's stored mcp_servers rows into real ai@7 tools, and
  // conv.approvals into streamText's toolApproval map — see plan 012 Phase 2
  // and .agents/memories/012-mcp-inbound-sse-transport.md's sibling decision
  // record for why this goes through the SDK's own tool-approval mechanism
  // instead of a hand-rolled one (.agents/memories/ai-sdk-native-features.md).
  // Plain application-level maps; the infrastructure adapter translates these
  // into SDK-specific tool and approval structures.
  let tools: Record<string, unknown> = {}
  let toolApproval: Record<string, unknown> | undefined
  let close: () => Promise<void> = async () => {}

  if (conv.mode === 'agent') {
    tools['task_update'] = buildTaskUpdateTool({ userId, conversationId: conv.id })
    const mcp = await deps.buildMcpTools(userId, conv.enabledToolIds, conv.approvals, conv.permissionMode)
    tools = mcp.tools
    toolApproval = mcp.toolApproval
    close = mcp.close
    if (conv.enabledToolIds.length > 0) telemetry?.event('chat.tool.mcp.dispatch', 'ok')

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
    const localTerminalPolicy = await createLocalTerminalPolicy({ userId, approvals: conv.approvals as Record<string, 'always' | 'never'>, toolId: NATIVE_LOCAL_TERMINAL_TOOL_ID, permissionMode: conv.permissionMode, localTerminal: deps.localTerminal, telemetry })
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
      telemetry?.event('chat.tool.local_terminal.dispatch', 'ok')
    }
    if (workspacePath) {
      tools['delegate_task'] = deps.subagent.build({
        userId,
        parentSessionId: conv.id,
        authority: {
          tools: conv.enabledToolIds,
          effects: conv.permissionMode === 'plan' ? ['workspace_read', 'git_read'] : ['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation'],
          working_mode: conv.permissionMode === 'plan' ? 'read-only' : 'workspace',
          model_policy: 'default',
          workspace_root: workspacePath
        },
        model: deps.getChatModel(provider, modelInfo.modelId),
        enabledToolIds: conv.enabledToolIds,
        approvals: conv.approvals,
        permissionMode: conv.permissionMode,
        abortSignal
      })
      telemetry?.event('chat.subagent.dispatch', 'ok')
      Object.assign(tools, deps.subagent.buildBackground({ userId, parentSessionId: conv.id, authority: { tools: conv.enabledToolIds, effects: conv.permissionMode === 'plan' ? ['workspace_read', 'git_read'] : ['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation'], working_mode: conv.permissionMode === 'plan' ? 'read-only' : 'workspace', model_policy: 'default', workspace_root: workspacePath }, model: deps.getChatModel(provider, modelInfo.modelId), enabledToolIds: conv.enabledToolIds, approvals: conv.approvals, permissionMode: conv.permissionMode, abortSignal }))
    }
  }

  const assistantLifecycle = createAssistantPersister({ conversationId: conv.id, modelId: modelInfo.modelId, providerType: provider.type, close, persistence: deps.persistence, telemetry })
  const persistAssistantMessage = assistantLifecycle.persist

  telemetry?.event('chat.stream.start', 'ok', { 'provider.type': provider.type })
  if (abortSignal.aborted) {
    telemetry?.event('chat.stream.abort', 'cancelled', { 'provider.type': provider.type })
  } else {
    abortSignal.addEventListener('abort', () => telemetry?.event('chat.stream.abort', 'cancelled', { 'provider.type': provider.type }), { once: true })
  }

  if (conv.mode === 'chat') {
    // Chat mode has no shell/file-access tool of its own (curl + search
    // only, see server/infrastructure/ai/langgraph-tools.ts) — the workspace-sandboxed
    // `terminal` tool it used to always wire in was removed; `local_terminal`
    // is agent-mode-only.
    const systemPrompt = buildWorkspaceSystemPrompt()
    const langgraphModel = deps.getLanggraphModel(provider, modelInfo.modelId, resolvedConfig.maxOutputTokens)
    return deps.streamLangGraphChat({
      messages: resolvedMessages,
      model: langgraphModel,
      system: systemPrompt,
      abortSignal,
      cleanup: assistantLifecycle.cleanup,
      persistAssistantMessage,
      telemetry
    })
  }

  const baseModel = deps.prepareAiSdkModel(deps.getChatModel(provider, modelInfo.modelId), resolvedConfig.thinkingEnabled)

  return deps.streamAiSdkAgent({
    model: baseModel,
    system: buildWorkspaceSystemPrompt(),
    messages: await deps.convertTurnMessages(resolvedMessages, tools),
    originalMessages: messages,
    tools,
    toolApproval,
    maxOutputTokens: resolvedConfig.maxOutputTokens,
    abortSignal,
    providerOptions: resolvedConfig.thinkingEnabled ? { [provider.type]: { reasoningEffort: conv.reasoningEffort ?? 'medium' } } : undefined,
    cleanup: assistantLifecycle.cleanup,
    persistAssistantMessage,
    telemetry
  })
}
