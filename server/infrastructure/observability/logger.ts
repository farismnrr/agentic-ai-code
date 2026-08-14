import { consola } from 'consola'
import { trace } from '@opentelemetry/api'
import { getLogger } from './otel'
import { sanitizeAttributes, sanitizeMessage, shouldIncludeStack } from './sanitize'

/**
 * Single logging entry point for server code — replaces raw `console.*`
 * calls, which only ever reached `docker compose logs` and were invisible
 * in Loki (see server/utils/http-errors.ts's `problem()` for the same fix
 * applied to thrown API errors). `consola` gives readable, leveled stdout
 * output in dev (it's already a Nitro/Nuxt-ecosystem convention — bundled
 * transitively via `nuxt`), and every call is also forwarded through the
 * existing OTel → Loki bridge so it shows up there too. `getLogger()`
 * no-ops when NUXT_OTEL_ENABLED isn't 'true', so this is safe to call
 * unconditionally in every environment.
 *
 * Plan 035 Phase 3: `emit()` below is the single chokepoint every one of
 * these calls funnels through, so it is also the single place that (a)
 * sanitizes attributes against the allowlist in `sanitize.ts`, (b) attaches
 * request/trace/span correlation when an active span exists, and (c)
 * degrades safely — a logging failure must never break the business
 * request it was describing.
 */

type LogAttributes = Record<string, unknown>

function errorAttributes(err: unknown): LogAttributes {
  if (err instanceof Error) {
    const attrs: LogAttributes = {
      'error.type': err.name || err.constructor?.name || 'Error',
      'error.message': err.message
    }
    if (shouldIncludeStack() && err.stack) attrs.stack = err.stack
    return attrs
  }
  if (err === undefined) return {}
  return { 'error.type': 'UnknownError', 'error.message': String(err) }
}

function emit(severityNumber: number, severityText: string, message: string, attributes: LogAttributes) {
  try {
    const spanContext = trace.getActiveSpan()?.spanContext()
    const correlated: LogAttributes = { 'service.name': 'ai-code-server', ...attributes }
    if (spanContext?.traceId) correlated.trace_id = spanContext.traceId
    if (spanContext?.spanId) correlated.span_id = spanContext.spanId

    getLogger('ai-code-server').emit({
      severityNumber,
      severityText,
      body: sanitizeMessage(message),
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      attributes: sanitizeAttributes(correlated) as any
    })
  } catch {
    // Logging/export must never break the request it is describing —
    // swallow any failure here (bad attribute shapes, exporter throwing
    // synchronously, etc). Network-level export failures are additionally
    // caught inside LokiLogExporter.export itself.
  }
}

export const logger = {
  error(message: string, err?: unknown, attributes: LogAttributes = {}) {
    if (err === undefined) consola.error(message)
    else consola.error(message, err)
    emit(17, 'ERROR', message, { ...errorAttributes(err), ...attributes })
  },
  warn(message: string, err?: unknown, attributes: LogAttributes = {}) {
    if (err === undefined) consola.warn(message)
    else consola.warn(message, err)
    emit(13, 'WARN', message, { ...errorAttributes(err), ...attributes })
  },
  info(message: string, attributes: LogAttributes = {}) {
    consola.info(message)
    emit(9, 'INFO', message, attributes)
  },
  // Forwards to Loki only, no consola print — for wrapping output Node/a
  // dependency already prints on its own (e.g. process.emitWarning), where
  // calling logger.warn() would duplicate every line on stdout.
  forwardOnly(severityNumber: number, severityText: string, message: string, attributes: LogAttributes = {}) {
    emit(severityNumber, severityText, message, attributes)
  }
}
