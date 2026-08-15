# Plan 035 Phase 2 live canary rerun — failed

Date: 2026-08-15. This was a real production build (`pnpm build`) started
with `node --import ./otel-preload.mjs .output/server/index.mjs`, on an
isolated port with `NUXT_DATABASE_URL` pointed at `127.0.0.1:1`. No database,
auth bypass, credentials, cookies, or secrets were changed.

## Runtime reproduction

`POST /api/auth/register` received normal registration fields containing the
three Plan 035 canary values. The first request lacked `confirm` and returned
422; the corrected request reached the real database adapter and returned an
uncaught 500 from the unreachable database.

- request: `POST /api/auth/register`
- status: `500`
- request ID: `7cdae982-3580-4d73-a661-6cba4e2025eb`
- trace ID: `a8f94b7da6f90a55c5d49997328652a3`
- response: generic RFC problem body with request ID only
- production artifact: `.output/server/index.mjs` built from the current worktree

The canaries were absent from the response body and headers, the captured
stdout excerpt, the Loki lifecycle record, and the Jaeger trace. The response
headers contain only the safe `x-request-id` correlation header among the
request-specific response metadata.

## Required operator-field result

The live Loki lifecycle record retained `request.id`, `trace_id`, `span_id`,
`operation`, `http.response.status_code`, and `outcome`; Jaeger retained the
HTTP operation and error/status tags. This is useful, bounded telemetry.

The required error-specific `error.type` and `error.classification` record
was not present in the live Loki query for this request. Queries for the
request ID, error stream, and `[unhandled]` returned no error record. Because
the acceptance requires proving classification/type as well as the lifecycle
fields, this rerun is **FAIL**, not a canary-security PASS.

## Supporting artifacts

- `phase2-rerun-20260815.headers.txt`
- `phase2-rerun-20260815.body.json`
- `phase2-rerun-20260815-loki.json`
- `phase2-rerun-20260815-jaeger.json`
- `phase2-rerun-20260815-stdout.txt`

