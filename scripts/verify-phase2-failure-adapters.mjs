#!/usr/bin/env node
// Plan 035 remediation round 3 Phase 2 black-box failure-adapter acceptance.
// Exercises the real package adapters; no unit-test framework or copied
// implementation is used.
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const canary = 'phase2-canary-secret-DO-NOT-LEAK'
const script = `
import { runTerminalCommand } from '${join(root, 'packages/terminal-tool/src/index.ts')}'
import { createCurlTool } from '${join(root, 'packages/curl-tool/src/index.ts')}'
import { createSearxngSearchTool } from '${join(root, 'packages/searxng-search-tool/src/index.ts')}'
import { runLanggraphChat } from '${join(root, 'server/infrastructure/ai/langgraph/langgraph-chat.ts')}'
import { publicMcpToolFailure } from '${join(root, 'server/application/observability/public-tool-error.ts')}'
import { sanitizeAttributes } from '${join(root, 'server/infrastructure/observability/sanitize.ts')}'
const results = []
const privateDiagnostics = []
const mcpTelemetry = { error: (...args) => {
  const cause = args[2]
  const safe = sanitizeAttributes({ 'error.type': cause.name, 'error.message': cause.message })
  privateDiagnostics.push({ code: args[1], safe })
} }
const mcp = publicMcpToolFailure(mcpTelemetry, 'update_workspace', new Error('database token=${canary}'))
results.push(['mcp-body', JSON.stringify(mcp)])
const terminal = await runTerminalCommand({ command: 'echo', cwd: '/tmp/${canary}', assertSafeCommand: async () => { throw new Error('path=${canary}') } })
results.push(['terminal', terminal])
const curl = await createCurlTool({ assertSafeUrl: async () => { throw new Error('api-key=${canary}') } }).invoke({ url: 'https://example.com' })
results.push(['curl', curl])
const search = await createSearxngSearchTool({ baseUrl: 'http://127.0.0.1:1' }).invoke({ query: '${canary}' })
results.push(['search', search])
globalThis.useRuntimeConfig = () => ({ searxngBaseUrl: 'http://127.0.0.1:1' })
async function consumeLanggraph(baseModel) {
  let persisted
  const stream = runLanggraphChat({
    uiMessages: [{ role: 'user', parts: [{ type: 'text', text: '@search trigger-failure' }] }],
    baseModel,
    systemPrompt: undefined,
    cleanup: async () => {},
    onEnd: async parts => { persisted = parts }
  })
  const reader = stream.getReader()
  const chunks = []
  while (true) { const next = await reader.read(); if (next.done) break; chunks.push(next.value) }
  return { stream: chunks, persisted }
}
const langgraph = await consumeLanggraph({ stream: async function* () { throw new Error('provider path=${canary}') } })
results.push(['langgraph-stream', langgraph.stream])
results.push(['langgraph-persisted', JSON.stringify(langgraph.persisted)])
const body = JSON.stringify(results)
const generic = results.filter(([, value]) => value === 'Tool execution failed').length >= 2
  && body.includes('Tool execution failed')
const leaked = body.includes('${canary}') || body.includes('Error:') || body.includes('Search failed')
const persistedParts = langgraph.persisted || []
const dynamicGeneric = persistedParts.some(part => part.type === 'dynamic-tool' && part.output === 'Tool execution failed' && !part.errorText)
const privateCaptured = privateDiagnostics.some(({ code, safe }) => code === 'mcp_tool_call_failed'
  && safe['error.type'] === 'Error'
  && safe['error.message'] === 'database token=[REDACTED]')
console.log(JSON.stringify({ results, generic, dynamicGeneric, privateCaptured, leaked, verdict: generic && dynamicGeneric && privateCaptured && !leaked ? 'PASS' : 'FAIL' }))
process.exit(generic && dynamicGeneric && privateCaptured && !leaked ? 0 : 1)
`
const tmp = join(root, 'node_modules', '.tmp-phase2-failure-adapters.mjs')
const fs = await import('node:fs')
fs.writeFileSync(tmp, script)
try {
  execFileSync('npx', ['tsx', tmp], { cwd: root, stdio: 'inherit' })
} finally {
  fs.rmSync(tmp, { force: true })
}
