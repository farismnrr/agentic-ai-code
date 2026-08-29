#!/usr/bin/env node
// Plan 035 P1/P2 — MCP tool-result error confidentiality.
//
// The fix in server/api/mcp/index.ts wraps a real MCP SSE session (auth +
// long-lived transport), so a full live E2E trigger requires the whole app
// stack. This script instead proves the *exact* catch-block logic — copied
// verbatim from server/api/mcp/index.ts's CallToolRequestSchema handler —
// against deterministic canary failures shaped like the ones named in the
// task: filesystem-path detail, DB-style detail, provider-style detail, and
// a secret canary value. It asserts the client-visible MCP content array
// never contains the raw detail, and that the private telemetry sink
// receives the full raw cause.

const CANARY = 'sk-canary-SECRET-9f3a7c21'
const cases = [
  { label: 'filesystem-path', err: new Error(`ENOENT: no such file or directory, open '/home/deploy/ai-code/data/${CANARY}/settings.db'`) },
  { label: 'db-style', err: new Error(`SQLITE_CONSTRAINT: UNIQUE constraint failed: workspaces.path (conn=${CANARY})`) },
  { label: 'provider-style', err: new Error(`upstream provider request failed: 401 invalid api key ${CANARY} for https://provider.example/v1/chat/completions`) },
  { label: 'secret-canary-only', err: new Error(`leaked secret token=${CANARY}`) }
]

// --- verbatim reproduction of the fixed catch block ---
function fakeTelemetry() {
  const calls = []
  return {
    calls,
    error(name, errorCode, cause, safeAttributes = {}) {
      calls.push({ name, errorCode, cause, safeAttributes })
    }
  }
}

function handleToolCallError(telemetry, name, err) {
  telemetry.error('mcp.tool.call', 'mcp_tool_call_failed', err, { 'mcp.tool.name': name })
  return { content: [{ type: 'text', text: 'Tool execution failed' }], isError: true }
}
// --- end reproduction ---

let failed = false

for (const { label, err } of cases) {
  const telemetry = fakeTelemetry()
  const result = handleToolCallError(telemetry, 'update_workspace', err)

  const clientText = JSON.stringify(result)
  const leaked = clientText.includes(CANARY) || clientText.includes(err.message)
  const genericOnly = result.content.length === 1 && result.content[0].text === 'Tool execution failed' && result.isError === true

  const privateFired = telemetry.calls.length === 1
    && telemetry.calls[0].cause === err
    && telemetry.calls[0].errorCode === 'mcp_tool_call_failed'
    && telemetry.calls[0].cause.message.includes(CANARY)

  const ok = !leaked && genericOnly && privateFired
  console.log(`[${ok ? 'PASS' : 'FAIL'}] ${label}: client-leak=${leaked} generic-only=${genericOnly} private-fired=${privateFired}`)
  console.log(`  client-visible: ${clientText}`)
  if (!ok) failed = true
}

if (failed) {
  console.error('\nFAIL: at least one case leaked raw detail or did not reach private telemetry')
  process.exit(1)
}
console.log('\nPASS: all cases — generic message only on client, raw detail only via private telemetry.error()')
