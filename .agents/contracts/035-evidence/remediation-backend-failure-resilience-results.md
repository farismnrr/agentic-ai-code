# Plan 035 Lane 6 — Remediation re-proof: Backend failure resilience

## Steps performed

1. `docker stop masih-awam-jaeger` (the OTLP/Jaeger export backend for the running
   `ai-code-app-1` container, `NUXT_OTEL_ENABLED=true`).
2. With Jaeger stopped, exercised three endpoint classes on `http://localhost:3334`:
   - `POST /api/chat` (real chat request) -> `404` in ~0.017s (fast, expected app-level 404 for a
     non-existent conversation id — not a hang, not a 5xx).
   - `POST /api/telemetry` (frontend telemetry envelope) -> `401` in ~0.028s (fast, expected —
     unauthenticated request; not a hang, not a 5xx).
   - `GET /api/sidebar` (normal authenticated CRUD read) -> `200` in ~0.019s (fast, normal
     success response).
3. `docker start masih-awam-jaeger` -> confirmed back up (`Up 3 seconds`) and serving
   `GET /api/services` -> `{"data":["ai-code-server"], ...}` again within a few seconds.

## Result

All three endpoint classes responded normally and fast (tens of milliseconds, no timeouts, no
5xx) with the trace-export backend completely down — the app does not block/hang/fail request
handling when Jaeger is unreachable, consistent with Phase 7's fail-closed-on-propagation-but-
never-blocking-on-export design. `ai-code-app-1` required no restart to recover once Jaeger came
back.

## Verdict: PASS

Backend failure resilience re-confirmed on the remediated commit with real traffic and a real
stop/start cycle of the trace-export backend.

## State on completion

`ai-code-app-1` and `masih-awam-jaeger` both left running, as instructed, for parent final review.
