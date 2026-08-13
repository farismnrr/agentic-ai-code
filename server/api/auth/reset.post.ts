import { consumePasswordReset } from '../../application/auth'
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

  const hashedToken = hashToken(body.token)
  const tokenRecord = await consumePasswordReset(hashedToken, await hashPassword(body.password))

  if (!tokenRecord) {
    throw badRequest('Invalid password reset link.')
  }

  if (tokenRecord.consumedAt || tokenRecord.expiresAt < new Date()) {
    throw gone('This password reset link has expired or already been used.')
  }

  // Optionally, clear their session here to force re-login on all devices
  // But we use cookie sessions without DB tracking, so we can't easily invalidate other devices.
  // The user can login normally now.

  return { ok: true }
})
