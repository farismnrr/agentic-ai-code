import { tooManyRequests, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'

const resendSchema = v.object({})

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const parsed = v.safeParse(resendSchema, await readBody(event) ?? {})
  if (!parsed.success) throw unprocessable(parsed.issues)

  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = event.context.application.network.rateLimit({ key: `verify-resend:${ip}:${session.user.id}`, maxAttempts: 3 })
  if (limited) throw tooManyRequests(retryAfter)

  const user = await event.context.application.auth.findUserByEmail(session.user.email) as { id: string, email: string, emailVerifiedAt?: Date | null } | undefined
  if (!user || user.id !== session.user.id || user.emailVerifiedAt) return { ok: true }

  const { token, hash: tokenHash } = event.context.application.security.generateToken()
  await event.context.application.auth.resendEmailVerification(user.id, {
    tokenHash,
    expiresAt: new Date(Date.now() + 24 * 60 * 60 * 1000)
  })
  const config = useRuntimeConfig()
  const { sendEmail, getTemplate } = event.context.application.mail
  const emailSent = await sendEmail({
    to: user.email,
    subject: 'Verify your email address',
    html: getTemplate('Verify your email', 'Please verify your email address to continue.', 'Verify Email', `${config.public.siteUrl}/verify-email?token=${token}`)
  })
  if (!emailSent) event.context.application.observability.logger.warn('[email] delivery failed', undefined, { userId: user.id, purpose: 'verify-resend' })
  return { ok: true }
})
