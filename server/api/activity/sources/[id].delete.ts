import { badRequest } from '#server/core/errors/http'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const sourceId = getRouterParam(event, 'id')
  if (!sourceId) throw badRequest('Missing activity source id')
  await event.context.application.activity.revoke(session.user.id, sourceId)
  return { revoked: true }
})
