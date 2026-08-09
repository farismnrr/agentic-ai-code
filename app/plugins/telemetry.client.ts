export default defineNuxtPlugin((nuxtApp) => {
  const telemetry = useTelemetry()

  // Vue error handler
  nuxtApp.vueApp.config.errorHandler = (err, instance, info) => {
    telemetry.logError(err, { source: 'vue.errorHandler', info })
    // Log to console in development so we can see it
    if (import.meta.dev) {
      console.error('[Vue Error]', err, info)
    }
  }

  // Window error handler
  window.addEventListener('error', (event) => {
    telemetry.logError(event.error || new Error(event.message), {
      source: 'window.onerror',
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno
    })
  })

  // Unhandled promise rejections
  window.addEventListener('unhandledrejection', (event) => {
    telemetry.logError(event.reason, { source: 'window.onunhandledrejection' })
  })
})
