// Plan 035 Phase 4 — same-origin W3C trace continuity for user-initiated
// API actions. Wraps the global `$fetch` (the documented Nuxt pattern for a
// global fetch interceptor) so every same-origin `/api/**` request carries a
// fresh `traceparent` header, and records a bounded, sanitized
// success/error telemetry event correlated to that trace ID and the
// server-returned `x-request-id` response header.
//
// `traceparent` is attached ONLY when `isSameOriginApiRequest()` returns
// true — there is no code path here that can attach it to a third-party
// origin (user-configured provider base URLs, remote MCP endpoints, OAuth,
// etc.), per the plan's trust-boundary rule.
export default defineNuxtPlugin(() => {
  const telemetry = useTelemetry()

  const tracedFetch = $fetch.create({
    onRequest({ request, options }) {
      if (!isSameOriginApiRequest(request)) return

      const traceId = generateTraceId()
      const spanId = generateSpanId()
      const headers = new Headers(options.headers)
      headers.set('traceparent', buildTraceparent(traceId, spanId))
      options.headers = headers
      // Stash on the options object so onResponse/onResponseError can read
      // it back without re-generating (would break trace continuity).
      ;(options as { _telemetryTraceId?: string })._telemetryTraceId = traceId
    },
    onResponse({ request, response, options }) {
      if (!isSameOriginApiRequest(request) || isTelemetryEndpoint(request)) return
      const traceId = (options as { _telemetryTraceId?: string })._telemetryTraceId
      const requestId = response.headers.get('x-request-id') ?? undefined
      const ok = response.status < 400
      telemetry.logEvent(
        ok ? 'info' : 'warn',
        ok ? 'api.request.success' : 'api.request.error',
        `API request ${ok ? 'succeeded' : 'failed'}`,
        {
          'http.response.status_code': response.status,
          'trace.id': traceId,
          'request.id': requestId,
          'outcome': ok ? 'success' : 'error'
        }
      )
    },
    onResponseError({ request, response, options }) {
      if (!isSameOriginApiRequest(request) || isTelemetryEndpoint(request)) return
      const traceId = (options as { _telemetryTraceId?: string })._telemetryTraceId
      const requestId = response?.headers?.get('x-request-id') ?? undefined
      telemetry.logEvent('error', 'api.request.error', 'API request failed', {
        'http.response.status_code': response?.status,
        'trace.id': traceId,
        'request.id': requestId,
        'outcome': 'error'
      })
    }
  })

  // Documented Nuxt recipe for a global fetch interceptor:
  // https://nuxt.com/docs/guide/recipes/custom-fetch
  globalThis.$fetch = tracedFetch
})
