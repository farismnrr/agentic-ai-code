import { eq, and } from 'drizzle-orm'
import { users, verificationTokens } from '../../database/schema'
import { hashToken } from '../../utils/token'
import * as v from 'valibot'

const verifySchema = v.object({
  token: v.string()
})

export default defineEventHandler(async (event) => {
  const body = await readValidatedBody(event, data => v.parse(verifySchema, data))

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({ key: `verify:${ip}`, maxAttempts: 10 })
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
        eq(verificationTokens.type, 'email_verify')
      )
    )
    .limit(1)

  if (!tokenRecord) {
    throw createError({ statusCode: 400, message: 'Invalid or expired verification link.' })
  }

  if (tokenRecord.consumedAt || tokenRecord.expiresAt < new Date()) {
    throw createError({ statusCode: 400, message: 'This verification link has expired or already been used.' })
  }

  // Mark token consumed and update user as verified
  const now = new Date()
  await db.transaction(async (tx) => {
    await tx
      .update(verificationTokens)
      .set({ consumedAt: now })
      .where(eq(verificationTokens.tokenHash, hashedToken))

    await tx
      .update(users)
      .set({ emailVerifiedAt: now, updatedAt: now })
      .where(eq(users.id, tokenRecord.userId))
  })

  // Update session if they are currently logged in as that user
  const session = await getUserSession(event)
  const sessionUser = session.user as { id: string, emailVerifiedAt?: string | null } | undefined

  if (sessionUser && sessionUser.id === tokenRecord.userId) {
    await replaceUserSession(event, {
      ...session,
      user: {
        ...session.user,
        emailVerifiedAt: now.toISOString()
      }
    })
  }

  return { ok: true }
})
