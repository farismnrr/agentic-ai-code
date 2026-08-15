# Plan 035 Phase 11 — Case 5: /api/telemetry abuse acceptance

Ran `scripts/verify-telemetry-endpoint-security.sh` against the live app at
`http://localhost:3334` with a real authenticated session cookie (from the Phase 11 test user).
Raw output saved as `telemetry-endpoint-abuse-raw.log`.

## Result summary

| Case | Expected | Result |
|---|---|---|
| (a) unauthenticated | 401 | PASS |
| (b) unknown event name | 400 | PASS |
| (c) unknown/extra attribute keys | stripped, not forwarded verbatim | PASS (400 at parse; confirmed via canary test that even when accepted, unknown keys never reach Loki — see `canary-secret-test-results.md`) |
| (d) malformed `trace.id`/`request.id` | discarded, not trusted | PASS (400 at parse; `buildAttributes()` in `server/api/telemetry.post.ts` regex-validates `trace.id`/`span.id`/`request.id` and silently drops non-conforming values rather than trusting them — confirmed by source read) |
| (e) oversized single field (>512 chars) | 400 | PASS (was 500 before fix, see below) |
| (f) oversized batch (60 records) | 400 | PASS (was 500 before fix, see below) |
| (g) >20 req/min same user | 429 | PASS (was misclassified as 500 before fix, see below) |

Final run: **7 passed, 0 failed**.

## Two genuine bugs found and fixed in `server/api/telemetry.post.ts` (Phase 11 finding)

1. **Rate limit misclassified as 500, not 429.** The handler threw a raw
   `createError({ statusCode: 429, statusMessage: 'Too Many Requests', ... })` instead of the
   existing `tooManyRequests()` RFC-9457 helper. The global error handler
   (`server/core/errors/index.ts`) only trusts status codes that either went through `problem()`
   (`isProblem`) or are in its `TRUSTED_TITLES` allowlist (400/401/403/404/405/409 — 429 was not
   present); anything else falls through to the generic `Internal Server Error` / status 500
   path. A legitimate rate-limit denial was therefore returned to the client as a `500` with a
   misleading `Too Many Requests` status line but `500` status code and body — silently breaking
   the plan's `outcome: rate_limited` classification (Definition of Done references normalized
   outcomes including `rate_limited`). Fixed by switching to `tooManyRequests(limited.retryAfter)`.
2. **Oversized field/batch produced an unhandled 500, not a 400.** `v.parse(telemetrySchema, ...)`
   throws a `ValiError` (not an H3 error) on validation failure, which is not caught anywhere in
   the handler, so it fell through to the global handler's "unhandled exception" branch (correct
   sanitization, but wrong status code — 500 instead of the intended 400). Fixed by wrapping the
   `v.parse` call in a `try { ... } catch { throw createError({ statusCode: 400, ... }) }`.

Both fixes are minimal, confined to `server/api/telemetry.post.ts` (already a Plan 035 Phase 5
file), and `pnpm verify:commit` was re-run after applying them (see final report).

## Verdict: PASS (after fixing 2 genuine pre-existing defects found during this acceptance pass).
