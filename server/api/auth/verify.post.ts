import { badRequest, unprocessable, tooManyRequests } from '#server/core/errors/http'
import { verifySchema } from '../../../shared/schemas/auth'
import * as v from 'valibot'

export default defineEventHandler(async (event) => {
  const result = v.safeParse(verifySchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = event.context.application.network.rateLimit({ key: `verify:${ip}`, maxAttempts: 10 })
  if (limited) {
    throw tooManyRequests(retryAfter)
  }

  const hashedToken = event.context.application.security.hashToken(body.token)
  const consumed = await event.context.application.auth.consumeEmailVerification(hashedToken)
  const tokenRecord = consumed?.record
  if (!tokenRecord) {
    await event.context.application.audit.record({ eventType: 'auth.email_verification', outcome: 'denied' })
    throw badRequest('Invalid verification link.')
  }

  const now = consumed.now

  // Update session if they are currently logged in as that user
  const session = await getUserSession(event)
  const sessionUser = session.user as { id: string, emailVerifiedAt?: string | null } | undefined

  if (sessionUser && sessionUser.id === tokenRecord.userId) {
    await replaceUserSession(event, {
      ...session,
      user: {
        ...session.user,
        id: sessionUser.id,
        emailVerifiedAt: now.toISOString()
      }
    })
  }

  await event.context.application.audit.record({ userId: tokenRecord.userId, eventType: 'auth.email_verification', outcome: 'ok' })

  return { ok: true }
})
