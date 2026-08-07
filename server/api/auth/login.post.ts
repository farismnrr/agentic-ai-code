import * as v from 'valibot'
import { eq } from 'drizzle-orm'
import { users } from '../../database/schema'
import { loginSchema } from '../../../shared/schemas/auth'

/**
 * POST /api/auth/login
 *
 * Verifies email + password and issues a session cookie.
 *
 * Error messages are intentionally identical for wrong email and wrong
 * password — a different message per case leaks which emails are registered.
 *
 * Rate limit: 10 attempts per (IP + email) pair per 15 minutes. Keying by
 * both prevents rotating one dimension to bypass the check.
 */

const GENERIC_ERROR = 'Invalid email or password'

export default defineEventHandler(async (event) => {
  const body = await readValidatedBody(event, data => v.parse(loginSchema, data))

  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({
    key: `login:${ip}:${body.email}`,
    maxAttempts: 10
  })
  if (limited) {
    throw createError({ statusCode: 429, message: `Too many attempts. Try again in ${retryAfter}s.` })
  }

  const db = useDb()

  const [user] = await db
    .select({
      id: users.id,
      email: users.email,
      name: users.name,
      passwordHash: users.passwordHash,
      emailVerifiedAt: users.emailVerifiedAt
    })
    .from(users)
    .where(eq(users.email, body.email))
    .limit(1)

  // No account OR account has no password (OAuth-only) — same generic message.
  if (!user || !user.passwordHash) {
    throw createError({ statusCode: 401, message: GENERIC_ERROR })
  }

  const valid = await verifyPassword(user.passwordHash, body.password)
  if (!valid) {
    throw createError({ statusCode: 401, message: GENERIC_ERROR })
  }

  await setUserSession(event, {
    user: {
      id: user.id,
      email: user.email,
      name: user.name,
      emailVerifiedAt: user.emailVerifiedAt?.toISOString() ?? null
    }
  })

  return { ok: true }
})
