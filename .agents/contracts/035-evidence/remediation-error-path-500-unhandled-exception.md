# Plan 035 Lane 6 — Remediation re-proof: Error path A (Nuxt 500, genuine unhandled exception)

## Steps performed (real, live traffic against `http://localhost:3334`)

1. Authenticated as the test account, created a provider (`openai_compatible`,
   `baseUrl: http://127.0.0.1:1`), a model under it, and a real conversation via
   `POST /api/conversations`.
2. Sent a real `POST /api/chat` with a client `traceparent`
   (`00-138966c35a13cf32c02b6053a91fd3b0-27d5796da5633a11-01`) against that conversation
   -> a genuine, unplanned `500`, not a synthetic/forced one.

## Client-visible response

```
HTTP/1.1 500 Internal Server Error
x-request-id: c694070c-7f45-4629-b2e2-431d4228e1c9
content-type: application/problem+json

{"type":"about:blank","title":"Internal Server Error","status":500,"instance":"/api/chat","requestId":"c694070c-7f45-4629-b2e2-431d4228e1c9"}
```

Fully generic — only `type`/`title`/`status`/`instance`/`requestId`. No stack trace, no error
message, no internal identifiers leaked, even though the underlying failure is a real
`ReferenceError`-class bug (see finding below).

## Correlated private Loki records

Query: `{job="ai-code-server"} | json`, filtered to `trace_id == 138966c35a13cf32c02b6053a91fd3b0`:

```json
{"message":"chat.stream.start","attributes":{"service.name":"ai-code-server","request.id":"c694070c-7f45-4629-b2e2-431d4228e1c9","operation":"chat.stream.start","outcome":"ok","provider.type":"openai_compatible","trace_id":"138966c35a13cf32c02b6053a91fd3b0","span_id":"6a73341e974d0ae6"}}
{"message":"[unhandled]","attributes":{"service.name":"ai-code-server","error.type":"Error","error.message":"runLanggraphChat is not defined","trace_id":"138966c35a13cf32c02b6053a91fd3b0","span_id":"eb3cbfc28e7dade7"}}
```

`request.id` in the `chat.stream.start` record matches the client `x-request-id` exactly. The
real cause (`runLanggraphChat is not defined`) is captured privately and never reached the client.

## Jaeger trace — error-marked span tree

`GET http://localhost:16686/api/traces/138966c35a13cf32c02b6053a91fd3b0`:

| span | spanID | parent | status |
|---|---|---|---|
| `POST` | `eb3cbfc28e7dade7` | `27d5796da5633a11` (client-sent traceparent span id) | ERROR |
| `chat.execute` | `6a73341e974d0ae6` | `eb3cbfc28e7dade7` | ERROR |

Client-generated trace id/span id correctly picked up server-side, full chain marked error.
Raw responses saved as `remediation-error-path-500-loki-raw.json` /
`remediation-error-path-500-jaeger-trace.json`.

## NEW bug found (reported, NOT fixed — out of scope for this evidence-only lane)

`server/infrastructure/ai/langgraph-stream.ts:29` calls `runLanggraphChat(...)` but the file has
**no import of `runLanggraphChat`** (the function is exported from
`server/infrastructure/ai/langgraph/langgraph-chat.ts:99`). This is a `ReferenceError` at runtime
in the production Docker build, of the exact same class as the previously-fixed `useDb`
auto-import gap documented in `error-path-500-unhandled-exception.md` (Nitro's
`.nuxt/tsconfig.json` type-check gate does not cover `server/**`, so `pnpm typecheck`/
`pnpm verify:commit` cannot catch it — this matches the known `ai-code-server-typecheck-gap`
class of issue). This makes the LangGraph chat-streaming path (agent mode / LangGraph-backed
chat) completely non-functional in production today. Not fixed here per this lane's strict
evidence-only scope — flagged for the parent to triage and fix (likely a one-line missing
`import { runLanggraphChat } from './langgraph/langgraph-chat'` in `langgraph-stream.ts`).

## Verdict: PASS (sanitization boundary holds); NEW bug reported separately (see above)

The 500 sanitization boundary correctly prevented any leak of this genuine unhandled exception's
detail to the client, and full private/trace correlation was captured — this is exactly Case-4-
style proof ("even a real, unplanned exception cannot bypass the sanitizer"), now re-confirmed
with live OTel export. The underlying bug is a functional defect, not a security or observability
regression, and is reported as a new finding rather than remediated in this lane.
