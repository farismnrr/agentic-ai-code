import * as v from 'valibot'
import { isTelemetryEventName } from '#shared/utils/telemetry'
import { tooManyRequests } from '#server/core/errors/http'

// Plan 035 Phase 5 — full hardening of frontend telemetry ingestion. Limits
// below are copied byte-for-byte from `app/composables/useTelemetry.ts`
// (Phase 4, frozen contract §6) so legitimate frontend traffic is never
// rejected, while a malicious/modified client is still bounded server-side.
const MAX_ATTRIBUTE_STRING_LENGTH = 256
const MAX_ATTRIBUTE_KEYS = 16
const MAX_MESSAGE_LENGTH = 512
const MAX_BATCH_RECORDS = 50
const MAX_BATCH_BYTES = 64 * 1024
// Frontend's own `sanitizeString()` appends a single "…" when it truncates,
// so an honestly-truncated string can be one rune longer than the raw cap.
const MAX_MESSAGE_LENGTH_WITH_ELLIPSIS = MAX_MESSAGE_LENGTH + 1
const MAX_ATTRIBUTE_STRING_LENGTH_WITH_ELLIPSIS = MAX_ATTRIBUTE_STRING_LENGTH + 1

const RATE_LIMIT_MAX_PER_MINUTE = 20
const RATE_LIMIT_WINDOW_MS = 60 * 1000

const TELEMETRY_LEVELS = ['debug', 'info', 'warn', 'error', 'fatal'] as const

const TRACE_ID_RE = /^[0-9a-f]{32}$/
const SPAN_ID_RE = /^[0-9a-f]{16}$/
const REQUEST_ID_RE = /^[A-Za-z0-9_-]{1,128}$/

const attributeValueSchema = v.union([
  v.pipe(v.string(), v.maxLength(MAX_ATTRIBUTE_STRING_LENGTH_WITH_ELLIPSIS)),
  v.number(),
  v.boolean()
])

const logEventSchema = v.object({
  level: v.picklist(TELEMETRY_LEVELS),
  message: v.pipe(v.string(), v.maxLength(MAX_MESSAGE_LENGTH_WITH_ELLIPSIS)),
  attributes: v.optional(v.pipe(
    v.record(v.string(), attributeValueSchema),
    v.check(attrs => Object.keys(attrs).length <= MAX_ATTRIBUTE_KEYS, 'Too many attribute keys')
  )),
  timestamp: v.optional(v.number())
})

const telemetrySchema = v.pipe(
  v.array(logEventSchema),
  v.maxLength(MAX_BATCH_RECORDS)
)

type LogEvent = v.InferOutput<typeof logEventSchema>

const SEVERITY_BY_LEVEL: Record<typeof TELEMETRY_LEVELS[number], { number: number, text: string }> = {
  fatal: { number: 21, text: 'FATAL' },
  error: { number: 17, text: 'ERROR' },
  warn: { number: 13, text: 'WARN' },
  debug: { number: 5, text: 'DEBUG' },
  info: { number: 9, text: 'INFO' }
}

/**
 * Remaps/validates the correlation and event-identity fields out of a raw
 * client attribute bag before it reaches `logger.forwardOnly` (which itself
 * re-applies the server allowlist in `sanitize.ts` as the final chokepoint).
 * Malformed correlation fields are silently discarded — they must not fail
 * the whole batch, per Phase 5 item 7. `service.name`/`component` are never
 * taken from the client; the server always assigns them.
 */
function buildAttributes(raw: Record<string, unknown> | undefined): Record<string, unknown> {
  const attrs: Record<string, unknown> = {
    'service.name': 'ai-code-frontend',
    'component': 'frontend'
  }
  if (!raw) return attrs

  for (const [key, value] of Object.entries(raw)) {
    if (key === 'service.name' || key === 'component' || key === 'deployment.environment') continue
    if (key === 'trace.id') {
      if (typeof value === 'string' && TRACE_ID_RE.test(value)) attrs.trace_id = value
      continue
    }
    if (key === 'span.id') {
      if (typeof value === 'string' && SPAN_ID_RE.test(value)) attrs.span_id = value
      continue
    }
    if (key === 'request.id') {
      if (typeof value === 'string' && REQUEST_ID_RE.test(value)) attrs['request.id'] = value
      continue
    }
    attrs[key] = value
  }
  return attrs
}

export default defineEventHandler(async (event) => {
  const session = await requireUserSession(event)

  const limited = event.context.application.network.rateLimit({
    key: `telemetry:${session.user.id}`,
    maxAttempts: RATE_LIMIT_MAX_PER_MINUTE,
    windowMs: RATE_LIMIT_WINDOW_MS
  })
  if (limited.limited) {
    throw tooManyRequests(limited.retryAfter)
  }

  const rawBody = await readRawBody(event, 'utf8')
  if (rawBody && Buffer.byteLength(rawBody, 'utf8') > MAX_BATCH_BYTES) {
    throw createError({ statusCode: 400, statusMessage: 'Telemetry batch exceeds max payload size' })
  }

  let parsed: unknown
  try {
    parsed = rawBody ? JSON.parse(rawBody) : []
  } catch {
    throw createError({ statusCode: 400, statusMessage: 'Invalid telemetry payload' })
  }

  let body: LogEvent[]
  try {
    body = v.parse(telemetrySchema, parsed)
  } catch {
    throw createError({ statusCode: 400, statusMessage: 'Invalid telemetry payload' })
  }

  // Reject-the-whole-batch on an unknown/missing event name rather than
  // silently dropping the record — a Phase 4-conformant client always sets
  // `event.name` to a vocabulary member, so this only fires for a
  // malicious/modified client, matching item 4's "reject unknown event
  // names" requirement.
  for (const log of body) {
    const eventName = log.attributes?.['event.name']
    if (!isTelemetryEventName(eventName)) {
      throw createError({ statusCode: 400, statusMessage: 'Unknown telemetry event name' })
    }
  }

  const { logger } = event.context.application.observability

  for (const log of body) {
    const severity = SEVERITY_BY_LEVEL[log.level]
    logger.forwardOnly(
      severity.number,
      severity.text,
      log.message,
      buildAttributes(log.attributes)
    )
  }

  return { success: true }
})
