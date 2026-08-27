import type { UIMessage } from '#shared/types/chat'
import { NATIVE_LOCAL_TERMINAL_TOOL_ID, isNativeToolId } from '#shared/utils/native-tools'
import type { ChatTurnDependencies, SubagentToolInput } from './contracts'
import type { RequestTelemetryContext } from '../observability/contracts'
import { loadAuthorizedChatContext } from './ownership'
import { buildChatWorkspaceSystemPrompt, resolveChatWorkspaceContext } from './workspace-context'
import { buildTurnMessages } from './history'
import { createAssistantPersister } from './persistence'
import { createLocalTerminalPolicy } from './local-terminal-policy'
import { buildTaskUpdateTool } from '../task-context-output'
import { buildOrchestratorPlanTool } from '../orchestration/tool'
import { composeAgentTools } from './tool-composition'

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
  const localTerminalEnabled = conv.enabledToolIds.includes(NATIVE_LOCAL_TERMINAL_TOOL_ID)
  const agentTurn = conv.mode === 'agent' && localTerminalEnabled
  const enabledMcpToolIds = conv.enabledToolIds.filter(toolId => !isNativeToolId(toolId))

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
    const prompts = [workspacePrompt]
    if (agentTurn && conv.permissionMode === 'plan') {
      prompts.push('Plan mode is active. Analyze and produce a concrete implementation plan only. Do not make changes or request mutating capabilities.')
    }
    if (boundedAgentContext && agentTurn) prompts.push(`Bounded repository hook context: ${JSON.stringify(boundedAgentContext)}`)
    return prompts.filter(Boolean).join('\n')
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

  if (agentTurn) {
    const internalTools = {
      task_update: buildTaskUpdateTool({ userId, conversationId: conv.id }),
      orchestrator_plan: buildOrchestratorPlanTool({ userId, conversationId: conv.id, parentSessionId: conv.id })
    }
    const mcp = await deps.buildMcpTools(userId, enabledMcpToolIds, conv.approvals, conv.permissionMode)
    tools = composeAgentTools(internalTools, mcp.tools)
    toolApproval = mcp.toolApproval
    close = mcp.close
    if (enabledMcpToolIds.length > 0) telemetry?.event('chat.tool.mcp.dispatch', 'ok')

    // Terminal relay is an explicit per-conversation capability. The client
    // additionally requires a live loopback connection before it exposes Agent
    // Mode; the server enforces the persisted enablement side.
    const localTerminalPolicy = createLocalTerminalPolicy({ approvals: conv.approvals as Record<string, 'always' | 'never'>, toolId: NATIVE_LOCAL_TERMINAL_TOOL_ID, permissionMode: conv.permissionMode, localTerminal: deps.localTerminal })
    // No `execute` here — this is a client-executed AI SDK tool. The browser
    // runs an approved call through its loopback relay; this server never
    // executes the shell command itself.
    tools['local_terminal'] = localTerminalPolicy.tool
    toolApproval = toolApproval ?? {}
    toolApproval['local_terminal'] = localTerminalPolicy.approval
    telemetry?.event('chat.tool.local_terminal.dispatch', 'ok')
    if (workspacePath) {
      const subagentInput: SubagentToolInput = {
        userId,
        parentSessionId: conv.id,
        authority: {
          tools: enabledMcpToolIds,
          effects: conv.permissionMode === 'plan' ? ['workspace_read', 'git_read'] : ['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation'],
          working_mode: conv.permissionMode === 'plan' ? 'read-only' : 'workspace',
          model_policy: 'default',
          workspace_root: workspacePath
        },
        model: deps.getChatModel(provider, modelInfo.modelId),
        enabledToolIds: enabledMcpToolIds,
        approvals: conv.approvals,
        permissionMode: conv.permissionMode,
        abortSignal
      }
      tools['delegate_task'] = deps.subagent.build(subagentInput)
      telemetry?.event('chat.subagent.dispatch', 'ok')
      Object.assign(tools, deps.subagent.buildBackground(subagentInput))
      Object.assign(tools, deps.subagent.buildOrchestration(subagentInput))
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

  if (!agentTurn) {
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
