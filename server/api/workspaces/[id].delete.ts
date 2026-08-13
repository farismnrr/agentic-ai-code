import { badRequest } from '#server/core/errors/http'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Missing workspace id')

  return event.context.application.workspaces.remove(session.user.id, id)
})
