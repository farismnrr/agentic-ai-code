import { badRequest, tooManyRequests, unauthorized, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'
import { reauthSchema } from '../../../shared/schemas/auth'
import { browserSessionFrom } from '../../application/auth-session'

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)
  const parsed = v.safeParse(reauthSchema, await readBody(event))
  if (!parsed.success) throw unprocessable(parsed.issues)

  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = event.context.application.network.rateLimit({ key: `reauth:${ip}:${session.user.id}`, maxAttempts: 5 })
  if (limited) throw tooManyRequests(retryAfter)

  const user = await event.context.application.auth.findUserForReauth(session.user.id) as { passwordHash?: string | null } | undefined
  if (!user?.passwordHash || !(await verifyPassword(user.passwordHash, parsed.output.password))) {
    throw unauthorized('Re-authentication failed')
  }

  const browserSession = browserSessionFrom(session)
  if (!browserSession) throw badRequest('A browser session is required')

  await replaceUserSession(event, {
    ...session,
    secure: {
      ...session.secure,
      authSession: {
        ...browserSession,
        freshAuthAt: Date.now()
      }
    }
  })
  return { ok: true }
})
