import { logger } from '../infrastructure/observability/logger'
import { verifyApiKey } from '../infrastructure/auth/api-key'
import { users } from '../database/schema'
import { eq } from 'drizzle-orm'

export default defineEventHandler(async (event) => {
  const authHeader = getHeader(event, 'Authorization')
  if (authHeader && authHeader.startsWith('Bearer aic_live_')) {
    try {
      const userId = await verifyApiKey(authHeader)
      const db = useDb()
      const [user] = await db.select().from(users).where(eq(users.id, userId)).limit(1)
      if (user) {
        // Inject into event context so requireUserSession succeeds without needing a cookie
        await setUserSession(event, {
          user: {
            id: user.id,
            email: user.email,
            name: user.name,
            avatarUrl: user.avatarUrl,
            emailVerifiedAt: user.emailVerifiedAt?.toISOString() ?? null
          }
        })
      }
    } catch (err) {
      logger.error('[api-auth] API Key verification failed', err)
    }
  }
})
