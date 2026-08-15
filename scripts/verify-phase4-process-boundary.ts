/* eslint-disable @typescript-eslint/no-explicit-any */
import { context, trace } from '@opentelemetry/api'
import { execFile } from 'node:child_process'
import { AsyncLocalStorage } from 'node:async_hooks'
import { promisify } from 'node:util'
import { runTerminalCommand } from '../packages/terminal-tool/src/index.ts'
import { aiToolsTraceEnv } from '../server/infrastructure/observability/ai-tools-trace.ts'

const traceId = process.env.PHASE4_TRACE_ID ?? 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const spanId = 'bbbbbbbbbbbbbbbb'
const span = { spanContext: () => ({ traceId, spanId, traceFlags: 1, isRemote: false }) } as any
const active = trace.setSpan(context.active(), span)
const execFileAsync = promisify(execFile)
const storage = new AsyncLocalStorage<any>()
context.setGlobalContextManager({
  active: () => storage.getStore() ?? context.active(),
  with: (value: any, fn: any, ...args: any[]) => storage.run(value, fn, ...args),
  bind: (value: any, target: any) => target,
  enable: () => undefined,
  disable: () => undefined
} as any)

await context.with(active, async () => {
  process.env.NUXT_OTEL_ENABLED = 'true'
  process.env.NUXT_OTEL_JAEGER_ENDPOINT ??= 'http://localhost:4317'
  const env = aiToolsTraceEnv()
  if (env.AI_TOOLS_TRACEPARENT !== `00-${traceId}-${spanId}-01`) throw new Error('active context was not encoded')
  if (Object.keys(env).some(key => !['AI_TOOLS_TRACEPARENT', 'NUXT_OTEL_ENABLED', 'NUXT_OTEL_JAEGER_ENDPOINT', 'NODE_ENV'].includes(key))) throw new Error('unexpected child env')

  if (process.env.PHASE4_PRINT_ENV === '1') {
    for (const [key, value] of Object.entries(env)) console.log(`${key}=${value}`)
    return
  }

  const result = await runTerminalCommand({
    command: 'env',
    cwd: process.cwd(),
    assertSafeCommand: async () => {},
    getChildEnv: aiToolsTraceEnv
  })
  if (result.includes('AI_TOOLS_TRACEPARENT') || result.includes('NUXT_OTEL_')) throw new Error('arbitrary command inherited internal telemetry env')

  const malformed = await execFileAsync(process.execPath, ['-e', 'process.stdout.write(process.env.AI_TOOLS_TRACEPARENT ?? "")'], {
    env: { AI_TOOLS_TRACEPARENT: 'not-a-traceparent' }
  })
  if (malformed.stdout !== 'not-a-traceparent') throw new Error('malformed-channel setup failed')
})

if (process.env.PHASE4_PRINT_ENV !== '1') console.log('PASS: Node context boundary, strict child env, malformed channel, and arbitrary-command isolation')
