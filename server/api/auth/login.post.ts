import { unauthorized, unprocessable, tooManyRequests } from '#server/core/errors/http'
import * as v from 'valibot'
import { loginSchema } from '../../../shared/schemas/auth'
import { establishAuthSession } from '../../transport/auth-session'

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
  const result = v.safeParse(loginSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const ipLimit = event.context.application.network.rateLimit({
    key: `login:${ip}:${body.email}`,
    maxAttempts: 10
  })
  const accountLimit = event.context.application.network.rateLimit({ key: `login-account:${body.email}`, maxAttempts: 20 })
  if (ipLimit.limited || accountLimit.limited) {
    throw tooManyRequests(Math.max(ipLimit.retryAfter ?? 0, accountLimit.retryAfter ?? 0))
  }

  const user = await event.context.application.auth.findLoginUser(body.email) as { id: string, email: string, name: string, passwordHash?: string | null, emailVerifiedAt?: Date | null, authVersion: number, role?: 'user' | 'admin' } | undefined

  // No account OR account has no password (OAuth-only) — same generic message.
  if (!user || !user.passwordHash) {
    await event.context.application.audit.record({ eventType: 'auth.login', outcome: 'denied' })
    throw unauthorized(GENERIC_ERROR)
  }

  const valid = await verifyPassword(user.passwordHash, body.password)
  if (!valid) {
    await event.context.application.audit.record({ userId: user.id, eventType: 'auth.login', outcome: 'denied' })
    throw unauthorized(GENERIC_ERROR)
  }

  await establishAuthSession(event, {
    id: user.id,
    email: user.email,
    name: user.name,
    emailVerifiedAt: user.emailVerifiedAt?.toISOString() ?? null,
    authVersion: user.authVersion,
    role: user.role
  })
  await event.context.application.audit.record({ userId: user.id, actorUserId: user.id, eventType: 'auth.login', outcome: 'ok' })

  return { ok: true }
})
