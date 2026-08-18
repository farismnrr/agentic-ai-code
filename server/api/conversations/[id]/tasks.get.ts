import { badRequest, notFound } from '#server/core/errors/http'
import { taskLedgerFor } from '../../../application/task-context-output'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')
  const conversation = await event.context.application.conversations.find(session.user.id, id)
  if (!conversation) throw notFound('Conversation not found')
  return taskLedgerFor(session.user.id, id, id)
})
