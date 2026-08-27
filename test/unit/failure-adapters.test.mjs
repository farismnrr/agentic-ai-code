#!/usr/bin/env node
// Failure-adapter confidentiality regression coverage using the real package adapters.
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const root = join(dirname(fileURLToPath(import.meta.url)), '../..')
const canary = 'phase2-canary-secret-DO-NOT-LEAK'
const script = `
import { runTerminalCommand } from '${join(root, 'packages/terminal-tool/src/index.ts')}'
import { createCurlTool } from '${join(root, 'packages/curl-tool/src/index.ts')}'
import { createSearxngSearchTool } from '${join(root, 'packages/searxng-search-tool/src/index.ts')}'
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
const body = JSON.stringify(results)
const generic = results.filter(([, value]) => value === 'Tool execution failed').length >= 2
  && body.includes('Tool execution failed')
const leaked = body.includes('${canary}') || body.includes('Error:') || body.includes('Search failed')
const privateCaptured = privateDiagnostics.some(({ code, safe }) => code === 'mcp_tool_call_failed'
  && safe['error.type'] === 'Error'
  && safe['error.message'] === 'database token=[REDACTED]')
console.log(JSON.stringify({ results, generic, privateCaptured, leaked, verdict: generic && privateCaptured && !leaked ? 'PASS' : 'FAIL' }))
process.exit(generic && privateCaptured && !leaked ? 0 : 1)
`
const tmp = join(root, 'node_modules', '.tmp-phase2-failure-adapters.mjs')
const fs = await import('node:fs')
fs.writeFileSync(tmp, script)
try {
  execFileSync('node', ['--experimental-strip-types', tmp], { cwd: root, stdio: 'inherit' })
} finally {
  fs.rmSync(tmp, { force: true })
}
