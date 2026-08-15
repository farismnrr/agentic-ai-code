# Plan 035 Lane 6 — Remediation re-proof: Error path B (502 / upstream failure)

## Steps performed (real, live traffic against `http://localhost:3334`)

1. Authenticated as the same test account from `remediation-happy-path-chat.md`.
2. `POST /api/providers` with `baseUrl: "http://127.0.0.1:1"` (unreachable/disallowed loopback
   target) -> `200`, provider created (`unreachable-test`).
3. `GET /api/providers/<id>/models` with a client `traceparent`
   (`00-f6871da2670013d7f3078390ed52465f-6cc3ae35f906c6e0-01`) -> triggers
   `providerPort.discoverModels` -> `provider.reachability_check` span -> `listProviderModels`
   throws (SSRF-guard rejects the loopback base URL) -> caught and rethrown as `badGateway()`
   (`server/infrastructure/composition/application.ts:122`).

## Client-visible response

```
HTTP/1.1 502 Bad Gateway
x-request-id: b4ff8f06-eb77-40f1-9c3f-3a60d3efb796
content-type: application/problem+json

{"type":"about:blank","title":"Bad Gateway","status":502,"instance":"/api/providers/c9cd3935-8523-4a07-a1c0-14a6690b2231/models","requestId":"b4ff8f06-eb77-40f1-9c3f-3a60d3efb796"}
```

Only `type`/`title`/`status`/`instance`/`requestId` — no cause text, no provider internals, no
SSRF-guard detail leaked to the client. Consistent with the Phase 2 sanitization contract.

## Correlated private Loki record (real diagnostic, operator-only)

Query: `{job="ai-code-server"} | json`, filtered to
`trace_id == f6871da2670013d7f3078390ed52465f` (the client-sent trace id):

```json
{
  "message": "502 Bad Gateway: Could not reach provider \"unreachable-test\": OpenAI-compatible provider base URL resolves to a disallowed address",
  "attributes": {
    "service.name": "ai-code-server",
    "error.type": "Error",
    "error.message": "Could not reach provider \"unreachable-test\": OpenAI-compatible provider base URL resolves to a disallowed address",
    "trace_id": "f6871da2670013d7f3078390ed52465f",
    "span_id": "277cc1bb33a54c38"
  },
  "trace_id": "f6871da2670013d7f3078390ed52465f",
  "span_id": "277cc1bb33a54c38"
}
```

The real cause (SSRF-guard rejection reason) is present here, privately, and nowhere in the
client-visible response — exactly the intended public/private split.

## Jaeger trace — span tree with error marking

`GET http://localhost:16686/api/traces/f6871da2670013d7f3078390ed52465f`:

| span | spanID | parent | status |
|---|---|---|---|
| `GET` | `277cc1bb33a54c38` | `6cc3ae35f906c6e0` (**the client-sent traceparent span id**) | ERROR |
| `provider.discover_models` | `df31ad97abb34c8e` | `277cc1bb33a54c38` | ERROR |
| `provider.reachability_check` | `d2d01f4c623a2236` | `df31ad97abb34c8e` | ERROR |

All three spans in the chain are marked `otel.status_code=ERROR`, and the top server span's parent
is exactly the client-generated span id — proving both trace continuity and correct error-status
propagation through the span tree for a real upstream-failure path. Full raw responses saved as
`remediation-error-path-502-loki-raw.json` / `remediation-error-path-502-jaeger-trace.json`.

## Verdict: PASS

Public/private split confirmed correct (generic body vs. detailed private Loki record); full
client-trace-id correlation into Jaeger with correct error-status spans, achieved with real OTel
export this time (container has `NUXT_OTEL_ENABLED=true`).
