import { badRequest, forbidden, conflict, internal } from '#server/core/errors/http'
import { safeDiagnostic } from '#server/core/errors/safe-diagnostic'
import { eq, and } from 'drizzle-orm'
import { users, oauthAccounts } from '../../database/schema'
import { establishAuthSession } from '../../transport/auth-session'

export default defineOAuthGoogleEventHandler({
  config: {
    // We expect the user's email address and profile info
    scope: ['email', 'profile']
  },
  async onSuccess(event, { user: googleUser }) {
    const db = event.context.application.database.db
    const provider = 'google'
    const providerAccountId = String(googleUser.sub)
    const email = googleUser.email
    const isEmailVerified = googleUser.email_verified

    if (!email) {
      throw badRequest('Google account has no email.')
    }

    // 1. Check if we already have this exact OAuth account linked
    const [existingAccount] = await db
      .select({ userId: oauthAccounts.userId })
      .from(oauthAccounts)
      .where(
        and(
          eq(oauthAccounts.provider, provider),
          eq(oauthAccounts.providerAccountId, providerAccountId)
        )
      )
      .limit(1)

    if (existingAccount) {
      // Fetch the user to seed the session
      const [user] = await db
        .select({ id: users.id, email: users.email, name: users.name, emailVerifiedAt: users.emailVerifiedAt, authVersion: users.authVersion, role: users.role })
        .from(users)
        .where(eq(users.id, existingAccount.userId))
        .limit(1)

      if (user) {
        await establishAuthSession(event, {
          id: user.id,
          email: user.email,
          name: user.name,
          emailVerifiedAt: user.emailVerifiedAt?.toISOString() ?? null,
          authVersion: user.authVersion,
          role: user.role
        })
        return sendRedirect(event, '/chat')
      }
    }

    // 2. Check if the email is already registered via password or another provider
    const [existingUser] = await db
      .select({ id: users.id, emailVerifiedAt: users.emailVerifiedAt })
      .from(users)
      .where(eq(users.email, email))
      .limit(1)

    if (existingUser) {
      // Security constraint from plan: Only auto-link if the provider verified the email
      if (!isEmailVerified) {
        throw forbidden('Please verify your Google email address first before linking.')
      }

      // Link the new OAuth account to the existing user
      try {
        await db.insert(oauthAccounts).values({
          provider,
          providerAccountId,
          userId: existingUser.id
        })
      } catch (err) {
        if (event.context.application.database.isUniqueViolation(err)) throw conflict('OAuth account already linked')
        throw err
      }

      // Fetch full user to seed session
      const [user] = await db
        .select({ id: users.id, email: users.email, name: users.name, emailVerifiedAt: users.emailVerifiedAt, authVersion: users.authVersion, role: users.role })
        .from(users)
        .where(eq(users.id, existingUser.id))
        .limit(1)

      if (user) {
        await establishAuthSession(event, {
          id: user.id,
          email: user.email,
          name: user.name,
          emailVerifiedAt: user.emailVerifiedAt?.toISOString() ?? null,
          authVersion: user.authVersion,
          role: user.role
        })
        return sendRedirect(event, '/chat')
      }
    }

    // 3. Completely new user
    if (!isEmailVerified) {
      throw forbidden('Google email must be verified to create an account.')
    }

    let createdUser
    try {
      [createdUser] = await db.insert(users).values({
        email,
        name: googleUser.name || 'User',
        avatarUrl: googleUser.picture,
        // Treat OAuth email as verified at creation time
        emailVerifiedAt: new Date()
      }).returning({
        id: users.id,
        email: users.email,
        name: users.name,
        emailVerifiedAt: users.emailVerifiedAt,
        authVersion: users.authVersion,
        role: users.role
      })
    } catch (err) {
      if (event.context.application.database.isUniqueViolation(err)) throw conflict('Email already registered')
      throw err
    }

    if (!createdUser) throw internal(safeDiagnostic('Account creation failed'))

    try {
      await db.insert(oauthAccounts).values({
        provider,
        providerAccountId,
        userId: createdUser.id
      })
    } catch (err) {
      if (event.context.application.database.isUniqueViolation(err)) throw conflict('OAuth account already linked')
      throw err
    }

    await establishAuthSession(event, {
      id: createdUser.id,
      email: createdUser.email,
      name: createdUser.name,
      emailVerifiedAt: createdUser.emailVerifiedAt?.toISOString() ?? null,
      authVersion: createdUser.authVersion,
      role: createdUser.role
    })

    return sendRedirect(event, '/chat')
  },
  onError(event, error) {
    event.context.application.observability.logger.error('Google OAuth error', error)
    return sendRedirect(event, '/login?error=Google login failed')
  }
})
