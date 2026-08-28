import type { UIMessage } from '#shared/types/chat'
import type { SubagentEffect } from '#shared/types/subagents'
import type { ChatTurnDependencies, SubagentToolInput } from './contracts'
import type { RequestTelemetryContext } from '../observability/contracts'
import { loadAuthorizedChatContext } from './ownership'
import { buildChatWorkspaceSystemPrompt, resolveChatWorkspaceContext } from './workspace-context'
import { buildTurnMessages } from './history'
import { createAssistantPersister } from './persistence'
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

async function executeChatTurnInner({ userId, conversationId, trigger, message, abortSignal, deps, telemetry }: ExecuteChatTurnInput) {
  const { conversation: conv, model: modelInfo, provider } = await loadAuthorizedChatContext(userId, conversationId, deps.ownership)
  const mcpExecution = await deps.resolveMcpExecutionContext(userId)
  const enabledMcpToolIds = mcpExecution.enabledToolIds
  const agentTurn = conv.mode === 'agent' && mcpExecution.terminalAvailable
  const readOnlyToolTurn = conv.mode === 'chat' && mcpExecution.terminalAvailable
  const toolTurn = agentTurn || readOnlyToolTurn

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

  const buildWorkspaceSystemPrompt = () => {
    const workspacePrompt = buildChatWorkspaceSystemPrompt(workspacePath, workspaceName)
    const prompts = [workspacePrompt]
    if (agentTurn && conv.permissionMode === 'plan') {
      prompts.push('Plan mode is active. Analyze and produce a concrete implementation plan only. Do not make changes or request mutating capabilities.')
    }
    if (readOnlyToolTurn) {
      prompts.push('Chat mode is read-only. You may inspect the workspace with the provided structured read capabilities, but you must not make changes or request mutating capabilities.')
    }
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

  if (toolTurn) {
    const effectivePermissionMode = agentTurn ? conv.permissionMode : 'plan'
    const allowedEffects: SubagentEffect[] = agentTurn && conv.permissionMode !== 'plan'
      ? ['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge']
      : ['workspace_read', 'git_read']
    const mcp = await deps.buildMcpTools(userId, enabledMcpToolIds, conv.approvals, effectivePermissionMode, { allowedEffects, abortSignal })
    tools = mcp.tools
    toolApproval = mcp.toolApproval
    close = mcp.close
    if (enabledMcpToolIds.length > 0) telemetry?.event('chat.tool.mcp.dispatch', 'ok', { 'chat.mode': agentTurn ? 'agent' : 'chat' })

    if (agentTurn) {
      const internalTools = {
        task_update: buildTaskUpdateTool({ userId, conversationId: conv.id }),
        orchestrator_plan: buildOrchestratorPlanTool({ userId, conversationId: conv.id, parentSessionId: conv.id })
      }
      tools = composeAgentTools(internalTools, tools)

      if (workspacePath) {
        const subagentInput: SubagentToolInput = {
          userId,
          parentSessionId: conv.id,
          workspaceName: workspaceName ?? '',
          authority: {
            tools: enabledMcpToolIds,
            effects: conv.permissionMode === 'plan' ? ['workspace_read', 'git_read'] : allowedEffects,
            working_mode: conv.permissionMode === 'plan' ? 'read-only' : 'workspace',
            model_policy: 'default',
            workspace_root: workspacePath
          },
          model: deps.getChatModel(provider, modelInfo.modelId),
          enabledToolIds: enabledMcpToolIds,
          approvals: conv.approvals,
          permissionMode: conv.permissionMode,
          abortSignal,
          taskNotifications: deps.taskNotifications
        }
        tools['delegate_task'] = deps.subagent.build(subagentInput)
        telemetry?.event('chat.subagent.dispatch', 'ok')
        Object.assign(tools, deps.subagent.buildBackground(subagentInput))
        Object.assign(tools, deps.subagent.buildOrchestration(subagentInput))
      }
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

  if (!toolTurn) {
    // Without the configured terminal relay, Chat remains a plain model chat.
    // Persisted Agent state is intentionally ignored here so a stale
    // conversation can never retain write authority after terminal loss.
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
