import { badRequest, forbidden, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'
import { mfaEnrollmentSchema } from '../../../../shared/schemas/auth'
import { isFreshAuth } from '../../../application/auth-session'
import { generateRecoveryCodes } from '../../../application/mfa'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  if (!isFreshAuth(await getUserSession(event))) throw badRequest('Recent authentication is required')
  const parsed = v.safeParse(mfaEnrollmentSchema, await readBody(event))
  if (!parsed.success) throw unprocessable(parsed.issues)
  const factor = await event.context.application.mfa.findFactor(session.user.id, parsed.output.factorId) as { id: string, secretEncrypted: string, confirmedAt?: Date | null, revokedAt?: Date | null } | undefined
  if (!factor || factor.confirmedAt || factor.revokedAt) throw forbidden('MFA enrollment is invalid')
  try {
    if (!event.context.application.security.verifyTotpCode(event.context.application.security.decryptSecret(factor.secretEncrypted), parsed.output.code)) throw new Error('invalid')
  } catch {
    await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'mfa.enrollment', outcome: 'denied' })
    throw forbidden('MFA code is invalid')
  }
  const confirmed = await event.context.application.mfa.confirmFactor(session.user.id, factor.id)
  if (!confirmed) throw forbidden('MFA enrollment is no longer valid')
  const codes = generateRecoveryCodes()
  await event.context.application.mfa.replaceRecoveryCodes(session.user.id, codes.map(code => code.hash))
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'mfa.enrollment', outcome: 'ok' })
  return { ok: true, recoveryCodes: codes.map(code => code.value) }
})
