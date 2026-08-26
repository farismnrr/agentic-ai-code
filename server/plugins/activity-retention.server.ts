import { activityDatabase } from '../infrastructure/database/activity'

const CLEANUP_INTERVAL_MS = 6 * 60 * 60 * 1000
const CLEANUP_INITIAL_DELAY_MS = 60 * 1000
const CLEANUP_BATCH_SIZE = 500

export default defineNitroPlugin(() => {
  const cleanup = async () => {
    const configuredDays = Number(useRuntimeConfig().activityRetentionDays)
    const retentionDays = Number.isInteger(configuredDays) && configuredDays >= 1 && configuredDays <= 3650 ? configuredDays : 90
    const cutoff = new Date(Date.now() - retentionDays * 24 * 60 * 60 * 1000)
    try {
      await activityDatabase.retain(cutoff, CLEANUP_BATCH_SIZE)
    } catch {
      // Database availability must not prevent Nitro startup. The next bounded
      // run retries cleanup without logging database errors or payload data.
    }
  }
  const initial = setTimeout(() => void cleanup(), CLEANUP_INITIAL_DELAY_MS)
  const interval = setInterval(() => void cleanup(), CLEANUP_INTERVAL_MS)
  initial.unref?.()
  interval.unref?.()
})
