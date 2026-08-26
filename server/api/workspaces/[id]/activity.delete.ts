import { badRequest } from '#server/core/errors/http'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const workspaceId = getRouterParam(event, 'id')
  if (!workspaceId) throw badRequest('Missing workspace id')
  await event.context.application.activity.clear(session.user.id, workspaceId)
  return { cleared: true }
})
