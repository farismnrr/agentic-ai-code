import { and, asc, eq, lt, lte, sql } from 'drizzle-orm'
import { taskNotificationOutbox } from '../../database/schema'
import type { SanitizedTaskCompletion, TaskCompletionInput, TaskCompletionNotificationPort } from '../../application/task-notifications'
import { sanitizeTaskCompletion } from '../../application/task-notifications'
import { useDb } from './connection'

const CLAIM_STALE_MS = 5 * 60 * 1000

export type TaskNotificationRow = typeof taskNotificationOutbox.$inferSelect

export const taskNotificationDatabase = {
  async enqueue(input: TaskCompletionInput | SanitizedTaskCompletion) {
    const payload = sanitizeTaskCompletion(input)
    await useDb().insert(taskNotificationOutbox).values({
      source: payload.source,
      taskId: payload.taskId,
      workspace: payload.workspace,
      title: payload.title,
      summary: payload.summary,
      completedAt: new Date(payload.completedAt),
      resultUrl: payload.resultUrl,
      status: 'pending',
      nextAttemptAt: new Date()
    }).onConflictDoNothing({ target: [taskNotificationOutbox.source, taskNotificationOutbox.taskId] })
  },

  async claimPending(limit = 10): Promise<TaskNotificationRow[]> {
    const db = useDb()
    const now = new Date()
    const staleBefore = new Date(now.getTime() - CLAIM_STALE_MS)
    // Crash recovery is a separate state transition so the claim below can
    // use the same compare-and-swap condition for every worker.
    await db.update(taskNotificationOutbox).set({ status: 'pending', nextAttemptAt: now, updatedAt: now }).where(and(eq(taskNotificationOutbox.status, 'sending'), lt(taskNotificationOutbox.updatedAt, staleBefore)))
    const candidates = await db.select().from(taskNotificationOutbox).where(and(
      eq(taskNotificationOutbox.status, 'pending'),
      lte(taskNotificationOutbox.nextAttemptAt, now)
    )).orderBy(asc(taskNotificationOutbox.createdAt), asc(taskNotificationOutbox.id)).limit(Math.min(limit, 25))
    const claimed: TaskNotificationRow[] = []
    for (const candidate of candidates) {
      const [row] = await db.update(taskNotificationOutbox).set({
        status: 'sending',
        attempts: sql`${taskNotificationOutbox.attempts} + 1`,
        updatedAt: now,
        lastError: null
      }).where(and(
        eq(taskNotificationOutbox.id, candidate.id),
        eq(taskNotificationOutbox.status, 'pending')
      )).returning()
      if (row) claimed.push(row)
    }
    return claimed
  },

  async markSent(id: string) {
    const now = new Date()
    await useDb().update(taskNotificationOutbox).set({ status: 'sent', sentAt: now, updatedAt: now, lastError: null }).where(and(eq(taskNotificationOutbox.id, id), eq(taskNotificationOutbox.status, 'sending')))
  },

  async markRetry(id: string, category: string, delayMs: number) {
    const now = new Date()
    await useDb().update(taskNotificationOutbox).set({ status: 'pending', nextAttemptAt: new Date(now.getTime() + Math.min(delayMs, 300_000)), updatedAt: now, lastError: category.slice(0, 64) }).where(and(eq(taskNotificationOutbox.id, id), eq(taskNotificationOutbox.status, 'sending')))
  },

  async markFailed(id: string, category: string) {
    await useDb().update(taskNotificationOutbox).set({ status: 'failed', updatedAt: new Date(), lastError: category.slice(0, 64) }).where(and(eq(taskNotificationOutbox.id, id), eq(taskNotificationOutbox.status, 'sending')))
  }
} satisfies TaskCompletionNotificationPort
