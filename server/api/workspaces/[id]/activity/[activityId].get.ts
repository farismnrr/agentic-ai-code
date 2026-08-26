import { badRequest } from '#server/core/errors/http'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const workspaceId = getRouterParam(event, 'id')
  const activityId = getRouterParam(event, 'activityId')
  if (!workspaceId || !activityId) throw badRequest('Missing activity identity')
  setResponseHeader(event, 'Cache-Control', 'private, no-store')
  return event.context.application.activity.detail(session.user.id, workspaceId, activityId)
})
