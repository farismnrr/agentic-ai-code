import { badRequest } from '#server/core/errors/http'
import { browserSessionFrom } from '../../../application/auth-session'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const current = browserSessionFrom(await getUserSession(event))
  if (!current) throw badRequest('A browser session is required')
  const revoked = await event.context.application.account.revokeOtherAuthSessions(session.user.id, current.id)
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'auth.sessions_revoked', outcome: 'ok', metadata: { count: revoked.length } })
  return { ok: true, revoked: revoked.length }
})
