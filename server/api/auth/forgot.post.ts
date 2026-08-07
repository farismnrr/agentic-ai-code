import { eq } from 'drizzle-orm'
import { users, verificationTokens } from '../../database/schema'
import { generateToken } from '../../utils/token'
import * as v from 'valibot'

const forgotSchema = v.object({
  email: v.pipe(v.string(), v.email())
})

export default defineEventHandler(async (event) => {
  const body = await readValidatedBody(event, data => v.parse(forgotSchema, data))

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({ key: `forgot:${ip}`, maxAttempts: 5 })
  if (limited) {
    throw createError({ statusCode: 429, message: `Too many attempts. Try again in ${retryAfter}s.` })
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

  await sendEmail({
    to: user.email,
    subject: 'Reset your password',
    html: getTemplate(
      'Reset Password',
      'We received a request to reset your password. If you didn\'t make this request, you can safely ignore this email.',
      'Reset Password',
      resetUrl
    )
  })

  return { ok: true }
})
