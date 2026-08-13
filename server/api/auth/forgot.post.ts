import { generateToken } from '../../utils/token'
import { forgotPasswordSchema as forgotSchema } from '../../../shared/schemas/auth'
import * as v from 'valibot'

export default defineEventHandler(async (event) => {
  const result = v.safeParse(forgotSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({ key: `forgot:${ip}`, maxAttempts: 5 })
  if (limited) {
    throw tooManyRequests(retryAfter)
  }

  const user = await event.context.application.auth.findUserByEmail(body.email) as { id: string, email: string } | undefined

  // We ALWAYS return success to prevent user enumeration
  if (!user) {
    return { ok: true }
  }

  // Generate and send password reset email
  const { token, hash: tokenHash } = generateToken()
  const expiresAt = new Date(Date.now() + 1 * 60 * 60 * 1000) // 1 hour

  await event.context.application.auth.addVerificationToken({
    tokenHash,
    userId: user.id,
    type: 'password_reset',
    expiresAt
  })

  const { sendEmail, getTemplate } = useMailer()
  const config = useRuntimeConfig()
  const resetUrl = `${config.public.siteUrl}/reset-password?token=${token}`

  const emailSent = await sendEmail({
    to: user.email,
    subject: 'Reset your password',
    html: getTemplate(
      'Reset Password',
      'We received a request to reset your password. If you didn\'t make this request, you can safely ignore this email.',
      'Reset Password',
      resetUrl
    )
  })

  if (!emailSent) {
    logger.warn('[email] delivery failed', undefined, { to: user.email, purpose: 'forgot' })
  }

  return { ok: true }
})
