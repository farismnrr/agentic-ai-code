// Node --import preload (plan 022). HTTP auto-instrumentation must patch
// `node:http`/`node:https` before ANYTHING else imports them — Nitro's own
// entry (.output/server/index.mjs) does `import 'node:http'` at the very
// top, before any Nitro plugin (including server/plugins/otel.server.ts)
// ever runs, so registering HttpInstrumentation from inside the app itself
// is always too late. This file runs via `node --import ./otel-preload.mjs`
// (see Dockerfile CMD), executing before Nitro's module graph loads at all.
//
// Only the tracer + HTTP instrumentation live here, since those are the
// only pieces with this early-patch requirement. The logs pipeline
// (LoggerProvider + Loki export) has no such constraint and stays in
// server/plugins/otel.server.ts, which is the natural place for it.
if (process.env.NUXT_OTEL_ENABLED === 'true') {
  const { NodeTracerProvider, SimpleSpanProcessor } = await import('@opentelemetry/sdk-trace-node')
  const { OTLPTraceExporter } = await import('@opentelemetry/exporter-trace-otlp-grpc')
  const { resourceFromAttributes } = await import('@opentelemetry/resources')
  const { ATTR_SERVICE_NAME } = await import('@opentelemetry/semantic-conventions')
  const { HttpInstrumentation } = await import('@opentelemetry/instrumentation-http')
  const { registerInstrumentations } = await import('@opentelemetry/instrumentation')
  const { diag, DiagConsoleLogger, DiagLogLevel } = await import('@opentelemetry/api')

  diag.setLogger(new DiagConsoleLogger(), DiagLogLevel.ERROR)

  const resource = resourceFromAttributes({
    [ATTR_SERVICE_NAME]: process.env.NUXT_OTEL_SERVICE_NAME || 'ai-code-server',
    'service.version': '1.0.0',
    'deployment.environment': process.env.NODE_ENV || 'development'
  })

  const traceExporter = new OTLPTraceExporter({
    url: process.env.NUXT_OTEL_JAEGER_ENDPOINT || 'http://localhost:4317'
  })

  // SimpleSpanProcessor, not Batch — see server/plugins/otel.server.ts's
  // comment on the same decision for the logs side; the same
  // Nitro-externalized BatchSpanProcessor never actually exports here
  // either, confirmed live, regardless of constructor argument shape.
  const tracerProvider = new NodeTracerProvider({
    resource,
    spanProcessors: [new SimpleSpanProcessor(traceExporter)]
  })
  tracerProvider.register()

  registerInstrumentations({
    instrumentations: [new HttpInstrumentation()]
  })
}
