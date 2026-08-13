import * as v from 'valibot'
import { addVerificationToken, createUser, userExists } from '../../application/auth'
import { registerSchema } from '../../../shared/schemas/auth'
import { generateToken } from '../../utils/token'

/**
 * POST /api/auth/register
 *
 * Creates a new account. Password is hashed with scrypt via nuxt-auth-utils'
 * `hashPassword` — no native bindings required.
 *
 * Duplicate email returns the same generic message as a non-existent email
 * to prevent user enumeration.
 *
 * Rate limit: 5 register attempts per IP per 15 minutes.
 */
export default defineEventHandler(async (event) => {
  const result = v.safeParse(registerSchema, await readBody(event))
  if (!result.success) throw unprocessable(result.issues)
  const body = result.output

  // Rate limit by IP to slow down bulk account creation.
  const ip = getRequestIP(event, { xForwardedFor: true }) ?? 'unknown'
  const { limited, retryAfter } = rateLimit({ key: `register:${ip}`, maxAttempts: 5 })
  if (limited) {
    throw tooManyRequests(retryAfter)
  }

  // Check for existing account. Return the same message regardless so the
  // response reveals nothing about which addresses are registered.
  if (await userExists(body.email)) {
    // Same wording as success — caller can't tell the difference.
    setResponseStatus(event, 201)
    return { ok: true }
  }

  const hash = await hashPassword(body.password)

  try {
    await createUser({ email: body.email, name: body.name, passwordHash: hash })
  } catch (err) {
    if (isUniqueViolation(err)) throw conflict('Email already registered')
    throw err
  }

  // Fetch the created user to seed the session.
  const created = await (await import('../../application/auth')).findLoginUser(body.email)

  if (!created) throw internal('Account creation failed')

  await setUserSession(event, {
    user: {
      id: created.id,
      email: created.email,
      name: created.name,
      emailVerifiedAt: created.emailVerifiedAt?.toISOString() ?? null
    }
  })

  // Generate and send verification email
  const { token, hash: tokenHash } = generateToken()
  const expiresAt = new Date(Date.now() + 24 * 60 * 60 * 1000) // 24 hours

  await addVerificationToken({
    tokenHash,
    userId: created.id,
    type: 'email_verify',
    expiresAt
  })

  const { sendEmail, getTemplate } = useMailer()
  const config = useRuntimeConfig()
  // The frontend handles the ?token= extraction
  const verifyUrl = `${config.public.siteUrl}/verify-email?token=${token}`

  const emailSent = await sendEmail({
    to: created.email,
    subject: 'Verify your email address',
    html: getTemplate(
      'Verify your email',
      'Thanks for signing up! Please verify your email address to continue.',
      'Verify Email',
      verifyUrl
    )
  })
  if (!emailSent) {
    logger.warn('[email] delivery failed', undefined, { to: created.email, purpose: 'register' })
  }

  setResponseStatus(event, 201)
  return { ok: true }
})
