import { messages as messagesTable, conversations, workspaces } from '../database/schema'
import { eq, and, desc } from 'drizzle-orm'
import { streamText, convertToModelMessages, stepCountIs, toUIMessageStream, wrapLanguageModel, extractReasoningMiddleware, createUIMessageStreamResponse } from 'ai'
import type { UIMessage } from '#shared/types/chat'
import { getChatModel, resolveModelConfig } from '../utils/providers/index'
import { getLanggraphModel } from '../utils/providers/langgraph-model'
import { getSettings } from '../utils/settings'
import { NATIVE_TERMINAL_TOOL_ID } from '#shared/utils/native-tools'
import { createTerminalAiTool } from '@ai-code/terminal-tool'
import { assertSafeCommand, isReadOnlyCommand } from '../utils/exec-guard'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const { messages, id: conversationId } = await readBody(event)

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

  const settings = await getSettings(session.user.id)
  const resolvedConfig = resolveModelConfig(modelInfo, settings)

  if (!conv) {
    throw notFound('Conversation not found')
  }

  // Resuming a tool-approval response re-sends the same in-flight assistant
  // message with an appended approval part, not a new user message — so this
  // only fires on an actual new turn, not every request.
  const lastMsg = messages[messages.length - 1]
  if (lastMsg && lastMsg.role === 'user') {
    await db.insert(messagesTable).values({
      conversationId: conv.id,
      role: 'user',
      parts: lastMsg.parts
    })
  }

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
        console.error('[chat] failed to resolve workspace path for terminal tool', err)
      }
    }
  }

  // Without this, the model has no idea a workspace/terminal tool exists at
  // all and falls back to asking the user to paste files — it never learns
  // to explore proactively just from the tool's own (generic) description.
  //
  // 'full' access additionally warns against editing blind: a real incident
  // had the model run `sed -i` on a file based on its contents from several
  // turns earlier in the conversation, never re-reading it in the same turn
  // and never checking the edit actually applied — a write tool with no
  // read-before-write discipline is a data-loss risk, not just a UX one.
  const buildWorkspaceSystemPrompt = (terminalAccess: 'none' | 'read-only' | 'full') => {
    if (!workspacePath) return undefined
    const base = `You are a coding assistant currently working in the workspace "${workspaceName}" located at ${workspacePath}.`
    // A conversation can reach here with a real workspace but zero enabled
    // tools (e.g. agent mode started fresh with nothing toggled on yet) —
    // without being told that plainly, a model asked to read/edit a file has
    // fabricated a plausible-sounding "I found the file and edited it"
    // narrative instead of saying it has no way to do that. Never let silent
    // tool absence read as an invitation to make something up.
    if (terminalAccess === 'none') return `${base} You do NOT have any tool to read, search, or modify files in this conversation right now. If asked to do any of that, say plainly that you don't have that capability here (the user can enable the terminal tool via the Tools picker) — never claim to have looked at, found, or changed a file you have no way to access.`
    const exploreGuidance = `You have access to a \`terminal\` tool scoped to this workspace directory — use it proactively (e.g. \`tree\`, \`find\`, \`grep\`/\`rg\`, \`cat\`, \`sed -n\`) to explore, read, or search files when the user asks about their project, rather than asking them to paste code or links.`
    if (terminalAccess === 'read-only') return `${base} ${exploreGuidance}`
    return `${base} ${exploreGuidance} This terminal has full write access (not read-only) — never edit a file whose exact path you have not confirmed with \`find\`/\`tree\`/\`grep\` in this same turn, even if a filename was mentioned earlier in the conversation; a remembered name or path can be wrong, stale, or entirely made up, and guessing has produced edits to files that don't exist. Before editing or overwriting any file, always re-read its current contents first with \`cat\`/\`sed -n\` in this same turn, even if you already saw it earlier in the conversation, since it may have changed since. Never assume a file's contents or line numbers from memory. After making a change, read the file back to confirm it applied correctly before telling the user it's done. Every new user request that asks you to read, search, or change something requires actually calling the terminal tool in this turn, even if an earlier turn already did something similar — describing a past action instead of performing the current request (e.g. restating an old edit when asked for a different one) is exactly the kind of fabrication this applies to, and has happened for real.`
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

    if (conv.enabledToolIds?.includes(NATIVE_TERMINAL_TOOL_ID) && workspacePath) {
      tools['terminal'] = createTerminalAiTool({
        cwd: workspacePath,
        assertSafeCommand: (c, a) => assertSafeCommand(c, a, 'full')
      })
      // Per-call, not per-tool: a read-only command (ls/cat/find/...) runs
      // immediately regardless of the user's remembered decision — only a
      // command capable of mutating something (anything outside the
      // read-only allowlist, including any `bash`/`sh` invocation, which
      // can't be statically judged safe) is gated behind approval.
      toolApproval['terminal'] = async (input: { command: string, args?: string[] }) => {
        if (await isReadOnlyCommand(input.command, input.args ?? [])) return 'approved'
        const approval = conv.approvals?.[NATIVE_TERMINAL_TOOL_ID]
        return approval === 'always' ? 'approved' : approval === 'never' ? 'denied' : 'user-approval'
      }
    }
  } else {
    close = async () => {}
  }

  const abortController = new AbortController()
  event.node.req.on('close', () => abortController.abort())

  const persistAssistantMessage = async (parts: UIMessage['parts'], isContinuation: boolean = false) => {
    try {
      await close()

      if (isContinuation) {
        const [last] = await db
          .select()
          .from(messagesTable)
          .where(eq(messagesTable.conversationId, conv.id))
          .orderBy(desc(messagesTable.createdAt))
          .limit(1)

        if (last && last.role === 'assistant') {
          await db.update(messagesTable).set({ parts }).where(eq(messagesTable.id, last.id))
          return
        }
      }

      await db.insert(messagesTable).values({
        conversationId: conv.id,
        role: 'assistant',
        parts
      })
    } catch (err) {
      console.error('[chat onEnd] failed to persist assistant message', err)
    }
  }

  if (conv.mode === 'chat') {
    // Chat mode always wires the read-only terminal tool in when a
    // workspace is resolved (see server/utils/langgraph-tools.ts).
    const systemPrompt = buildWorkspaceSystemPrompt(workspacePath ? 'read-only' : 'none')
    const langgraphModel = getLanggraphModel(provider, modelInfo.modelId)
    const uiStream = runLanggraphChat({
      uiMessages: messages as UIMessage[],
      baseModel: langgraphModel,
      workspacePath,
      systemPrompt,
      onEnd: async (parts) => {
        await persistAssistantMessage(parts, false)
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
    system: buildWorkspaceSystemPrompt(tools?.terminal ? 'full' : 'none'),
    messages: await convertToModelMessages(messages as UIMessage[], { tools }),
    tools,
    toolApproval,
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
    abortSignal: abortController.signal,
    providerOptions: resolvedConfig.thinkingEnabled
      ? {
          '9router': { reasoningEffort: conv.reasoningEffort ?? 'medium' }
        }
      : undefined,
    onError: ({ error }) => {
      console.error('[chat stream]', error)
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
      await persistAssistantMessage(responseMessage.parts, isContinuation)
    }
  })

  return createUIMessageStreamResponse({ stream: uiStream })
})
