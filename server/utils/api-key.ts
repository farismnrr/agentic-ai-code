import { randomBytes, createHash } from 'node:crypto'
import { eq } from 'drizzle-orm'
import type { H3Event } from 'h3'
import { apiKeys } from '../database/schema'

export function generateApiKey() {
  // e.g., aic_live_a1b2c3d4...
  const rawKey = `aic_live_${randomBytes(32).toString('hex')}`
  const keyPrefix = rawKey.substring(0, 13) // "aic_live_..."
  const keyHash = createHash('sha256').update(rawKey).digest('hex')
  return { rawKey, keyPrefix, keyHash }
}

export function hashApiKey(rawKey: string) {
  return createHash('sha256').update(rawKey).digest('hex')
}

export async function verifyApiKey(event: H3Event) {
  const authHeader = getHeader(event, 'Authorization')
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    throw createError({ statusCode: 401, message: 'Missing or invalid Authorization header' })
  }

  const rawKey = authHeader.substring(7)
  const keyHash = hashApiKey(rawKey)

  const db = useDb()
  const [apiKey] = await db
    .select({ userId: apiKeys.userId, id: apiKeys.id })
    .from(apiKeys)
    .where(eq(apiKeys.keyHash, keyHash))
    .limit(1)

  if (!apiKey) {
    throw createError({ statusCode: 401, message: 'Invalid API Key' })
  }

  // Bump lastUsedAt asynchronously
  db.update(apiKeys).set({ lastUsedAt: new Date() }).where(eq(apiKeys.id, apiKey.id)).execute().catch(err => logger.error('[api-key] failed to update lastUsedAt', err))

  return apiKey.userId
}
