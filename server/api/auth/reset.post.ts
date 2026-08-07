import { eq, and } from 'drizzle-orm'
import { users, verificationTokens } from '../../database/schema'
import { hashToken } from '../../utils/token'
import { resetPasswordSchema as resetSchema } from '../../../shared/schemas/auth'
import * as v from 'valibot'

export default defineEventHandler(async (event) => {
  const result = v.safeParse(resetSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({ key: `reset:${ip}`, maxAttempts: 10 })
  if (limited) {
    throw tooManyRequests(retryAfter)
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
    throw badRequest('Invalid password reset link.')
  }

  if (tokenRecord.consumedAt || tokenRecord.expiresAt < new Date()) {
    throw gone('This password reset link has expired or already been used.')
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
