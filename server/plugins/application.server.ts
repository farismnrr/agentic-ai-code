import { randomUUID } from 'node:crypto'
import { createApplicationAdapters } from '../infrastructure/composition/application'
import { beginRequestLifecycle, recordRequestLifecycle } from '../infrastructure/observability/request-lifecycle'

// Server-generated request ID (Plan 035 Phase 1): assigned once per inbound
// request, before any handler/middleware runs, and set on the response
// immediately — so it is present on both success and error responses (the
// header is written here, not after handler completion, so a thrown/
// unhandled error later in the request lifecycle does not need its own
// header-setting path). Never trust a client-supplied request ID as this
// value's source.
export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook('request', (event) => {
    beginRequestLifecycle(event)
    const requestId = randomUUID()
    // Correlate the lifecycle record with the server-generated response ID;
    // never trust a client-supplied header for this value.
    event.context.requestId = requestId
    setResponseHeader(event, 'x-request-id', requestId)
    event.context.application = createApplicationAdapters(requestId)
  })
  nitroApp.hooks.hook('afterResponse', event => recordRequestLifecycle(event))
  // Nitro's 'error' hook signature is (error, context) where `context` is
  // `{ event, tags }` — NOT the raw H3Event itself (see nitropack's
  // createNitroApp -> captureError -> hooks.callHookParallel('error', error,
  // { event, tags })). Passing `context` straight into recordRequestLifecycle
  // as if it were the event silently no-ops (or, pre-guard, throws) because
  // `context.context`/`context.node` don't exist — the real event is
  // `context.event`.
  nitroApp.hooks.hook('error', (error, context) => {
    const statusCode = (error as { statusCode?: unknown })?.statusCode
    const status = typeof statusCode === 'number' ? statusCode : 500
    recordRequestLifecycle(context?.event, status)
  })
})
