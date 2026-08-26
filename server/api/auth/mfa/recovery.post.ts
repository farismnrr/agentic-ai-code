import { badRequest, forbidden, tooManyRequests, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'
import { recoveryCodeSchema } from '../../../../shared/schemas/auth'
import { browserSessionFrom } from '../../../application/auth-session'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const parsed = v.safeParse(recoveryCodeSchema, await readBody(event))
  if (!parsed.success) throw unprocessable(parsed.issues)
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = event.context.application.network.rateLimit({ key: `mfa-recovery:${ip}:${session.user.id}`, maxAttempts: 5 })
  if (limited) throw tooManyRequests(retryAfter)
  const factors = await event.context.application.mfa.listFactors(session.user.id) as Array<{ confirmedAt?: Date | null }>
  if (!factors.some(factor => factor.confirmedAt)) throw badRequest('MFA is not enabled')
  const currentSession = await getUserSession(event)
  const browserSession = browserSessionFrom(currentSession)
  if (!browserSession) throw badRequest('A browser session is required')
  const code = await event.context.application.mfa.consumeRecoveryCode(session.user.id, event.context.application.security.hashToken(parsed.output.code.toLowerCase()))
  if (!code) {
    await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'mfa.recovery_code', outcome: 'denied' })
    throw forbidden('Recovery code is invalid or already used')
  }
  await replaceUserSession(event, {
    ...currentSession,
    secure: { ...currentSession.secure, authSession: { ...browserSession, freshAuthAt: Date.now() } }
  })
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'mfa.recovery_code', outcome: 'ok' })
  return { ok: true }
})
