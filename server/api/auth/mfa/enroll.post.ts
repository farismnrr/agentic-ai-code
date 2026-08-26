import { badRequest, conflict } from '#server/core/errors/http'
import { isFreshAuth } from '../../../application/auth-session'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  if (!isFreshAuth(await getUserSession(event))) throw badRequest('Recent authentication is required')
  const factors = await event.context.application.mfa.listFactors(session.user.id) as Array<{ id: string, confirmedAt?: Date | null }>
  if (factors.length) throw conflict('MFA enrollment is already pending or enabled')
  const email = session.user.email
  if (!email) throw badRequest('Account email is required')

  const secret = event.context.application.security.generateTotpSecret()
  const factor = await event.context.application.mfa.createFactor(session.user.id, event.context.application.security.encryptSecret(secret)) as { id: string } | undefined
  if (!factor) throw badRequest('MFA enrollment could not be started')
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'mfa.enrollment_started', outcome: 'challenged' })
  return { factorId: factor.id, secret, otpauthUri: event.context.application.security.buildTotpUri(secret, email) }
})
