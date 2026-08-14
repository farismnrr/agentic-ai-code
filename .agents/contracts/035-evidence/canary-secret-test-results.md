# Plan 035 Phase 11 — Case 4/6: Canary secret negative test

Marker used: `canary-secret-fake-token-DO-NOT-LEAK-12345`, sent as a fake `Authorization: Bearer`
header value AND as fake `password`/`cookie`/`api_key`/`prompt`/`authorization` attribute values.

## Send 1 — invalid event name (no `event.name` attribute at all)

```
POST /api/telemetry  (Authorization: Bearer <marker>, attributes: {password, cookie, api_key, prompt})
-> 400 Unknown telemetry event name
```
Rejected before any attribute ever reached the logging pipeline.

## Send 2 — valid event name (`chat.stream.start`), all forbidden keys present

```
POST /api/telemetry  (Authorization: Bearer <marker>,
  attributes: {event.name: "chat.stream.start", password, cookie, api_key, prompt, authorization: <marker>})
-> 200 {"success":true}
```

Resulting Loki record (captured, saved as `canary-loki-raw.json`):

```json
{
  "message": "chat.stream.start",
  "attributes": {
    "service.name": "ai-code-frontend",
    "component": "frontend",
    "event.name": "chat.stream.start",
    "trace_id": "b32c6e56ab9a2925c4c9adb82bc8cc89",
    "span_id": "1216da9e8a797616"
  },
  "trace_id": "b32c6e56ab9a2925c4c9adb82bc8cc89",
  "span_id": "1216da9e8a797616"
}
```

Every forbidden key (`password`, `cookie`, `api_key`, `prompt`) was silently stripped by
`sanitizeAttributes()`'s allowlist chokepoint in `server/infrastructure/observability/logger.ts` ->
`server/infrastructure/observability/sanitize.ts` — only the safe vocabulary fields survive. The
raw `Authorization` header value (the marker) was never read/logged at all — the telemetry
endpoint only inspects the session cookie for auth, never the `Authorization` header.

## Send 3 — `scripts/verify-no-secret-leakage.sh` (rate-limited, 429, confirming rate limiting itself works — see `telemetry-endpoint-abuse-results.md`)

## Grep sweep for the marker across all captured evidence

Checked: every file under `.agents/contracts/035-evidence/` (excluding intentional-input files
named `*canary-request-body.*`/`*canary-input*`), the Rust relay's captured stderr
(`relay_stderr.log`), and Jaeger's raw trace dumps for both `ai-code-server` and `ai-code-relay`
services (`jaeger_nuxt_recent.json`, `jaeger_relay_recent.json`).

```
grep -rc "canary-secret-fake-token-DO-NOT-LEAK-12345" <all above> -> 0 matches everywhere except the one intentional-input capture (canary-request-body.txt)
```

`scripts/verify-no-secret-leakage.sh`'s own automated sweep also reported:
```
PASS: canary marker not found in any non-input evidence file
```

## Verdict: PASS — canary marker never appeared in any log, trace, or span outside the file that intentionally records the raw request we sent.
