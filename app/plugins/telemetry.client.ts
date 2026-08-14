// Plan 035 Phase 4 — global error handlers emit sanitized error lifecycle
// events only. Never forward raw `error.message`/`error.stack` — only a
// stable `error.type` classification, per the Phase 0 audit's confirmed gap.
export default defineNuxtPlugin((nuxtApp) => {
  const telemetry = useTelemetry()

  // Vue error handler
  nuxtApp.vueApp.config.errorHandler = (err, _instance, info) => {
    telemetry.logError('page.error', err, { component: 'vue', operation: String(info).slice(0, 256) })
    if (import.meta.dev) {
      console.error('[Vue Error]', err, info)
    }
  }

  // Window error handler
  window.addEventListener('error', (event) => {
    telemetry.logError('page.error', event.error || new Error(event.message), { component: 'window' })
  })

  // Unhandled promise rejections
  window.addEventListener('unhandledrejection', (event) => {
    telemetry.logError('page.unhandled_rejection', event.reason, { component: 'window' })
  })
})
