import type { H3Event } from 'h3'
import { trace } from '@opentelemetry/api'
import { logger } from './logger'
import { classifyRawCause } from '../../core/errors/classify'

const STARTED_AT = Symbol('plan035.request.startedAt')
const RECORDED = Symbol('plan035.request.recorded')
const TRACE_ID = Symbol('plan035.request.traceId')
const SPAN_ID = Symbol('plan035.request.spanId')
const MAX_DURATION_MS = 60_000
const MAX_ROUTE_LENGTH = 256

// `route` is a structured, known-safe attribute (never raw free text), so it
// gets validated against its own shape instead of flowing through
// `sanitize.ts`'s generic filesystem-path-redaction pattern meant for
// error messages/stacks — that generic pattern would otherwise treat any
// multi-segment path (e.g. `/api/auth/register`) as a filesystem path and
// collapse it to `[REDACTED-PATH]`, making the field useless for anything
// but `/`. Nitro route segments (see `server/api/**`, e.g.
// `providers/[id]/models`) are alphanumeric plus `-`, `_`, `.`, `/`, and
// bracketed param segments `[...]`; `event.path` at runtime carries the
// literal resolved segment value (e.g. the real id), not the `[id]`
// template, so the charset also allows whatever characters legitimately
// appear in path params (kept conservative: alphanumeric, `-`, `_`, `.`).
const SAFE_ROUTE_PATTERN = /^\/[A-Za-z0-9._/-]*$/

type LifecycleContext = Record<PropertyKey, unknown>

// Nitro's 'error' hook can fire for errors captured outside a real inbound
// request (e.g. during prerendering, or other internal error-capture paths
// that pass a partial/synthetic event) where `event` itself may be
// missing entirely. Guard against that rather than throwing out of an error
// handler — a missing event means there is no request to correlate, so
// the safe behavior is a no-op, never a crash.
function lifecycleContext(event?: H3Event): LifecycleContext | undefined {
  return event?.context as LifecycleContext | undefined
}

/**
 * Dedicated route sanitizer (Plan 035 Phase 3 remediation). Strips the
 * query string, length-caps, and validates the remaining path against a
 * conservative safe charset. Anything that doesn't match the shape of a
 * normal path (unexpected characters, control chars, etc.) is replaced
 * wholesale with `/` rather than partially redacted — this field is a
 * classification attribute, not free text, so there is no value in
 * preserving a mangled fragment of it.
 */
function safeRoute(event: H3Event): string {
  const withoutQuery = (event.path?.split('?')[0] || '/').slice(0, MAX_ROUTE_LENGTH)
  return SAFE_ROUTE_PATTERN.test(withoutQuery) ? withoutQuery : '/'
}

function outcome(status: number): 'ok' | 'client_error' | 'server_error' {
  if (status >= 500) return 'server_error'
  if (status >= 400) return 'client_error'
  return 'ok'
}

export function beginRequestLifecycle(event: H3Event): void {
  const context = lifecycleContext(event)
  if (!context) return
  context[STARTED_AT] = performance.now()
  const spanContext = trace.getActiveSpan()?.spanContext()
  if (spanContext?.traceId) context[TRACE_ID] = spanContext.traceId
  if (spanContext?.spanId) context[SPAN_ID] = spanContext.spanId
}

export function recordRequestLifecycle(event: H3Event | undefined, statusOverride?: number, errorCause?: unknown): void {
  const context = lifecycleContext(event)
  if (!context || !event?.node?.res) return
  if (context[RECORDED] === true) return
  context[RECORDED] = true

  const startedAt = typeof context[STARTED_AT] === 'number' ? context[STARTED_AT] : performance.now()
  const duration = Math.max(0, Math.min(MAX_DURATION_MS, Math.round(performance.now() - startedAt)))
  const status = statusOverride ?? event.node?.res?.statusCode ?? 500
  const attributes: Record<string, unknown> = {
    'event.name': 'http.request.lifecycle',
    'operation': 'http.request.lifecycle',
    'http.request.method': (event.method || 'UNKNOWN').slice(0, 16),
    'route': safeRoute(event),
    'http.response.status_code': status,
    'duration_ms': duration,
    'outcome': outcome(status)
  }
  if (typeof context.requestId === 'string') attributes['request.id'] = context.requestId
  if (typeof context[TRACE_ID] === 'string') attributes.trace_id = context[TRACE_ID]
  if (typeof context[SPAN_ID] === 'string') attributes.span_id = context[SPAN_ID]
  if (errorCause !== undefined) {
    attributes['error.type'] = errorCause instanceof Error
      ? errorCause.name || errorCause.constructor?.name || 'Error'
      : 'UnknownError'
    attributes['error.classification'] = classifyRawCause(errorCause)
  }
  logger.info('http.request.lifecycle', attributes)
}
