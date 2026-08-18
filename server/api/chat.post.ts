import { executeChatTurn } from '../application/chat/execute-chat-turn'

/**
 * Thin transport adapter (Plan 031A finding H): authenticate, parse the
 * request body, wire request-close cancellation to an `AbortSignal`, build
 * the concrete provider/AI SDK/LangGraph/MCP dependency object at this
 * composition edge (Plan 031A finding S), hand off to the H3-independent
 * `executeChatTurn` application use case, and return whatever stream
 * response it built. No orchestration, persistence, or AI SDK/LangGraph/
 * provider construction happens in this file itself.
 */
export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const { message, trigger, id: conversationId, agentContext } = await readBody(event)

  const abortController = new AbortController()
  event.node.req.on('close', () => abortController.abort())

  return executeChatTurn({
    userId: session.user.id,
    conversationId,
    trigger,
    message,
    abortSignal: abortController.signal,
    agentContext: typeof agentContext === 'object' && agentContext !== null && !Array.isArray(agentContext)
      ? { repository_identity: typeof agentContext.repository_identity === 'string' ? agentContext.repository_identity.slice(0, 512) : undefined }
      : undefined,
    deps: event.context.application.chat(),
    telemetry: event.context.application.observability.request
  })
})
