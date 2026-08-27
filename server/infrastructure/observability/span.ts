import { SpanStatusCode } from '@opentelemetry/api'
import { getTracer } from './otel'
import { recordSanitizedException } from './exception'
import { sanitizeAttributes } from './sanitize'

const TRACER_NAME = 'ai-code-server'

/**
 * Infrastructure-safe child-span helper.
 *
 * Every attribute passes through the same allowlist sanitizer used by the
 * structured logger. Raw errors are recorded only as sanitized private OTel
 * exception data and are never returned to callers.
 */
export function withInfrastructureSpan<T>(
  operation: string,
  attributes: Record<string, unknown>,
  fn: () => T | Promise<T>
): T | Promise<T> {
  const tracer = getTracer(TRACER_NAME)
  return tracer.startActiveSpan(operation, { attributes: sanitizeAttributes(attributes) }, (span) => {
    const fail = (error: unknown) => {
      recordSanitizedException(span, error)
      span.setStatus({ code: SpanStatusCode.ERROR })
      span.end()
    }

    try {
      const result = fn()
      if (result instanceof Promise) {
        return result.then(
          (value) => {
            span.end()
            return value
          },
          (error: unknown) => {
            fail(error)
            throw error
          }
        ) as T
      }
      span.end()
      return result
    } catch (error) {
      fail(error)
      throw error
    }
  })
}
