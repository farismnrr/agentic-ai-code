# Plan 035 Phase 5 — live acceptance evidence

Proves: for a given inbound request, the returned `x-request-id` correlates
to exactly one structured Loki log record (`http.request.lifecycle`), whose
`trace_id` resolves to a real trace in the trace backend — for all four
journeys (success, 4xx, handled 5xx, raw/unhandled 5xx).

## Environment used

- App: this repo's real production build (`pnpm build` + `.output/server/index.mjs`,
  started with `node --import ./otel-preload.mjs`), not `pnpm dev`.
- Loki: `shared-loki` (already running shared infra), queried at
  `http://localhost:3101`.
- Trace backend: a real standalone `jaegertracing/all-in-one` container
  (`plan035-jaeger`), started on `masihawam-net` with alias `jaeger` (matches
  `docker-compose.yml`'s `NUXT_OTEL_JAEGER_ENDPOINT=http://jaeger:4317`), OTLP
  receiver reachable at `localhost:4317` for locally-run instances and
  `jaeger:4317` for the containerized app. Query API at `localhost:16686`.
  No mock/stub — genuine Jaeger, same historical pattern as a prior round of
  this plan.
- Two app instances were used, per Phase 5C:
  - **Normal instance** (port 3335): fully configured, real Postgres DB,
    used for success / 4xx / handled-5xx.
  - **Isolated broken-DB instance** (port 3336): identical build, only
    `NUXT_DATABASE_URL` repointed at `127.0.0.1:1` (nothing listening,
    `connect_timeout=2`) — an intentionally unreachable database dependency.
    Loki/Jaeger left reachable. Used only for the raw/unhandled-5xx journey.
    No shared/other-instance state touched; auth was not bypassed anywhere.

## Journeys

| Journey | Endpoint | Status | request_id | trace_id | Evidence files |
|---|---|---|---|---|---|
| success | `GET /` (port 3335) | 200 | `ca850616-60bf-401a-8da1-d3f6e36e0589` | `50c6ef25a92bdeca5a6c19479fec48d9` | `success.*` |
| 4xx | `POST /api/auth/register` with `{}` (port 3335, unauthenticated) | 422 | `063538c0-54c5-4d80-8394-c19c99fd43af` | `98e8d9610c4998689bc0781ed0a92c8e` | `4xx.*` |
| handled 5xx | `GET /api/providers/:id/models` (port 3335, authenticated session, provider `baseUrl` pointed at an unreachable/SSRF-blocked address) | 502 (`badGateway()`) | `8961be39-0eba-4995-b890-f06465db2959` | `a053b908fa9dfeb0617b747710add716` | `handled-5xx.*` |
| unhandled/raw 5xx | `POST /api/auth/register` (port 3336, broken-DB isolated instance, unauthenticated) | 500 (raw exception, not `problem()`) | `0845778d-6d00-4844-8a4c-0eb1af6c33a9` | `9a6833960bb15ee09ed22c1e8c1c5a32` | `unhandled-5xx.*` |

Each `*.headers.txt` is the real response headers (redacted of nothing —
these are already generic/safe by design). Each `*.loki-record.json` is the
exact structured Loki log line matched by `request.id`. Each
`*.jaeger-trace.json` is the real Jaeger trace for that record's `trace_id`,
trimmed to the interesting tags.

## Phase 5C: which endpoint/dependency, and why it's safe

- **Handled 5xx**: `POST /api/providers` (create) + `GET /api/providers/:id/models`
  (`server/api/providers/[id]/models.get.ts` → `discoverModels` in
  `server/infrastructure/composition/application.ts`). The provider's
  `baseUrl` was set to `http://127.0.0.1:1` — a real, deliberately
  unreachable/SSRF-guarded address, not a debug flag. The app's own SSRF
  guard classifies it as "resolves to a disallowed address" and the reachability
  check's `catch` wraps it in `badGateway(error, ...)`, producing a genuine
  `problem()`-based 502. Response body is fully generic
  (`{"type":"about:blank","title":"Bad Gateway","status":502,"instance":...,"requestId":...}`)
  — no host/URL/credential leaked. The private log line
  (see app log, not committed) reads `502 Bad Gateway: Could not reach
  provider "Phase5 Broken Provider": OpenAI-compatible provider base URL
  resolves to a disallowed address` — a sanitized failure classification,
  no credentials, no stack beyond redacted file paths. This required a real
  authenticated session (register + login), not an auth bypass. No shared
  infrastructure was touched — the "broken dependency" here is a
  user-owned provider row the test itself created.

- **Raw/unhandled 5xx**: `POST /api/auth/register`
  (`server/api/auth/register.post.ts`) on a **second, isolated** instance
  (port 3336) built from the same production artifact, with only
  `NUXT_DATABASE_URL` repointed at an unreachable host:port
  (`127.0.0.1:1`, `connect_timeout=2`). `userExists(email)` issues a real
  Postgres query which fails with a connection error; the handler does not
  catch it, so it propagates as a genuine uncaught exception through
  Nitro's global error handler (`server/core/errors/index.ts`), which forces
  the generic `{"type":"about:blank","title":"Internal Server Error",...,"requestId":...}`
  body (no DB host/credentials, no stack) on the client response. The
  private server log (`[unhandled] Failed query: select "id" from
  "ai_code"."users" where ...`, stack frames all `[REDACTED-PATH]`) shows the
  query text and submitted email but no connection string or credentials —
  `redactSecrets()`'s userinfo pattern (`postgres://user:pass@host` →
  `postgres://[REDACTED]@host`) and path redaction did their job. This does
  not touch the shared dev Postgres other instances/services depend on
  (`shared-postgres` was never pointed at) — only this isolated instance's
  own `NUXT_DATABASE_URL` was changed, and no auth/admission/sandbox check
  was weakened (registration remains a normal, unauthenticated, rate-limited
  endpoint; the failure happens on first DB access before any of that
  matters).

## `pnpm verify:commit` / build

Both were run against the fixed code (see `.agents/contracts/035-evidence/`
sibling report in the worker's final message for exact pass/fail and any
findings) — this file only documents live acceptance evidence, not gate
output.

## Known limitation found, not fixed (documented, not blocking)

`server/infrastructure/observability/sanitize.ts`'s filesystem-path
redaction regex (`/(?:[A-Za-z0-9._-]+\/){2,}[A-Za-z0-9._-]*/g` →
`[REDACTED-PATH]`) also matches ordinary multi-segment API route strings
(e.g. `/api/auth/register`), so the `route` attribute on the
`http.request.lifecycle` record is `[REDACTED-PATH]` for any nested route
(only `/` survives). This is over-redaction, not a leak — no security
weakening — but it defeats the `route` attribute's usefulness for anything
but the root path. Left unfixed: it lives in the shared sanitizer used by
every log call site (`error.message`, `stack`, etc.), so a regex change
needs its own scoped review rather than a Phase-5-driven edit under this
delegated task's file scope.
