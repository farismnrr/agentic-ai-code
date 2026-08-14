# Plan 035 Phase 11 — Case 1: Happy-path end-to-end trace proof

## Steps performed (real, against the running Docker stack at http://localhost:3334)

1. `POST /api/auth/register` with a fresh test account -> `201`, session cookie captured.
2. `POST /api/api-keys` (authenticated via that session cookie) -> created API key
   `aic_live_f09c...` (raw key shown once, captured).
3. `GET /api/sidebar` with `Authorization: Bearer aic_live_f09c...` (no cookie) -> exercises the
   API-key auth middleware path end to end.

## Client-visible response

```
HTTP/1.1 200 OK
x-request-id: bfc70fe1-3ecc-47f0-bc72-b46c2379cbae
content-type: application/json

{"workspaces":[...],"conversations":[]}
```

## Correlated private Loki record (query: `{job="ai-code-server"} | json`, filtered to this window)

```json
{
  "message": "auth.login",
  "attributes": {
    "service.name": "ai-code-server",
    "request.id": "bfc70fe1-3ecc-47f0-bc72-b46c2379cbae",
    "operation": "auth.login",
    "outcome": "ok",
    "auth.present": true,
    "trace_id": "441155ebd7cef414b4d9c84ea1364384",
    "span_id": "285f35a499efa73c"
  },
  "trace_id": "441155ebd7cef414b4d9c84ea1364384",
  "span_id": "285f35a499efa73c"
}
```

`request.id` in the log body matches the `x-request-id` response header exactly, proving the
Loki-lookup-by-`request.id` step of the documented operator flow.

## Jaeger trace for the correlated `trace_id`

`GET http://localhost:16686/api/traces/441155ebd7cef414b4d9c84ea1364384` returns a real trace
(`data[0].traceID == 441155ebd7cef414b4d9c84ea1364384`) containing a server span for this
`GET` request under service `ai-code-server`. Full raw response saved alongside this file
(`happy-path-jaeger-trace.json`); raw Loki query response saved as `happy-path-loki-raw.json`.

## Verdict: PASS — full requestId -> Loki -> trace_id -> Jaeger chain proven with real evidence, not fabricated.

## Related finding fixed during this run

This path was **broken** (500/401, `ReferenceError: useDb is not defined`) before this Phase 11
acceptance run — see `error-path-500-unhandled-exception.md` for the root cause and the minimal
import fixes applied (5 files) to make this happy path reachable at all.
