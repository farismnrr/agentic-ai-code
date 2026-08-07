import { eq } from 'drizzle-orm'
import { users, verificationTokens } from '../../database/schema'
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

  const db = useDb()
  const [user] = await db
    .select({ id: users.id, email: users.email })
    .from(users)
    .where(eq(users.email, body.email))
    .limit(1)

  // We ALWAYS return success to prevent user enumeration
  if (!user) {
    return { ok: true }
  }

  // Generate and send password reset email
  const { token, hash: tokenHash } = generateToken()
  const expiresAt = new Date(Date.now() + 1 * 60 * 60 * 1000) // 1 hour

  await db.insert(verificationTokens).values({
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
    console.warn('[email] delivery failed', { to: user.email, purpose: 'forgot' })
  }

  return { ok: true }
})
