import { messages as messagesTable, conversations } from '../database/schema'
import { eq, and, desc } from 'drizzle-orm'
import { streamText, convertToModelMessages, stepCountIs, toUIMessageStream, wrapLanguageModel, extractReasoningMiddleware, createUIMessageStreamResponse } from 'ai'
import type { UIMessage } from '#shared/types/chat'
import { models } from '#shared/utils/models'

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
    const uiStream = runLanggraphChat(messages as UIMessage[], conv.modelId || 'vx/gemini-3-flash-preview', async (parts) => {
      await persistAssistantMessage(parts, false)
    })
    return createUIMessageStreamResponse({ stream: uiStream })
  }

  const modelInfo = models.find(m => m.id === (conv.modelId || 'vx/gemini-3-flash-preview'))
  let baseModel = getRouterModel(conv.modelId || 'vx/gemini-3-flash-preview')

  if (modelInfo?.supportsReasoning) {
    baseModel = wrapLanguageModel({
      model: baseModel,
      middleware: extractReasoningMiddleware({ tagName: 'think' })
    })
  }

  const result = streamText({
    model: baseModel,
    messages: await convertToModelMessages(messages as UIMessage[], { tools }),
    tools,
    toolApproval,
    stopWhen: stepCountIs(5),
    abortSignal: abortController.signal,
    providerOptions: modelInfo?.supportsReasoning
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
