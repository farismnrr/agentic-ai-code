import { badRequest } from '#server/core/errors/http'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing provider ID')
  return event.context.application.providers.remove(session.user.id, id)
})
