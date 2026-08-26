import { logger } from '../infrastructure/observability/logger'
import { verifyApiKey } from '../infrastructure/auth/api-key'
import { useDb } from '../infrastructure/database/connection'
import { users } from '../database/schema'
import { eq } from 'drizzle-orm'
import { browserSessionFrom, hashSessionSecret } from '../application/auth-session'

export default defineEventHandler(async (event) => {
  const authHeader = getHeader(event, 'Authorization')
  if (authHeader && authHeader.startsWith('Bearer aic_live_')) {
    // The Nitro composition-edge plugin (server/plugins/application.server.ts)
    // runs on the `request` hook before any server/middleware/** handler, so
    // event.context.application.observability.request is already present
    // here — safe to record the auth/API-key decision as a cheap event
    // (Plan 035 Phase 6 item 2), not a span, at the point the decision is made.
    const telemetry = event.context.application?.observability?.request
    try {
      const userId = await verifyApiKey(authHeader.slice('Bearer '.length))
      const db = useDb()
      const [user] = await db.select().from(users).where(eq(users.id, userId)).limit(1)
      if (user) {
        (event.context as typeof event.context & { apiKeyAuthenticated?: boolean }).apiKeyAuthenticated = true
        // Inject into event context so requireUserSession succeeds without needing a cookie
        await setUserSession(event, {
          user: {
            id: user.id,
            email: user.email,
            name: user.name,
            avatarUrl: user.avatarUrl,
            emailVerifiedAt: user.emailVerifiedAt?.toISOString() ?? null,
            authVersion: user.authVersion,
            role: user.role
          },
          secure: { authMethod: 'api_key' }
        })
        telemetry?.event('auth.login', 'ok', { 'auth.present': true })
      } else {
        telemetry?.event('auth.login', 'denied', { 'auth.present': true })
      }
    } catch (err) {
      logger.error('[api-auth] API Key verification failed', err)
      telemetry?.event('auth.login', 'denied', { 'auth.present': true })
    }
  }

  // Sealed cookies cannot be deleted centrally. Bind browser sessions to an
  // auth generation so a password reset invalidates every older cookie.
  const session = await getUserSession(event)
  const sessionUser = session.user as { id?: string, authVersion?: number, role?: 'user' | 'admin' } | undefined
  if (!sessionUser?.id) return

  // API-key requests are stateless and intentionally do not create browser
  // session rows. Their credential was already verified above.
  if (session.secure?.authMethod === 'api_key') {
    if ((event.context as typeof event.context & { apiKeyAuthenticated?: boolean }).apiKeyAuthenticated) return
    await clearUserSession(event)
    event.context.application?.observability?.request?.event('auth.session', 'denied', { 'auth.present': true })
    return
  }

  const browserSession = browserSessionFrom(session)
  if (!browserSession) {
    await clearUserSession(event)
    event.context.application?.observability?.request?.event('auth.session', 'denied', { 'auth.present': true })
    return
  }

  const validBrowserSession = await event.context.application.account.validateAuthSession({
    id: browserSession.id,
    userId: sessionUser.id,
    secretHash: hashSessionSecret(browserSession.secret)
  })
  if (!validBrowserSession) {
    await clearUserSession(event)
    event.context.application?.observability?.request?.event('auth.session', 'denied', { 'auth.present': true })
    return
  }
  await event.context.application.account.touchAuthSession({
    id: browserSession.id,
    userId: sessionUser.id,
    secretHash: hashSessionSecret(browserSession.secret)
  })

  const db = useDb()
  const [current] = await db.select({ authVersion: users.authVersion, role: users.role }).from(users).where(eq(users.id, sessionUser.id)).limit(1)
  if (!current || (sessionUser.authVersion ?? 0) !== current.authVersion || sessionUser.role !== current.role) {
    await clearUserSession(event)
    event.context.application?.observability?.request?.event('auth.session', 'denied', { 'auth.present': true })
  }
})
