/* eslint-disable @stylistic/max-statements-per-line */
import { badRequest, notFound } from '#server/core/errors/http'
import { inspectContext } from '../../../application/task-context-output'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event); const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')
  const conversation = await event.context.application.conversations.find(session.user.id, id)
  if (!conversation) throw notFound('Conversation not found')
  const [models, messages] = await Promise.all([
    event.context.application.models.list(session.user.id),
    event.context.application.conversations.listMessages(session.user.id, id)
  ])
  const model = (models as Array<{ id: string, contextWindow?: number | null, maxOutputTokens?: number | null }>).find(candidate => candidate.id === conversation.modelId)
  const latestMessageId = Array.isArray(messages) ? (messages.at(-1) as { id?: unknown } | undefined)?.id : undefined
  return inspectContext({
    contextWindow: model?.contextWindow ?? null,
    usedTokens: conversation.lastMeasuredTokens,
    measuredAtBoundary: Boolean(conversation.lastMeasuredMessageId && latestMessageId === conversation.lastMeasuredMessageId),
    maxOutputTokens: model?.maxOutputTokens ?? null,
    summary: conversation.contextSummary,
    summaryAgeMs: conversation.contextSummaryUpToCreatedAt ? Date.now() - new Date(conversation.contextSummaryUpToCreatedAt).getTime() : null
  })
})
