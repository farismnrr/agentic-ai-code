import { badRequest, conflict, tooManyRequests, unauthorized, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'
import { emailChangeSchema } from '../../../shared/schemas/auth'
import { browserSessionFrom } from '../../application/auth-session'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const parsed = v.safeParse(emailChangeSchema, await readBody(event))
  if (!parsed.success) throw unprocessable(parsed.issues)

  const reauthUser = await event.context.application.auth.findUserForReauth(session.user.id) as { passwordHash?: string | null } | undefined
  if (!reauthUser?.passwordHash || !(await verifyPassword(reauthUser.passwordHash, parsed.output.password))) {
    throw unauthorized('Re-authentication failed')
  }
  const currentSession = await getUserSession(event)
  const browserSession = browserSessionFrom(currentSession)
  if (!browserSession) throw badRequest('A browser session is required')
  await replaceUserSession(event, {
    ...currentSession,
    secure: {
      ...currentSession.secure,
      authSession: { ...browserSession, freshAuthAt: Date.now() }
    }
  })

  const email = parsed.output.email.trim().toLowerCase()
  if (email === (session.user.email ?? '').toLowerCase()) throw conflict('This email address is already in use')
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = event.context.application.network.rateLimit({ key: `email-change:${ip}:${session.user.id}`, maxAttempts: 3 })
  if (limited) throw tooManyRequests(retryAfter)

  if (await event.context.application.auth.userExists(email)) throw conflict('This email address is already in use')

  const { token, hash: tokenHash } = event.context.application.security.generateToken()
  await event.context.application.auth.requestEmailChange(session.user.id, {
    email,
    tokenHash,
    expiresAt: new Date(Date.now() + 30 * 60 * 1000)
  })
  const config = useRuntimeConfig()
  const { sendEmail, getTemplate } = event.context.application.mail
  const emailSent = await sendEmail({
    to: email,
    subject: 'Confirm your new email address',
    html: getTemplate('Confirm email change', 'Confirm this email address to finish changing the email on your account.', 'Confirm Email Change', `${config.public.siteUrl}/confirm-email-change#token=${token}`)
  })
  if (!emailSent) event.context.application.observability.logger.warn('[email] delivery failed', undefined, { userId: session.user.id, purpose: 'email-change' })
  await event.context.application.audit.record({ userId: session.user.id, actorUserId: session.user.id, eventType: 'auth.email_change_requested', outcome: 'challenged' })
  return { ok: true }
})
