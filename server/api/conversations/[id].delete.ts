export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing conversation ID')

  const deleted = await event.context.application.conversations.remove(session.user.id, id)

  if (!deleted) {
    throw notFound('Conversation not found')
  }

  return { ok: true }
})
