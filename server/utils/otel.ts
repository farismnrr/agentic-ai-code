import { trace, type Tracer } from '@opentelemetry/api'
import { logs, type Logger } from '@opentelemetry/api-logs'
import type { LogRecord, LogRecordExporter } from '@opentelemetry/sdk-logs'
import { ExportResultCode, type ExportResult } from '@opentelemetry/core'

const IS_ENABLED = process.env.NUXT_OTEL_ENABLED === 'true'

export function getTracer(name: string): Tracer {
  if (!IS_ENABLED) {
    return trace.getTracer('noop-tracer')
  }
  return trace.getTracer(name)
}

export function getLogger(name: string): Logger {
  if (!IS_ENABLED) {
    return logs.getLogger('noop-logger')
  }
  return logs.getLogger(name)
}

function getLokiLevel(severityNumber?: number): string {
  if (!severityNumber) return 'info'
  if (severityNumber >= 21) return 'critical' // FATAL
  if (severityNumber >= 17) return 'error' // ERROR
  if (severityNumber >= 13) return 'warning' // WARN
  if (severityNumber >= 9) return 'info' // INFO
  return 'debug' // DEBUG/TRACE
}

export class LokiLogExporter implements LogRecordExporter {
  private url: string

  constructor() {
    this.url = process.env.NUXT_OTEL_LOKI_PUSH_URL || 'http://localhost:3100/loki/api/v1/push'
  }

  export(logsBatch: LogRecord[], resultCallback: (result: ExportResult) => void): void {
    if (logsBatch.length === 0) {
      resultCallback({ code: ExportResultCode.SUCCESS })
      return
    }

    const streams: Record<string, [string, string][]> = {}

    for (const log of logsBatch) {
      const level = getLokiLevel(log.severityNumber)
      const serviceName = log.resource.attributes['service.name']?.toString() || 'ai-code'

      const streamKey = JSON.stringify({
        job: serviceName,
        level
      })
      // we can't cleanly mix all log attributes into labels if they're high cardinality,
      // but for simplicity we will put them in labels for now, or just limit to static ones.
      // Let's actually put arbitrary attributes into the json body to prevent high cardinality issues.

      if (!streams[streamKey]) {
        streams[streamKey] = []
      }

      const ns = (log.hrTime[0] * 1e9 + log.hrTime[1]).toString()
      const linePayload: Record<string, unknown> = {
        message: log.body,
        attributes: log.attributes
      }

      if (log.spanContext?.traceId) {
        linePayload.trace_id = log.spanContext.traceId
      }
      if (log.spanContext?.spanId) {
        linePayload.span_id = log.spanContext.spanId
      }

      streams[streamKey].push([ns, JSON.stringify(linePayload)])
    }

    const payload = {
      streams: Object.entries(streams).map(([key, values]) => ({
        stream: JSON.parse(key),
        values
      }))
    }

    fetch(this.url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    }).then((res: Response) => {
      if (res.ok) {
        resultCallback({ code: ExportResultCode.SUCCESS })
      } else {
        resultCallback({ code: ExportResultCode.FAILED, error: new Error(`Loki returned ${res.status}`) })
      }
    }).catch((err: Error) => {
      resultCallback({ code: ExportResultCode.FAILED, error: err })
    })
  }

  shutdown(): Promise<void> {
    return Promise.resolve()
  }

  forceFlush(): Promise<void> {
    return Promise.resolve()
  }
}
