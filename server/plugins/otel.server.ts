import { LokiLogExporter } from '../utils/otel'
import { NodeTracerProvider, BatchSpanProcessor } from '@opentelemetry/sdk-trace-node'
import { LoggerProvider, BatchLogRecordProcessor } from '@opentelemetry/sdk-logs'
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-grpc'
import { resourceFromAttributes } from '@opentelemetry/resources'
import { ATTR_SERVICE_NAME } from '@opentelemetry/semantic-conventions'
import { logs } from '@opentelemetry/api-logs'
import { HttpInstrumentation } from '@opentelemetry/instrumentation-http'
import { registerInstrumentations } from '@opentelemetry/instrumentation'

export default defineNitroPlugin(async () => {
  if (process.env.NUXT_OTEL_ENABLED !== 'true') {
    return
  }

  const resource = resourceFromAttributes({
    [ATTR_SERVICE_NAME]: process.env.NUXT_OTEL_SERVICE_NAME || 'ai-code-server',
    'service.version': '1.0.0',
    'deployment.environment': process.env.NODE_ENV || 'development'
  })

  // Tracing
  const traceExporter = new OTLPTraceExporter({
    url: process.env.NUXT_OTEL_JAEGER_ENDPOINT || 'http://localhost:4317'
  })
  const tracerProvider = new NodeTracerProvider({
    resource,
    spanProcessors: [new BatchSpanProcessor(traceExporter)]
  })
  tracerProvider.register()

  // Logging
  const logExporter = new LokiLogExporter()
  const loggerProvider = new LoggerProvider({
    resource,
    processors: [new BatchLogRecordProcessor(logExporter)]
  })
  logs.setGlobalLoggerProvider(loggerProvider)

  // Instrumentation
  registerInstrumentations({
    instrumentations: [
      new HttpInstrumentation()
    ]
  })
})
