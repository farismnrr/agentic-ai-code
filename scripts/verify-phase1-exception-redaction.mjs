#!/usr/bin/env node
// Plan 035 remediation round 3 Phase 1 black-box acceptance.
// Calls the production helper through a real NodeTracerProvider/OTLP exporter
// and verifies the exported span through Jaeger's query API. Both endpoints
// are mandatory; this acceptance must never silently downgrade to local-only.
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const canary = 'phase1-canary-secret-DO-NOT-LEAK'
const otlpEndpoint = process.env.JAEGER_OTLP_ENDPOINT || process.env.NUXT_OTEL_JAEGER_ENDPOINT
const queryUrl = process.env.JAEGER_QUERY_URL
if (!otlpEndpoint || !queryUrl) throw new Error('JAEGER_OTLP_ENDPOINT (or NUXT_OTEL_JAEGER_ENDPOINT) and JAEGER_QUERY_URL are required')

const script = `
import { NodeTracerProvider, SimpleSpanProcessor } from '@opentelemetry/sdk-trace-node'
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-grpc'
import { resourceFromAttributes } from '@opentelemetry/resources'
import { ATTR_SERVICE_NAME } from '@opentelemetry/semantic-conventions'
import { recordSanitizedException } from '${join(root, 'server/infrastructure/observability/exception.ts')}'
const provider = new NodeTracerProvider({
  resource: resourceFromAttributes({ [ATTR_SERVICE_NAME]: 'ai-code-server' }),
  spanProcessors: [new SimpleSpanProcessor(new OTLPTraceExporter({ url: '${otlpEndpoint}' }))]
})
provider.register()
const tracer = provider.getTracer('phase1-acceptance')
const span = tracer.startSpan('phase1.exception.canary')
const err = new Error('provider failed password=${canary} path=/srv/private/workspace')
err.name = 'ProviderRequestError'
err.stack = 'ProviderRequestError: password=${canary}\\n    at /srv/private/workspace/chat.ts:42:7'
recordSanitizedException(span, err)
span.end()
await provider.forceFlush()
console.log(JSON.stringify({ exported: true, operation: 'phase1.exception.canary' }))
`
const tmp = join(root, 'node_modules', '.tmp-phase1-exception-canary.mjs')
const fs = await import('node:fs')
fs.writeFileSync(tmp, script)
try {
  execFileSync('npx', ['tsx', tmp], { cwd: root, stdio: 'inherit' })
} finally {
  fs.rmSync(tmp, { force: true })
}

const endpoint = `${queryUrl.replace(/\/$/, '')}/api/traces?service=ai-code-server&operation=phase1.exception.canary&limit=20`
const response = await fetch(endpoint)
if (!response.ok) throw new Error(`Jaeger query failed: ${response.status}`)
const payload = await response.text()
if (!payload.includes('phase1.exception.canary')) throw new Error('Jaeger query returned no Phase 1 canary span')
if (payload.includes(canary) || payload.includes('/srv/private/workspace')) throw new Error('Jaeger contains forbidden exception data')
if (!payload.includes('ProviderRequestError') || !payload.includes('provider failed')) throw new Error('Jaeger span lacks useful sanitized diagnostics')
console.log(JSON.stringify({ jaeger: 'PASS', operation: 'phase1.exception.canary' }))
