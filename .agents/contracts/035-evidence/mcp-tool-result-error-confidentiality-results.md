# Plan 035 — MCP tool-result error confidentiality (P1/P2)

## Problem

`server/api/mcp/index.ts` (`CallToolRequestSchema` handler, in the `GET`/SSE
branch) caught any error thrown while dispatching a tool call and returned
the raw `err.message` directly inside a **200 OK** JSON-RPC MCP tool-result
body:

```ts
return { content: [{ type: 'text', text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true }
```

Because this is a 200 response (not a 5xx), none of the Phase 2/9 sanitized
`problem()`/`internal()` logic in `server/core/errors/http.ts` ever runs for
it — raw filesystem paths, DB errors, provider/network text, or any other
implementation detail thrown by a tool handler was returned verbatim to the
MCP client.

## Fix

`server/api/mcp/index.ts`, the `catch` block of the tool-dispatch handler:

```ts
} catch (err: unknown) {
  telemetry.error('mcp.tool.call', 'mcp_tool_call_failed', err, { 'mcp.tool.name': name })
  return { content: [{ type: 'text', text: 'Tool execution failed' }], isError: true }
}
```

- Client-visible body: stable, generic `'Tool execution failed'` text,
  `isError: true`, HTTP status unchanged (still 200 — MCP/JSON-RPC semantics
  preserved, no conversion to HTTP 500).
- Raw diagnostic (`err`, whatever it contains — path/DB/provider/secret
  text) goes only to `telemetry.error(...)`, the same request-scoped
  `RequestTelemetryContext` (`event.context.application.observability.request`)
  used by every other error path in this plan (Phase 1/6 pattern, e.g.
  `server/application/chat/persistence.ts:40`,
  `server/infrastructure/ai/ai-sdk-stream.ts:71`). This reaches the normal
  logger → Loki pipeline via `logger.error` in
  `server/infrastructure/observability/request-context.ts:55-57`, keyed by
  `request.id` for operator correlation with Jaeger traces.
- No request-ID was added to the client-visible MCP content array: the
  `text` content block is the only field the MCP tool-result content shape
  reasonably supports, and forcing a support ID into that string would be
  awkward/non-standard for this protocol shape. A generic message alone
  satisfies the requirement per the task's own fallback guidance.

## Audit of the rest of `server/api/mcp/**` and `server/infrastructure/mcp/**`

- `server/api/mcp/index.ts` — only one `defineEventHandler`; only one
  raw-error-into-200-body site existed (the one fixed above). No other
  tool-result-shaped response construction in this file.
- `server/infrastructure/mcp/*` (`mcp-tools.ts`, `test-server.ts`) — errors
  here are all `logger.error(...)` calls (private-only, never surfaced to
  an MCP client body); no `content:`/`isError:` tool-result construction in
  this directory. No fix needed there.

grep confirmation (before fix, single hit; after fix, zero remaining raw
`err.message` interpolations into a client body):

```
$ grep -rn "err instanceof Error" server/api/mcp server/infrastructure/mcp
(no matches after fix — the only occurrence was the one replaced above)
```

## Deterministic acceptance

Live E2E through this exact route requires a full authenticated MCP SSE
session (Bearer API key + long-lived transport + POST-back correlated by
`sessionId`), which is heavier than a curl-style check. Per the task's
fallback allowance ("or by directly testing the sanitization function/logic
in isolation with a canary input if a live end-to-end trigger isn't cheaply
available — code-level proof is acceptable"), verification was done by
reproducing the exact fixed catch-block logic verbatim in
`scripts/verify-mcp-tool-result-error-confidentiality.mjs` and driving it
with four deterministic canary failures, each embedding a fake secret
canary value (`sk-canary-SECRET-9f3a7c21`):

1. **filesystem-path-shaped**: `ENOENT: no such file or directory, open
   '/home/deploy/ai-code/data/<canary>/settings.db'`
2. **DB-style**: `SQLITE_CONSTRAINT: UNIQUE constraint failed:
   workspaces.path (conn=<canary>)`
3. **provider-style**: `upstream provider request failed: 401 invalid api
   key <canary> for https://api.openai.com/v1/chat/completions`
4. **secret-canary-only**: `leaked secret token=<canary>`

Run:

```
$ node scripts/verify-mcp-tool-result-error-confidentiality.mjs
[PASS] filesystem-path: client-leak=false generic-only=true private-fired=true
  client-visible: {"content":[{"type":"text","text":"Tool execution failed"}],"isError":true}
[PASS] db-style: client-leak=false generic-only=true private-fired=true
  client-visible: {"content":[{"type":"text","text":"Tool execution failed"}],"isError":true}
[PASS] provider-style: client-leak=false generic-only=true private-fired=true
  client-visible: {"content":[{"type":"text","text":"Tool execution failed"}],"isError":true}
[PASS] secret-canary-only: client-leak=false generic-only=true private-fired=true
  client-visible: {"content":[{"type":"text","text":"Tool execution failed"}],"isError":true}

PASS: all cases — generic message only on client, raw detail only via private telemetry.error()
```

For all four cases:

- The client-visible MCP content array (`JSON.stringify(result)`) contains
  neither the canary value nor the raw `err.message` — only the literal
  string `Tool execution failed`.
- The fake `RequestTelemetryContext.error(...)` was invoked exactly once
  per case, with `errorCode: 'mcp_tool_call_failed'` and `cause === err`
  (the full, unsanitized `Error` object, canary and all) — proving the
  sanitized classification (`mcp_tool_call_failed`) is what's queryable,
  while the raw diagnostic remains attached to `cause` for the logger to
  serialize server-side.

This is code-level proof (per the task's own acceptance clause); the
Docker/Jaeger/Loki stack from prior evidence runs was not re-verified live
in this pass since no `SESSION_COOKIE`/live server context was provided,
but the call path from `telemetry.error(...)` into `logger.error(...)` →
Loki is unchanged from the already-proven Phase 1/6 pattern used
elsewhere in this plan.

## Verification commands

```
$ pnpm verify:commit
commit-gate: OK — repository policy, architecture, lint, and typecheck passed; commit may proceed.

$ node scripts/verify-mcp-tool-result-error-confidentiality.mjs
PASS: all cases — generic message only on client, raw detail only via private telemetry.error()
```

## Client-visible response — before vs. after

Before:

```json
{"content":[{"type":"text","text":"Error: ENOENT: no such file or directory, open '/home/deploy/ai-code/data/settings.db'"}],"isError":true}
```

After:

```json
{"content":[{"type":"text","text":"Tool execution failed"}],"isError":true}
```

HTTP status in both cases: 200 (unchanged — MCP/JSON-RPC tool-result
semantics preserved).
