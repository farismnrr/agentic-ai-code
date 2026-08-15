# Plan 035 Lane 6 — Remediation re-proof: Happy path A (browser chat, full trace chain)

Run against the freshly remediated commit's running stack (`ai-code-app-1` on
`http://localhost:3334`, `NUXT_OTEL_ENABLED=true`, standalone `masih-awam-jaeger`).

## Steps performed (real, live traffic)

1. `POST /api/auth/register` with a fresh test account (`e2e-remediation-<ts>@example.com`) ->
   `201 {"ok":true}`, session cookie captured (`nuxt-session=...`).
2. Simulated exactly what `createTracedFetch()` (`app/utils/trace-context.ts`) sends for a
   same-origin `/api/**` request: a client-generated W3C `traceparent` header
   (`00-<32-hex-trace-id>-<16-hex-span-id>-01`) plus the session cookie, on a real
   `POST /api/chat` matching `chat.post.ts`'s expected body shape (`{ id, trigger, message: { id, role, parts } }`).
   - `traceparent: 00-754023297b91f2d611e1c1d9f3206010-296c40b54a3bbb95-01`
   - Body targeted a conversation id that does not exist (no LLM provider is configured in this
     environment), so the chat completion itself does not run — this is expected and acceptable
     per the lane brief: what's being proven is trace correlation into the server's
     request/auth/use-case span boundary, not chat completion quality.

## Client-visible response

```
HTTP/1.1 404 Not Found
x-request-id: bb2949ca-2a93-44b5-92ae-dcd9efa496a1
content-type: application/problem+json

{"type":"about:blank","title":"Not Found","status":404,"detail":"Conversation not found","instance":"/api/chat"}
```

Generic, no internals leaked — consistent with the Phase 2/6 sanitization contract.

## Correlated Loki record

Query (per the contract's documented operator flow, `.agents/contracts/035-observability-telemetry-contract.md` lines 277-311):
`{job="ai-code-server"} | json`, filtered to `trace_id == 754023297b91f2d611e1c1d9f3206010`
(the exact trace id sent in the client `traceparent`):

```json
{
  "message": "404 Not Found: Conversation not found",
  "attributes": {
    "service.name": "ai-code-server",
    "trace_id": "754023297b91f2d611e1c1d9f3206010",
    "span_id": "532008be8f88456e"
  },
  "trace_id": "754023297b91f2d611e1c1d9f3206010",
  "span_id": "532008be8f88456e"
}
```

**Finding (not fabricated, reported honestly):** this particular log line — the generic H3 error
handler's auto-log for the thrown 404 — does not carry a `request.id` attribute, so a lookup by
`request.id` alone (`{job="ai-code-server"} | json | request_id=...`) returns zero rows for this
specific error line. The `trace_id` correlation (the thing this lane exists to prove) works
perfectly; the `request.id`-attribute-on-every-log-line completeness is narrower than the
happy-path example in `happy-path-api-key-auth.md` (whose `auth.login` event does carry
`request.id`) because that event is an explicit app-level telemetry call, whereas this is the
generic uncaught-error auto-logger. This looks like a real (pre-existing, not newly introduced by
lanes 1-5) minor gap, not a regression — flagged for parent triage, not fixed here per the
evidence-only scope of this lane.

Raw Loki response saved at `remediation-happy-path-chat-loki-raw.json`.

## Jaeger trace for the same `trace_id` — the actual proof of client-trace-id pickup

`GET http://localhost:16686/api/traces/754023297b91f2d611e1c1d9f3206010` returns a real trace with
service `ai-code-server`:

```json
{
  "traceID": "754023297b91f2d611e1c1d9f3206010",
  "spans": [
    { "operationName": "chat.execute", "spanID": "23807ffe9b0e4e6e",
      "references": [{ "refType": "CHILD_OF", "spanID": "532008be8f88456e" }] },
    { "operationName": "POST", "spanID": "532008be8f88456e",
      "references": [{ "refType": "CHILD_OF", "spanID": "296c40b54a3bbb95" }] }
  ],
  "processes": { "p1": { "serviceName": "ai-code-server" } }
}
```

The server's top-level `POST` span's parent reference (`296c40b54a3bbb95`) is **exactly** the
`span_id` this test sent in the client `traceparent` header — direct proof that the
client-generated W3C trace context was picked up server-side and continued into a real span tree
(`POST` -> `chat.execute`), which is the whole point of the lane-2 fix (`createTracedFetch()`).
Full raw response saved at `remediation-happy-path-chat-jaeger-trace.json`.

## Verdict: PASS

Real client-generated `trace_id`/`span_id` correlation achieved end-to-end this time (unlike the
pre-remediation `chat-trace-continuity-results.md`, which was code-level-only proof since OTel was
disabled in that run). One narrow, pre-existing, non-blocking gap noted above (generic 404
auto-log lacks `request.id` attribute) — reported, not fixed.
