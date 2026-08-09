// Singleton state
const batch = ref<Record<string, unknown>[]>([])
let isInitialized = false

const flush = () => {
  if (batch.value.length === 0) return
  const payload = JSON.stringify(batch.value)
  batch.value = [] // clear early
  if (navigator.sendBeacon) {
    const blob = new Blob([payload], { type: 'application/json' })
    navigator.sendBeacon('/api/telemetry', blob)
  } else {
    fetch('/api/telemetry', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: payload,
      keepalive: true
    }).catch(console.error)
  }
}

const logEvent = (level: string, message: string, attributes?: Record<string, unknown>) => {
  batch.value.push({
    level,
    message,
    attributes,
    timestamp: Date.now()
  })
}

const logError = (error: unknown, context?: Record<string, unknown>) => {
  const message = error instanceof Error ? error.message : String(error)
  const stack = error instanceof Error ? error.stack : undefined

  logEvent('error', message, {
    ...context,
    stack
  })
}

export function useTelemetry() {
  if (import.meta.client && !isInitialized) {
    isInitialized = true
    setInterval(flush, 5000)

    window.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') flush()
    })

    window.addEventListener('beforeunload', flush)
  }

  return { logEvent, logError, flush }
}
