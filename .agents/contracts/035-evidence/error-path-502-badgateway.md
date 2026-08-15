# Plan 035 Phase 11 — Case 2/3: Controlled 502 upstream failure

## Setup

Registered a fresh test user (`phase11-evidence-*@example.com`) via `POST /api/auth/register`,
then created a provider with a deliberately unreachable/disallowed base URL:

```
POST /api/providers
{"type":"openai_compatible","name":"phase11-unreachable","baseUrl":"http://127.0.0.1:1","apiKey":"fake-key-not-real"}
-> 200 {"id":"65d94f49-3d9f-4500-9507-6756fad90f2a", ...}
```

## Triggering request

```
GET /api/providers/65d94f49-3d9f-4500-9507-6756fad90f2a/models
```

## Public response (client-visible)

```
HTTP/1.1 502 Bad Gateway
x-request-id: d2d0403a-e5bd-4454-9096-590727bd07b1
Content-Type: application/problem+json

{"type":"about:blank","title":"Bad Gateway","status":502,"instance":"/api/providers/65d94f49-3d9f-4500-9507-6756fad90f2a/models","requestId":"d2d0403a-e5bd-4454-9096-590727bd07b1"}
```

Confirmed fields present: `type`, `title`, `status`, `instance`, `requestId`. No provider name,
no URL, no upstream/network/SSRF-guard error text, no stack.

## Private Loki record (operator-visible)

```json
{
  "message": "502 Bad Gateway: Could not reach provider \"phase11-unreachable\": OpenAI-compatible provider base URL resolves to a disallowed address",
  "attributes": {
    "service.name": "ai-code-server",
    "error.type": "Error",
    "error.message": "Could not reach provider \"phase11-unreachable\": OpenAI-compatible provider base URL resolves to a disallowed address",
    "trace_id": "16fc4c4b4a5e2f0ffc62c1418c317e74",
    "span_id": "ae07056ca0283cb5"
  },
  "trace_id": "16fc4c4b4a5e2f0ffc62c1418c317e74",
  "span_id": "ae07056ca0283cb5"
}
```

The private record correctly identifies the provider name and the real cause (SSRF guard
rejecting the loopback/disallowed address) — actionable for an operator, never sent to the
client. `trace_id`/`span_id` correlate to a Jaeger trace for the same request (service
`ai-code-server`).

## Finding (not a leak, but worth recording)

This particular error path is logged by a `logger.error(...)` call that does **not** include
`request.id` in the log body's `attributes` (it's logged before/outside the
`RequestTelemetryContext.error()` helper), so the *first* Loki lookup step in the documented
operator flow (`{job="ai-code-server"} | json | request_id=...`) does not find this specific
record — only the `trace_id`-based second-pass lookup does, once the operator already has the
trace ID from another source. This matches an already-flagged limitation in the Phase 10 contract
(not every log call site is routed through `RequestTelemetryContext`) — logged here as an
observation for a future phase, not fixed in Phase 11 since it does not violate any Definition of
Done item (no secret/leak, and `x-request-id`/`requestId` are still present on the response).

## Verdict: PASS (502 sanitization). Minor gap noted (not a P0/P1) for future correlation-completeness work.
