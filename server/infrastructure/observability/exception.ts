import type { Span } from '@opentelemetry/api'
import { sanitizeAttributes, sanitizeMessage, shouldIncludeStack } from './sanitize'
import { classifyRawCause } from '../../core/errors/classify'
import { isSafeDiagnostic } from '../../core/errors/safe-diagnostic'

const MAX_EXCEPTION_FIELD_LENGTH = 512

function bounded(value: string): string {
  const sanitized = sanitizeMessage(value)
  return sanitized.length > MAX_EXCEPTION_FIELD_LENGTH
    ? `${sanitized.slice(0, MAX_EXCEPTION_FIELD_LENGTH)}…`
    : sanitized
}

/**
 * Records only a bounded, sanitized exception representation on a span.
 * Never pass the original Error's raw `.message`/`.stack` to OTel by
 * default: they can carry credentials, request bodies, provider responses,
 * filesystem paths, or arbitrary request-derived/PII data that
 * `redactSecrets()` cannot detect (it only masks credential-shaped
 * substrings and paths — it is not a general data-classification
 * boundary). Only a `SafeDiagnosticError` (server/core/errors/
 * safe-diagnostic.ts) — a developer-authored, non-interpolated safe
 * message — may have its `.message`/`.stack` recorded verbatim; every
 * other raw/untrusted exception is reduced to its constructor name plus a
 * bounded static classification (server/core/errors/classify.ts).
 */
export function recordSanitizedException(span: Span, cause: unknown): void {
  try {
    const isError = cause instanceof Error
    const safeDiagnostic = isSafeDiagnostic(cause)
    const rawType = isError ? cause.name || cause.constructor?.name || 'Error' : 'UnknownError'
    const rawMessage = safeDiagnostic ? cause.message : classifyRawCause(cause)
    const rawStack = safeDiagnostic ? cause.stack : undefined
    const safe = sanitizeAttributes({
      'error.type': bounded(rawType),
      'error.message': bounded(rawMessage),
      ...(shouldIncludeStack() && rawStack ? { stack: bounded(rawStack) } : {})
    })

    const exception: { name: string, message: string, stack?: string } = {
      name: String(safe['error.type'] ?? 'UnknownError'),
      message: String(safe['error.message'] ?? 'Unknown exception')
    }
    if (typeof safe.stack === 'string') exception.stack = safe.stack
    span.recordException(exception)
  } catch {
    // Exception recording is diagnostic only and must never alter lifecycle.
  }
}
