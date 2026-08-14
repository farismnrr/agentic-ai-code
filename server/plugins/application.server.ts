import { randomUUID } from 'node:crypto'
import { createApplicationAdapters } from '../infrastructure/composition/application'

// Server-generated request ID (Plan 035 Phase 1): assigned once per inbound
// request, before any handler/middleware runs, and set on the response
// immediately — so it is present on both success and error responses (the
// header is written here, not after handler completion, so a thrown/
// unhandled error later in the request lifecycle does not need its own
// header-setting path). Never trust a client-supplied request ID as this
// value's source.
export default defineNitroPlugin((nitroApp) => {
  nitroApp.hooks.hook('request', (event) => {
    const requestId = randomUUID()
    setResponseHeader(event, 'x-request-id', requestId)
    event.context.application = createApplicationAdapters(requestId)
  })
})
