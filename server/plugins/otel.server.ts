import { diag, DiagConsoleLogger, DiagLogLevel } from '@opentelemetry/api'
import { LokiLogExporter } from '../infrastructure/observability/otel'
import { logger } from '../infrastructure/observability/logger'
import { LoggerProvider, SimpleLogRecordProcessor } from '@opentelemetry/sdk-logs'
import { resourceFromAttributes } from '@opentelemetry/resources'
import { ATTR_SERVICE_NAME } from '@opentelemetry/semantic-conventions'
import { logs } from '@opentelemetry/api-logs'

// Tracing + HTTP instrumentation live in otel-preload.mjs (repo root),
// loaded via `node --import` before Nitro's own module graph starts —
// HttpInstrumentation must patch `node:http` before anything else imports
// it, and Nitro's entry does `import 'node:http'` before any plugin here
// ever runs, so registering it from inside the app is always too late.
// The logs pipeline has no such early-patch requirement, so it stays here.
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

  const logExporter = new LokiLogExporter()
  const loggerProvider = new LoggerProvider({
    resource,
    // Error-handler telemetry must be exported before the short-lived request
    // process can finish. A batch processor can leave the unhandled-error
    // record buffered while the lifecycle record is queried, hiding the
    // bounded error.type/error.classification fields from Loki.
    processors: [new SimpleLogRecordProcessor({ exporter: logExporter })]
  })
  logs.setGlobalLoggerProvider(loggerProvider)

  // Node-level warnings (process.emitWarning) — including the AI SDK's own
  // "AI SDK Warning System" (e.g. missing Gemini 3 thoughtSignature on
  // replayed tool calls) — only ever reached `docker compose logs` before
  // this. logger.warn() also goes through consola for stdout, so this
  // doesn't change what operators see locally, only what reaches Loki.
  process.on('warning', (warning) => {
    logger.forwardOnly(13, 'WARN', warning.message, { warningName: warning.name, stack: warning.stack })
  })
})
