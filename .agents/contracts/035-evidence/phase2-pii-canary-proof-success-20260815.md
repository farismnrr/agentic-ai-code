# Plan 035 Phase 2 live canary proof — PASS

Fresh genuine runtime proof after fixing unhandled-error lifecycle telemetry.
The app was built with `pnpm build` and started from `.output/server/index.mjs`
with `node --import ./otel-preload.mjs` on an isolated port. Its only broken
dependency was its own `NUXT_DATABASE_URL` pointed at `127.0.0.1:1`; shared
Postgres and telemetry services were not modified.

## Reproduction

Normal registration input containing all three Plan 035 canaries was posted to
`/api/auth/register`. The real `userExists()` database query failed and the
uncaught exception produced HTTP 500.

- request ID: `eab2c035-4690-4441-bccd-3527dbf962ce`
- trace ID: `415865a0ff5aff2912ae135945df7150`
- status: `500`
- response: generic problem body with request ID only

## Required absence checks

The email, free-text/password, and person/name canaries were absent from:

- client response body and all captured response headers;
- full captured production stdout/consola output;
- the Loki record returned by querying the request ID;
- the Jaeger trace and span attributes;
- this committed evidence set.

No cookies, credentials, or raw request data were committed.

## Required operator fields

The live Loki record contains `request.id`, `trace_id`, `span_id`,
`error.type: Error`, `error.classification: unclassified`,
`http.response.status_code: 500`, `outcome: server_error`, and
`operation: http.request.lifecycle`. Jaeger resolves the same trace ID and
contains the POST operation, HTTP 500 status, and `error: true`.

The fix adds only bounded error type/classification to the already-correlated
request lifecycle event. It never serializes `Error.message` or `Error.stack`.

Result: **PASS**.

