import { trace, SpanStatusCode } from '@opentelemetry/api'
import type { RequestTelemetryContext } from '../../application/observability/contracts'
import { getTracer } from './otel'
import { logger } from './logger'

const TRACER_NAME = 'ai-code-server'

// Implements the application-owned `RequestTelemetryContext` contract on top
// of the existing OTel tracer/logger pipeline. Composed once per request at
// the Nitro composition edge (server/infrastructure/composition/application.ts)
// and handed to application/core code only through that already-frozen
// contract, so OTel SDK types never leak past this file.
export function createRequestTelemetryContext(requestId: string): RequestTelemetryContext {
  return {
    requestId,
    get traceId() {
      return trace.getActiveSpan()?.spanContext().traceId
    },
    get spanId() {
      return trace.getActiveSpan()?.spanContext().spanId
    },
    withSpan<T>(operation: string, safeAttributes: Record<string, unknown>, fn: () => T | Promise<T>): T | Promise<T> {
      const tracer = getTracer(TRACER_NAME)
      return tracer.startActiveSpan(operation, { attributes: { 'request.id': requestId, ...safeAttributes } }, (span) => {
        const onError = (err: unknown) => {
          span.recordException(err instanceof Error ? err : String(err))
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
              (err: unknown) => {
                onError(err)
                throw err
              }
            ) as T
          }
          span.end()
          return result
        } catch (err) {
          onError(err)
          throw err
        }
      })
    },
    event(name: string, outcome: string, safeAttributes: Record<string, unknown> = {}) {
      logger.info(name, { 'request.id': requestId, 'operation': name, outcome, ...safeAttributes })
    },
    error(name: string, errorCode: string, cause: unknown, safeAttributes: Record<string, unknown> = {}) {
      logger.error(name, cause, { 'request.id': requestId, 'operation': name, 'outcome': 'error', 'error.code': errorCode, ...safeAttributes })
    }
  }
}
