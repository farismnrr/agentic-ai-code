import { and, desc, eq, lt } from 'drizzle-orm'
import { securityEvents } from '../../database/schema'
import { useDb } from './connection'

export type SecurityEventMetadata = Record<string, string | number | boolean>

let lastPrunedAt = 0

export async function recordSecurityEvent(input: {
  userId?: string
  actorUserId?: string
  eventType: string
  outcome: 'ok' | 'denied' | 'error' | 'challenged'
  metadata?: SecurityEventMetadata
}) {
  const result = await useDb().insert(securityEvents).values({
    userId: input.userId,
    actorUserId: input.actorUserId,
    eventType: input.eventType,
    outcome: input.outcome,
    metadata: input.metadata ?? {}
  }).returning({ id: securityEvents.id })
  const now = Date.now()
  if (now - lastPrunedAt > 24 * 60 * 60 * 1000) {
    lastPrunedAt = now
    await useDb().delete(securityEvents).where(and(lt(securityEvents.createdAt, new Date(now - 180 * 24 * 60 * 60 * 1000))))
  }
  return result
}

export async function listSecurityEvents(userId: string) {
  return useDb().select({
    id: securityEvents.id,
    eventType: securityEvents.eventType,
    outcome: securityEvents.outcome,
    metadata: securityEvents.metadata,
    createdAt: securityEvents.createdAt
  }).from(securityEvents).where(eq(securityEvents.userId, userId)).orderBy(desc(securityEvents.createdAt)).limit(100)
}
