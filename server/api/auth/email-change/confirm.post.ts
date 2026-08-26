import { badRequest, gone, tooManyRequests, unprocessable } from '#server/core/errors/http'
import * as v from 'valibot'
import { verifySchema } from '../../../../shared/schemas/auth'

export default defineEventHandler(async (event) => {
  const parsed = v.safeParse(verifySchema, await readBody(event))
  if (!parsed.success) throw unprocessable(parsed.issues)
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = event.context.application.network.rateLimit({ key: `email-change-confirm:${ip}`, maxAttempts: 10 })
  if (limited) throw tooManyRequests(retryAfter)

  let result: { id: string, oldEmail: string, newEmail: string, now: Date } | null
  try {
    result = await event.context.application.auth.consumeEmailChange(event.context.application.security.hashToken(parsed.output.token)) as { id: string, oldEmail: string, newEmail: string, now: Date } | null
  } catch (error) {
    if (event.context.application.database.isUniqueViolation(error)) throw badRequest('This email address is no longer available.')
    throw error
  }
  if (!result) {
    await event.context.application.audit.record({ eventType: 'auth.email_change', outcome: 'denied' })
    throw gone('This email change link has expired or already been used.')
  }
  await clearUserSession(event)
  const { sendEmail, getTemplate } = event.context.application.mail
  const config = useRuntimeConfig()
  const notified = await sendEmail({
    to: result.oldEmail,
    subject: 'Your account email was changed',
    html: getTemplate('Account email changed', 'The sign-in email for your account was changed. If you did not request this, reset your password immediately.', 'Open account recovery', `${config.public.siteUrl}/forgot-password`)
  })
  if (!notified) await event.context.application.audit.record({ userId: result.id, eventType: 'auth.email_change_notification', outcome: 'error' })
  await event.context.application.audit.record({ userId: result.id, eventType: 'auth.email_change', outcome: 'ok' })
  return { ok: true }
})
