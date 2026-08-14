import * as v from 'valibot'

const logEventSchema = v.object({
  level: v.string(),
  message: v.string(),
  attributes: v.optional(v.record(v.string(), v.any())),
  timestamp: v.optional(v.number())
})

const telemetrySchema = v.array(logEventSchema)

export default defineEventHandler(async (event) => {
  await requireUserSession(event)

  const body = await readValidatedBody(event, data => v.parse(telemetrySchema, data))

  // Plan 035 Phase 3 (narrow fix only — full endpoint hardening, schema
  // allowlisting, and rate limiting are Phase 5's job): this was calling
  // `.logger.emit(...)` on the sanitized `logger` wrapper, which has no
  // `emit` method — use the raw OTel logger the composition edge already
  // exposes via `getLogger` instead.
  const frontendLogger = event.context.application.observability.getLogger('ai-code-frontend')

  for (const log of body) {
    let severityNumber: number
    switch (log.level.toLowerCase()) {
      case 'fatal':
        severityNumber = 21
        break
      case 'error':
        severityNumber = 17
        break
      case 'warn':
      case 'warning':
        severityNumber = 13
        break
      case 'debug':
        severityNumber = 5
        break
      case 'info':
      default:
        severityNumber = 9
        break // INFO
    }

    // Phase 5 will replace this with the full sanitized/allowlisted
    // ingestion pipeline (attribute allowlist, size/rate limits, rejecting
    // unknown keys). For now: the confirmed raw-userId leak from the Phase 0
    // audit (`'userId': session.user?.id` written straight into every
    // telemetry record) is removed here, and `attributes` are no longer
    // spread verbatim — deferred hardening of the rest of this endpoint
    // (schema/rate-limit/reject-unknown-keys) is explicitly out of scope
    // for Phase 3.
    frontendLogger.emit({
      severityNumber,
      severityText: log.level.toUpperCase(),
      body: log.message,
      attributes: {
        'service.name': 'ai-code-frontend'
      },
      timestamp: log.timestamp ? new Date(log.timestamp) : undefined
    })
  }

  return { success: true }
})
