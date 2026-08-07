import { eq, and } from 'drizzle-orm'
import { users, verificationTokens } from '../../database/schema'
import { hashToken } from '../../utils/token'
import * as v from 'valibot'

const resetSchema = v.object({
  token: v.string(),
  password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters'), v.maxLength(128, 'Password too long'))
})

export default defineEventHandler(async (event) => {
  const body = await readValidatedBody(event, data => v.parse(resetSchema, data))

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({ key: `reset:${ip}`, maxAttempts: 10 })
  if (limited) {
    throw createError({ statusCode: 429, message: `Too many attempts. Try again in ${retryAfter}s.` })
  }

  const db = useDb()
  const hashedToken = hashToken(body.token)

  const [tokenRecord] = await db
    .select()
    .from(verificationTokens)
    .where(
      and(
        eq(verificationTokens.tokenHash, hashedToken),
        eq(verificationTokens.type, 'password_reset')
      )
    )
    .limit(1)

  if (!tokenRecord) {
    throw createError({ statusCode: 400, message: 'Invalid or expired password reset link.' })
  }

  if (tokenRecord.consumedAt || tokenRecord.expiresAt < new Date()) {
    throw createError({ statusCode: 400, message: 'This password reset link has expired or already been used.' })
  }

  const newHash = await hashPassword(body.password)

  // Mark token consumed and update user's password
  await db.transaction(async (tx) => {
    await tx
      .update(verificationTokens)
      .set({ consumedAt: new Date() })
      .where(eq(verificationTokens.tokenHash, hashedToken))

    await tx
      .update(users)
      .set({ passwordHash: newHash, updatedAt: new Date() })
      .where(eq(users.id, tokenRecord.userId))
  })

  // Optionally, clear their session here to force re-login on all devices
  // But we use cookie sessions without DB tracking, so we can't easily invalidate other devices.
  // The user can login normally now.

  return { ok: true }
})
