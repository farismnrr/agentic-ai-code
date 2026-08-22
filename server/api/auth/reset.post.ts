import { badRequest, unprocessable, tooManyRequests } from '#server/core/errors/http'
import { resetPasswordSchema as resetSchema } from '../../../shared/schemas/auth'
import * as v from 'valibot'

export default defineEventHandler(async (event) => {
  const result = v.safeParse(resetSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = event.context.application.network.rateLimit({ key: `reset:${ip}`, maxAttempts: 10 })
  if (limited) {
    throw tooManyRequests(retryAfter)
  }

  const hashedToken = event.context.application.security.hashToken(body.token)
  const tokenRecord = await event.context.application.auth.consumePasswordReset(hashedToken, await hashPassword(body.password))

  if (!tokenRecord) {
    event.context.application.observability.request?.event('auth.password_reset', 'denied', { 'auth.present': false })
    throw badRequest('Invalid password reset link.')
  }

  // Clear this browser immediately. Other sealed-cookie sessions are rejected
  // by the auth-version guard on their next request.
  await clearUserSession(event)
  event.context.application.observability.request?.event('auth.password_reset', 'ok', { 'auth.present': true })

  return { ok: true }
})
