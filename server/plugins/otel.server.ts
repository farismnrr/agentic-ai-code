import { diag, DiagConsoleLogger, DiagLogLevel } from '@opentelemetry/api'
import { LokiLogExporter } from '../utils/otel'
import { NodeTracerProvider, SimpleSpanProcessor } from '@opentelemetry/sdk-trace-node'
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

  // A silently-failing telemetry pipeline is worse than none — a real
  // constructor-shape bug here previously broke log export with zero
  // indication anything was wrong. ERROR-level keeps this quiet in normal
  // operation; bump to DiagLogLevel.DEBUG locally to diagnose export issues.
  diag.setLogger(new DiagConsoleLogger(), DiagLogLevel.ERROR)

  const resource = resourceFromAttributes({
    [ATTR_SERVICE_NAME]: process.env.NUXT_OTEL_SERVICE_NAME || 'ai-code-server',
    'service.version': '1.0.0',
    'deployment.environment': process.env.NODE_ENV || 'development'
  })

  // Tracing
  const traceExporter = new OTLPTraceExporter({
    url: process.env.NUXT_OTEL_JAEGER_ENDPOINT || 'http://localhost:4317'
  })
  // BatchSpanProcessor was tried here for parity with the logs pipeline's
  // batching, but the version Nitro's build actually externalizes into
  // .output/server/node_modules never delivers spans to the exporter at
  // all — confirmed live: neither the positional nor the options-object
  // constructor form gets a span to Jaeger, and forceFlush() throws an
  // unhandled rejection. SimpleSpanProcessor (exports synchronously per
  // span-end, no batching) is the one that has actually been verified
  // end-to-end against the real Jaeger container. Correctness over
  // consistency with the logs pipeline's batching.
  const tracerProvider = new NodeTracerProvider({
    resource,
    spanProcessors: [new SimpleSpanProcessor(traceExporter)]
  })
  tracerProvider.register()

  // Logging
  const logExporter = new LokiLogExporter()
  const loggerProvider = new LoggerProvider({
    resource,
    processors: [new BatchLogRecordProcessor({ exporter: logExporter })]
  })
  logs.setGlobalLoggerProvider(loggerProvider)

  // Instrumentation
  registerInstrumentations({
    instrumentations: [
      new HttpInstrumentation()
    ]
  })
})
