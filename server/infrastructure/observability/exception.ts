import type { Span } from '@opentelemetry/api'
import { sanitizeAttributes, sanitizeMessage, shouldIncludeStack } from './sanitize'

const MAX_EXCEPTION_FIELD_LENGTH = 512

function bounded(value: string): string {
  const sanitized = sanitizeMessage(value)
  return sanitized.length > MAX_EXCEPTION_FIELD_LENGTH
    ? `${sanitized.slice(0, MAX_EXCEPTION_FIELD_LENGTH)}…`
    : sanitized
}

/**
 * Records only a bounded, sanitized exception representation on a span.
 * Never pass the original Error to OTel: Error messages and stacks can carry
 * credentials, request bodies, provider responses, or filesystem paths.
 */
export function recordSanitizedException(span: Span, cause: unknown): void {
  try {
    const isError = cause instanceof Error
    const rawType = isError ? cause.name || cause.constructor?.name || 'Error' : 'UnknownError'
    const rawMessage = isError ? cause.message : cause === undefined ? 'Unknown exception' : String(cause)
    const rawStack = isError ? cause.stack : undefined
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
