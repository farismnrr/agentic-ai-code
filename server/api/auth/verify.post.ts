import { consumeEmailVerification } from '../../infrastructure/composition'
import { hashToken } from '../../utils/token'
import { verifySchema } from '../../../shared/schemas/auth'
import * as v from 'valibot'

export default defineEventHandler(async (event) => {
  const result = v.safeParse(verifySchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Rate limit
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({ key: `verify:${ip}`, maxAttempts: 10 })
  if (limited) {
    throw tooManyRequests(retryAfter)
  }

  const hashedToken = hashToken(body.token)
  const consumed = await consumeEmailVerification(hashedToken)
  const tokenRecord = consumed?.record

  if (!tokenRecord) {
    throw badRequest('Invalid verification link.')
  }

  if (tokenRecord.consumedAt || tokenRecord.expiresAt < new Date()) {
    throw gone('This verification link has expired or already been used.')
  }

  const now = consumed!.now

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

  return { ok: true }
})
