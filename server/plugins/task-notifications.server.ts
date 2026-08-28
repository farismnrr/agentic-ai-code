import { drainTaskNotificationOutbox } from '../infrastructure/task-notifications/outbox-worker'

const INITIAL_DELAY_MS = 5_000
const INTERVAL_MS = 5_000

export default defineNitroPlugin(() => {
  const drain = () => void drainTaskNotificationOutbox().catch(() => undefined)
  const initial = setTimeout(drain, INITIAL_DELAY_MS)
  const interval = setInterval(drain, INTERVAL_MS)
  initial.unref?.()
  interval.unref?.()
})
