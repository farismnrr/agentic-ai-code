import { taskNotificationDatabase, type TaskNotificationRow } from '../database/task-notifications'
import { createConfiguredFirstPartyRelayClient, type McpClientLike } from '../mcp/client'

const BATCH_SIZE = 10
const RETRY_DELAY_MS = 5_000

function retryDelayMs(attempts: number) {
  return Math.min(300_000, RETRY_DELAY_MS * 2 ** Math.min(attempts, 6))
}

export async function drainTaskNotificationOutbox() {
  const rows = await taskNotificationDatabase.claimPending(BATCH_SIZE)
  if (!rows.length) return 0

  let relay: McpClientLike | undefined
  try {
    try {
      relay = await createConfiguredFirstPartyRelayClient()
    } catch {
      for (const row of rows) await retry(row)
      return 0
    }
    if (!relay?.supportsTaskCompletion?.() || !relay.taskCompleted) {
      for (const row of rows) await retry(row, 'unsupported_capability')
      return 0
    }
    let delivered = 0
    for (const row of rows) {
      try {
        const result = await relay.taskCompleted({
          taskId: row.taskId,
          workspace: row.workspace,
          title: row.title,
          summary: row.summary,
          completedAt: row.completedAt.toISOString(),
          ...(row.resultUrl ? { resultUrl: row.resultUrl } : {})
        })
        if (result.status === 'disabled') {
          await taskNotificationDatabase.markFailed(row.id, 'relay_disabled')
        } else {
          await taskNotificationDatabase.markSent(row.id)
          delivered++
        }
      } catch {
        await retry(row)
      }
    }
    return delivered
  } finally {
    await relay?.close().catch(() => undefined)
  }
}

async function retry(row: TaskNotificationRow, category = 'relay_unavailable') {
  await taskNotificationDatabase.markRetry(row.id, category, retryDelayMs(row.attempts))
}
