import { trace } from '@opentelemetry/api'

export const AI_TOOLS_TRACEPARENT_ENV = 'AI_TOOLS_TRACEPARENT'
const OTEL_ENABLED_ENV = 'NUXT_OTEL_ENABLED'
const OTEL_ENDPOINT_ENV = 'NUXT_OTEL_JAEGER_ENDPOINT'
const NODE_ENV_ENV = 'NODE_ENV'

const allowedNodeEnvironments = new Set(['development', 'test', 'production'])

function approvedTelemetryEnv(): Record<string, string> {
  const env: Record<string, string> = {}
  if (process.env[OTEL_ENABLED_ENV] === 'true') env[OTEL_ENABLED_ENV] = 'true'
  const endpoint = process.env[OTEL_ENDPOINT_ENV]
  if (endpoint && /^https?:\/\/[^\s]+$/.test(endpoint)) env[OTEL_ENDPOINT_ENV] = endpoint
  const nodeEnv = process.env[NODE_ENV_ENV]
  if (nodeEnv && allowedNodeEnvironments.has(nodeEnv)) env[NODE_ENV_ENV] = nodeEnv
  return env
}

/** Return only a valid W3C parent for the repository-owned ai-tools process. */
export function aiToolsTraceEnv(): Record<string, string> {
  const spanContext = trace.getActiveSpan()?.spanContext()
  if (!spanContext || !/^[0-9a-f]{32}$/.test(spanContext.traceId) || !/^[0-9a-f]{16}$/.test(spanContext.spanId)) return approvedTelemetryEnv()
  const flags = Number(spanContext.traceFlags) & 1
  return {
    ...approvedTelemetryEnv(),
    [AI_TOOLS_TRACEPARENT_ENV]: `00-${spanContext.traceId}-${spanContext.spanId}-${flags.toString(16).padStart(2, '0')}`
  }
}
