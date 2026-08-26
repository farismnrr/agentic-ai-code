import { badRequest } from '#server/core/errors/http'
import { isFreshAuth } from '../../../application/auth-session'
import { generateRecoveryCodes } from '../../../application/mfa'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  if (!isFreshAuth(await getUserSession(event))) throw badRequest('Recent authentication is required')
  const factors = await event.context.application.mfa.listFactors(session.user.id) as Array<{ confirmedAt?: Date | null }>
  if (!factors.some(factor => factor.confirmedAt)) throw badRequest('MFA is not enabled')
  const codes = generateRecoveryCodes()
  await event.context.application.mfa.replaceRecoveryCodes(session.user.id, codes.map(code => code.hash))
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'mfa.recovery_codes_regenerated', outcome: 'ok' })
  return { ok: true, recoveryCodes: codes.map(code => code.value) }
})
