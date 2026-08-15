#!/usr/bin/env node
// Plan 035 Phase 4 black-box acceptance: a mutable raw Error.name must not
// cross the console, Loki, request-lifecycle, or Jaeger exception boundaries.
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { randomUUID } from 'node:crypto'
import fs from 'node:fs'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const jaeger = process.env.JAEGER_QUERY_URL
const loki = process.env.LOKI_QUERY_URL
const lokiPush = process.env.LOKI_PUSH_URL || loki?.replace(/\/loki\/api\/v1\/query_range\/?$/, '/loki/api/v1/push')
const lokiQuery = loki?.endsWith('/query_range') ? loki : `${loki}/loki/api/v1/query_range`
const otlp = process.env.JAEGER_OTLP_ENDPOINT || process.env.NUXT_OTEL_JAEGER_ENDPOINT
if (!jaeger || !loki || !lokiPush || !otlp) throw new Error('JAEGER_QUERY_URL, LOKI_QUERY_URL, and JAEGER_OTLP_ENDPOINT are required')

const requestId = `phase4-${randomUUID()}`
const name = 'USER_CONTROLLED_ERROR_TYPE_CANARY_035'
const forbidden = 'phase4-forbidden-message-DO-NOT-LEAK'
const sourcePath = '/private/phase4/forbidden/source.ts'
const operation = 'phase4.raw-error.canary'
const child = join(root, 'node_modules', '.tmp-phase4-raw-error-canary.mjs')
const source = `
import { context, trace } from '@opentelemetry/api'
import { NodeTracerProvider, SimpleSpanProcessor } from '@opentelemetry/sdk-trace-node'
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-grpc'
import { LoggerProvider, SimpleLogRecordProcessor } from '@opentelemetry/sdk-logs'
import { logs } from '@opentelemetry/api-logs'
import { resourceFromAttributes } from '@opentelemetry/resources'
import { ATTR_SERVICE_NAME } from '@opentelemetry/semantic-conventions'
import { LokiLogExporter } from '${join(root, 'server/infrastructure/observability/otel.ts')}'
import { logger } from '${join(root, 'server/infrastructure/observability/logger.ts')}'
import { recordSanitizedException } from '${join(root, 'server/infrastructure/observability/exception.ts')}'
import { beginRequestLifecycle, recordRequestLifecycle } from '${join(root, 'server/infrastructure/observability/request-lifecycle.ts')}'
process.env.NUXT_OTEL_ENABLED = 'true'
process.env.NUXT_OTEL_LOKI_PUSH_URL = '${lokiPush}'
const resource = resourceFromAttributes({ [ATTR_SERVICE_NAME]: 'ai-code-server' })
const provider = new NodeTracerProvider({ resource, spanProcessors: [new SimpleSpanProcessor(new OTLPTraceExporter({ url: '${otlp}' }))] })
provider.register()
logs.setGlobalLoggerProvider(new LoggerProvider({ resource, processors: [new SimpleLogRecordProcessor({ exporter: new LokiLogExporter() })] }))
const span = trace.getTracer('phase4-acceptance').startSpan('${operation}')
const err = new Error('${forbidden}')
err.name = '${name}'
err.stack = '${name}: ${forbidden}\\n    at ${sourcePath}:35:7'
const event = { method: 'GET', path: '/api/phase4-canary?secret=${forbidden}', context: { requestId: '${requestId}', matchedRoute: { route: '/api/phase4-canary' } }, node: { res: { statusCode: 500 } } }
beginRequestLifecycle(event)
await context.with(trace.setSpan(context.active(), span), async () => {
  logger.error('phase4 raw error', err)
  recordSanitizedException(span, err)
  recordRequestLifecycle(event, 500, err)
})
span.end()
await provider.forceFlush()
await new Promise(resolve => setTimeout(resolve, 500))
console.log(JSON.stringify({ requestId: '${requestId}', operation: '${operation}' }))
`
fs.writeFileSync(child, source)
let stdout
try {
  stdout = execFileSync('npx', ['tsx', child], { cwd: root, encoding: 'utf8', env: { ...process.env, NUXT_OTEL_ENABLED: 'true' } })
} finally {
  fs.rmSync(child, { force: true })
}
if (stdout.includes(name) || stdout.includes(forbidden) || stdout.includes(sourcePath)) throw new Error('stdout/consola leaked raw exception data')

const query = async (url) => {
  const response = await fetch(url)
  if (!response.ok) throw new Error(`query failed: ${response.status}`)
  return response.text()
}
let lokiPayload = ''
for (let attempt = 0; attempt < 15; attempt++) {
  lokiPayload = await query(`${lokiQuery}?query=%7Bjob%3D%22ai-code-server%22%7D%20%7C%3D%20%22${requestId}%22&limit=20`)
  if (lokiPayload.includes(requestId)) break
  await new Promise(resolve => setTimeout(resolve, 300))
}
if (!lokiPayload.includes(requestId)) throw new Error('Loki returned no correlated request lifecycle record')
if (lokiPayload.includes(name) || lokiPayload.includes(forbidden) || lokiPayload.includes(sourcePath)) throw new Error('Loki leaked raw exception data')

const jaegerPayload = await query(`${jaeger.replace(/\/$/, '')}/api/traces?service=ai-code-server&operation=${operation}&limit=20`)
if (!jaegerPayload.includes(operation)) throw new Error('Jaeger returned no canary span')
if (jaegerPayload.includes(name) || jaegerPayload.includes(forbidden) || jaegerPayload.includes(sourcePath)) throw new Error('Jaeger leaked raw exception data')
const traces = JSON.parse(jaegerPayload).data ?? []
const spans = traces.flatMap(trace => trace.spans ?? [])
const span = spans.find(candidate => candidate.operationName === operation)
const exception = span?.logs?.find(log => log.fields?.some(field => field.key === 'event' && field.value === 'exception'))
const fields = Object.fromEntries((exception?.fields ?? []).map(field => [field.key, field.value]))
if (!exception || !['Error', 'UnknownError'].includes(fields['exception.type'])) throw new Error('Jaeger exception type was not bounded')
if (fields['exception.message'] !== 'unclassified') throw new Error('Jaeger exception message was not classified')
console.log(JSON.stringify({ status: 'PASS', requestId, errorType: fields['exception.type'], classification: fields['exception.message'] }))
