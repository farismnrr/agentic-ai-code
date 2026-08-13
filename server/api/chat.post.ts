import { messages as messagesTable, conversations, workspaces, models, modelProviders, userDevices } from '../database/schema'
import { eq, and, gt, desc, asc, isNull } from 'drizzle-orm'
import { streamText, tool as aiTool, convertToModelMessages, stepCountIs, toUIMessageStream, wrapLanguageModel, extractReasoningMiddleware, createUIMessageStreamResponse, type ToolSet, type ToolApprovalConfiguration } from 'ai'
import type { UIMessage } from '#shared/types/chat'
import { getChatModel, resolveModelConfig } from '../utils/providers/index'
import { getLanggraphModel } from '../utils/providers/langgraph-model'
import { resolveMessagesForModel } from '../utils/context-compaction'
import { NATIVE_LOCAL_TERMINAL_TOOL_ID } from '#shared/utils/native-tools'
import { terminalToolSchema } from '@ai-code/terminal-tool'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const { message, trigger, id: conversationId } = await readBody(event)

  if (!conversationId) {
    throw badRequest('Missing conversationId')
  }

  const db = useDb()

  const [conv] = await db
    .select()
    .from(conversations)
    .where(and(eq(conversations.id, conversationId), eq(conversations.userId, session.user.id)))
    .limit(1)

  if (!conv) {
    throw notFound('Conversation not found')
  }

  const [modelInfo] = await db
    .select()
    .from(models)
    .where(eq(models.id, conv.modelId))
    .limit(1)

  if (!modelInfo) {
    throw notFound('Model not found')
  }

  const [provider] = await db
    .select()
    .from(modelProviders)
    .where(eq(modelProviders.id, modelInfo.providerId))
    .limit(1)

  if (!provider) {
    throw notFound('Model provider not found')
  }

  // Bound the query with the compaction cutoff (once one exists) instead of
  // fetching every message in the conversation on every single turn —
  // everything at/before the cutoff is already represented by
  // conv.contextSummary and gets discarded by resolveMessagesForModel
  // anyway. See server/utils/context-compaction.ts for where this cached
  // timestamp is written (alongside contextSummaryUpToMessageId, only on
  // an actual compaction event, not the per-turn hot path).
  const historyWhere = conv.contextSummaryUpToCreatedAt
    ? and(eq(messagesTable.conversationId, conv.id), gt(messagesTable.createdAt, conv.contextSummaryUpToCreatedAt))
    : eq(messagesTable.conversationId, conv.id)

  const dbRows = await db.select().from(messagesTable)
    .where(historyWhere)
    .orderBy(asc(messagesTable.createdAt))
  let messages: UIMessage[] = dbRows.map(r => ({ id: r.id, role: r.role as UIMessage['role'], parts: r.parts as UIMessage['parts'] }))

  if (trigger === 'submit-message' && message?.role === 'user') {
    const [inserted] = await db.insert(messagesTable)
      .values({ conversationId: conv.id, role: 'user', parts: message.parts })
      .returning({ id: messagesTable.id })
    if (!inserted) throw internal('Failed to insert user message')
    messages.push({ ...message, id: inserted.id })
  } else if (trigger === 'regenerate-message') {
    // drop the stale assistant answer being replaced — not history for this call
    if (messages.at(-1)?.role === 'assistant') messages = messages.slice(0, -1)
  } else {
    // tool-approval resume (and resume-stream, safe no-op if `message` is undefined):
    // swap in the client's freshly-updated version of the in-flight assistant
    // message — DB still has the pre-approval parts.
    if (message && messages.length > 0) messages[messages.length - 1] = message
  }

  const resolvedConfig = resolveModelConfig(modelInfo)

  const resolvedMessages = await resolveMessagesForModel({
    messages,
    conv,
    contextWindow: resolvedConfig.contextWindow,
    maxOutputTokens: resolvedConfig.maxOutputTokens,
    getSummarizerModel: () => getChatModel(provider, modelInfo.modelId)
  })

  let workspacePath: string | undefined
  let workspaceName: string | undefined
  if (conv.workspaceId) {
    const [workspace] = await db.select().from(workspaces).where(eq(workspaces.id, conv.workspaceId)).limit(1)
    if (workspace) {
      // `workspace.path` is stored relative to NUXT_WORKSPACES_ROOT (see
      // createWorkspace/updateWorkspace in server/utils/workspaces.ts) — it
      // is NOT an absolute, ready-to-use cwd. Re-resolve it through the same
      // fail-closed, symlink-aware jail check server/api/fs/browse.get.ts
      // already uses, rather than trusting the raw column value: passing
      // the relative string straight to execa's `cwd` would resolve against
      // the Nitro process's own cwd instead of the workspace root, silently
      // pointing the terminal tool at the wrong (or nonexistent) directory.
      try {
        workspacePath = await resolveWorkspacePath(workspace.path)
        workspaceName = workspace.name
      } catch (err) {
        logger.error('[chat] failed to resolve workspace path for terminal tool', err)
      }
    }
  }

  // The server itself no longer has any file/shell access to offer the
  // model — that tool (`native.terminal`, workspace-sandboxed, server-side)
  // was removed by deliberate decision; the only execution path left is
  // `local_terminal`, which runs on the user's own machine via their paired
  // relay-agent CLI (see plan 026) and is opt-in per conversation. This is
  // now just location context, not a capability description — the model
  // learns what tools it actually has (if any) from the tools/approvals the
  // SDK gives it directly, not from this prompt.
  const buildWorkspaceSystemPrompt = () => {
    if (!workspacePath) return undefined
    return `You are a coding assistant currently working in the workspace "${workspaceName}" located at ${workspacePath}.`
  }

  // Resolves conv.enabledToolIds (McpTool ids, `${serverId}.${toolName}`)
  // against the user's stored mcp_servers rows into real ai@7 tools, and
  // conv.approvals into streamText's toolApproval map — see plan 012 Phase 2
  // and .agents/memories/012-mcp-inbound-sse-transport.md's sibling decision
  // record for why this goes through the SDK's own tool-approval mechanism
  // instead of a hand-rolled one (.agents/memories/ai-sdk-native-features.md).
  let tools, toolApproval, close: () => Promise<void>

  if (conv.mode === 'agent') {
    const mcp = await buildMcpTools(session.user.id, conv.enabledToolIds, conv.approvals)
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
    let activeDevice: { id: string } | undefined
    try {
      [activeDevice] = await db.select({ id: userDevices.id })
        .from(userDevices)
        .where(and(eq(userDevices.userId, session.user.id), isNull(userDevices.revokedAt)))
        .limit(1)
    } catch (err) {
      logger.error('[chat] failed to check paired relay-agent devices', err)
    }

    if (activeDevice) {
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
      tools['local_terminal'] = aiTool({
        description: 'Execute a shell command on the user\'s own machine via their paired local CLI relay agent (a loopback bridge — this server never runs the command itself). Not scoped to any single project folder — pass an explicit `cwd` (absolute path) whenever the target directory matters, since it otherwise runs in the agent\'s own default directory, which may not be the folder the user means. Only available if the user has paired a device; if execution reports the agent is not connected, tell the user to open Settings → Local Terminal and pair it.',
        inputSchema: terminalToolSchema
      })
      toolApproval['local_terminal'] = async (_input: { command: string, args?: string[] }) => {
        const approval = conv.approvals?.[NATIVE_LOCAL_TERMINAL_TOOL_ID]
        return approval === 'always' ? 'approved' : approval === 'never' ? 'denied' : 'user-approval'
      }
    }
  } else {
    close = async () => {}
  }

  const abortController = new AbortController()
  event.node.req.on('close', () => abortController.abort())

  // Cached on `conversations` so the next turn's compaction budget check
  // (server/utils/context-compaction.ts) can read the last real usage
  // number off the already-loaded conversation row instead of issuing its
  // own `messages` query every turn.
  const cacheLastMeasuredTokens = async (messageId: string, totalTokens: number) => {
    await db.update(conversations)
      .set({ lastMeasuredTokens: totalTokens, lastMeasuredMessageId: messageId })
      .where(eq(conversations.id, conv.id))
  }

  const persistAssistantMessage = async (parts: UIMessage['parts'], isContinuation: boolean = false, totalTokens?: number | null) => {
    try {
      await close()

      // Diagnostic trail for provider-specific tool-call metadata (e.g.
      // Gemini 3's thoughtSignature, carried as callProviderMetadata) —
      // added after a real session showed the AI SDK's own "Replayed N
      // functionCall part(s) without a thoughtSignature" warning with no
      // way to tell, after the fact, whether the metadata was ever present
      // at the point we persisted it. Logs every turn, not just failures,
      // since the previous debugging session had to reconstruct this from
      // raw DB rows after the fact.
      const toolParts = parts.filter(p => String(p.type).startsWith('tool-'))
      if (toolParts.length > 0) {
        logger.info('[chat persist] assistant message with tool calls', {
          conversationId: conv.id,
          modelId: modelInfo.modelId,
          providerType: provider.type,
          isContinuation,
          toolCallCount: toolParts.length,
          toolCallsMissingProviderMetadata: toolParts.filter(p => !('callProviderMetadata' in p) && !('resultProviderMetadata' in p)).length
        })
      }

      if (isContinuation) {
        const [last] = await db
          .select()
          .from(messagesTable)
          .where(eq(messagesTable.conversationId, conv.id))
          .orderBy(desc(messagesTable.createdAt))
          .limit(1)

        if (last && last.role === 'assistant') {
          const updateData: { parts: UIMessage['parts'], totalTokens?: number } = { parts }
          if (totalTokens != null) updateData.totalTokens = totalTokens
          await db.update(messagesTable).set(updateData).where(eq(messagesTable.id, last.id))
          if (totalTokens != null) await cacheLastMeasuredTokens(last.id, totalTokens)
          return
        }
      }

      const [inserted] = await db.insert(messagesTable).values({
        conversationId: conv.id,
        role: 'assistant',
        parts,
        totalTokens
      }).returning({ id: messagesTable.id })

      if (totalTokens != null && inserted) await cacheLastMeasuredTokens(inserted.id, totalTokens)
    } catch (err) {
      logger.error('[chat onEnd] failed to persist assistant message', err)
    }
  }

  if (conv.mode === 'chat') {
    // Chat mode has no shell/file-access tool of its own (curl + search
    // only, see server/utils/langgraph-tools.ts) — the workspace-sandboxed
    // `terminal` tool it used to always wire in was removed; `local_terminal`
    // is agent-mode-only.
    const systemPrompt = buildWorkspaceSystemPrompt()
    const langgraphModel = getLanggraphModel(provider, modelInfo.modelId, resolvedConfig.maxOutputTokens)
    const uiStream = runLanggraphChat({
      uiMessages: resolvedMessages,
      baseModel: langgraphModel,
      systemPrompt,
      onEnd: async (parts, totalTokens) => {
        await persistAssistantMessage(parts, false, totalTokens)
      }
    })
    return createUIMessageStreamResponse({ stream: uiStream })
  }

  let baseModel = getChatModel(provider, modelInfo.modelId)

  if (resolvedConfig.thinkingEnabled) {
    baseModel = wrapLanguageModel({
      model: baseModel,
      middleware: extractReasoningMiddleware({ tagName: 'think' })
    })
  }

  const result = streamText({
    model: baseModel,
    system: buildWorkspaceSystemPrompt(),
    messages: await convertToModelMessages(resolvedMessages, { tools }),
    tools,
    // Cast once, here, at the boundary into the SDK — see the note on
    // `ToolApprovalValue` in server/utils/mcp-tools.ts for why `toolApproval`
    // is kept as a plain mutable Record everywhere before this point.
    toolApproval: toolApproval as ToolApprovalConfiguration<ToolSet, never> | undefined,
    // 5 was too low for a real terminal-backed edit flow (explore, read,
    // write, verify already eats 4-5 steps on its own) — it cut the loop
    // off exactly at the last tool call, leaving no budget for the model to
    // ever produce a closing text summary telling the user it was done.
    stopWhen: stepCountIs(20),
    // Without this, a hung model API call (as opposed to a hung terminal
    // command, which execa already caps at 30s) had nothing to time it out —
    // the request would wait indefinitely. `timeout` is streamText's own
    // native option (see .agents/memories/ai-sdk-native-features.md on
    // preferring SDK mechanisms over hand-rolled ones).
    timeout: { totalMs: 180_000, stepMs: 60_000 },
    maxOutputTokens: resolvedConfig.maxOutputTokens,
    abortSignal: abortController.signal,
    providerOptions: resolvedConfig.thinkingEnabled
      ? {
          [provider.type]: { reasoningEffort: conv.reasoningEffort ?? 'medium' }
        }
      : undefined,
    onError: ({ error }) => {
      logger.error('[chat stream]', error)
    }
  })

  const uiStream = toUIMessageStream({
    stream: result.stream,
    tools,
    originalMessages: messages,
    onEnd: async ({ responseMessage, isContinuation }) => {
      // toUIMessageStream only invokes onEnd from its underlying
      // TransformStream's flush()/cancel() hooks, which — unlike the
      // per-step onStepFinish callback — the `ai` SDK does not wrap in a
      // try/catch (see node_modules/ai/dist/index.js, handleUIMessageStreamFinish).
      // An unhandled throw here errors the response stream after the
      // client has already received every visible byte, so the browser
      // renders a complete answer while the DB write silently never
      // happens and nothing is logged anywhere. Catch and log explicitly
      // so a persistence failure is at least visible instead of invisible.
      let totalTokens: number | undefined
      try {
        // Not `result.usage` — that's `totalUsage`, the SUM of every step's
        // usage across this turn's whole multi-step tool-calling loop
        // (see node_modules/ai/dist/index.js:5946-5985's `steps.reduce(...)`).
        // A turn with several tool calls re-sends most of the context on
        // each step, so that sum inflates far past the conversation's real
        // size — confirmed against production data where it swung
        // 40k/199k/249k/563k across turns with no relation to actual
        // history growth. What compaction/the context-usage indicator need
        // is the size of the *last* call only — the one call whose input
        // reflects the full conversation as it stands now.
        const step = await result.finalStep
        if (step?.usage?.totalTokens) totalTokens = step.usage.totalTokens
      } catch {
        // ignore
      }
      await persistAssistantMessage(responseMessage.parts, isContinuation, totalTokens)
    }
  })

  return createUIMessageStreamResponse({ stream: uiStream })
})
