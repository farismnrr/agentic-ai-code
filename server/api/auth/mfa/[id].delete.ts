import { badRequest, forbidden } from '#server/core/errors/http'
import { isFreshAuth } from '../../../application/auth-session'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  if (!isFreshAuth(await getUserSession(event))) throw badRequest('Recent authentication is required')
  const id = getRouterParam(event, 'id')
  if (!id) throw badRequest('Factor id is required')
  const revoked = await event.context.application.mfa.revokeFactor(session.user.id, id)
  if (!revoked.length) throw forbidden('MFA factor is not available')
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'mfa.factor_revoked', outcome: 'ok' })
  return { ok: true }
})
