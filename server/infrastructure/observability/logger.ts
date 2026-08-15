import { consola } from 'consola'
import { trace } from '@opentelemetry/api'
import { getLogger } from './otel'
import { redactSecrets, sanitizeAttributes, sanitizeMessage, shouldIncludeStack } from './sanitize'
import { classifyErrorType, classifyRawCause } from '../../core/errors/classify'
import { isSafeDiagnostic } from '../../core/errors/safe-diagnostic'

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

// Plan 035 P1 remediation (round 4): raw `Error.message`/`stack` are NOT a
// data-classification boundary — `redactSecrets()` only masks
// credential-shaped substrings/paths, not arbitrary request-derived/PII
// text (e.g. a DB unique-constraint message embedding a submitted email).
// Only a `SafeDiagnosticError` (a developer deliberately opted a composed,
// non-interpolated message in — see server/core/errors/safe-diagnostic.ts)
// may have its `.message`/`.stack` logged verbatim. Every other raw/
// untrusted exception gets only its constructor name plus a bounded static
// classification (server/core/errors/classify.ts) — fail-closed by design:
// less detail for truly unknown exceptions is the point, not a regression.
function errorAttributes(err: unknown): LogAttributes {
  if (err === undefined) return {}
  if (isSafeDiagnostic(err)) {
    const attrs: LogAttributes = { 'error.type': 'SafeDiagnosticError', 'error.message': err.message }
    if (shouldIncludeStack() && err.stack) attrs.stack = err.stack
    return attrs
  }
  if (err instanceof Error) {
    return {
      'error.type': classifyErrorType(err),
      'error.classification': classifyRawCause(err)
    }
  }
  return { 'error.type': 'UnknownError', 'error.classification': classifyRawCause(err) }
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

// consola prints straight to stdout (dev console / `docker compose logs`),
// a separate output path from the sanitized `emit()` -> Loki bridge above.
// Never pass an exception object to consola: its formatter can inspect
// message/stack (and nested properties) before our structured sanitizer sees
// it. Raw causes are therefore represented only by bounded, non-Error
// classification fields. SafeDiagnosticError is an explicit opt-in for a
// developer-authored diagnostic and still uses a plain object to keep the
// stdout boundary free of Error objects.
function consolaSafe(err: unknown): Record<string, string> | undefined {
  if (err === undefined) return undefined
  if (isSafeDiagnostic(err)) {
    return { type: 'SafeDiagnosticError', message: redactSecrets(err.message) }
  }
  if (err instanceof Error) {
    return {
      type: classifyErrorType(err),
      classification: classifyRawCause(err)
    }
  }
  return { type: 'UnknownError', classification: classifyRawCause(err) }
}

export const logger = {
  error(message: string, err?: unknown, attributes: LogAttributes = {}) {
    if (err === undefined) consola.error(redactSecrets(message))
    else consola.error(redactSecrets(message), consolaSafe(err))
    emit(17, 'ERROR', message, { ...errorAttributes(err), ...attributes })
  },
  warn(message: string, err?: unknown, attributes: LogAttributes = {}) {
    if (err === undefined) consola.warn(redactSecrets(message))
    else consola.warn(redactSecrets(message), consolaSafe(err))
    emit(13, 'WARN', message, { ...errorAttributes(err), ...attributes })
  },
  info(message: string, attributes: LogAttributes = {}) {
    consola.info(redactSecrets(message))
    emit(9, 'INFO', message, attributes)
  },
  // Forwards to Loki only, no consola print — for wrapping output Node/a
  // dependency already prints on its own (e.g. process.emitWarning), where
  // calling logger.warn() would duplicate every line on stdout.
  forwardOnly(severityNumber: number, severityText: string, message: string, attributes: LogAttributes = {}) {
    emit(severityNumber, severityText, message, attributes)
  }
}
