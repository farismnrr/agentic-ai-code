import { badRequest } from '#server/core/errors/http'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Session id is required')
  const revoked = await event.context.application.account.revokeAuthSession(session.user.id, id)
  const current = await getUserSession(event)
  const currentId = current.secure?.authSession?.id
  if (currentId === id) await clearUserSession(event)
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'auth.session_revoked', outcome: 'ok', metadata: { current: currentId === id } })
  return { ok: true, revoked: revoked.length > 0 }
})
