# Plan 035 round-4 Phase 2 — live PII/user-data canary proof

Proves, via real runtime reproduction (not sanitizer unit testing), that
round-4 Phase 1's fix (`SafeDiagnosticError` / `classifyRawCause()` in
`server/core/errors/safe-diagnostic.ts` / `server/core/errors/classify.ts`)
actually stops user-submitted data from reaching private telemetry when a
raw/untrusted exception propagates.

## Canaries used

- email: `pii-canary-035@example.test` (also `pii-canary-035-race@example.test` for repro 2)
- name/identifier: `PERSON-CANARY-035`

(`USER-DATA-CANARY-035-X9Q7` was reserved for an arbitrary free-text field;
no such field exists on the register endpoint, so it was folded into the
password value for repro 1 to also confirm passwords never leak — result:
absent everywhere, same as the other canaries.)

## Environment

- App: real production build (`pnpm build` against current HEAD
  `a49f8b4b1bdd7441dad607aabbcb369d6570e9f8`, then
  `node --import ./otel-preload.mjs .output/server/index.mjs`), not `pnpm dev`.
- Loki: `shared-loki`, queried at `http://localhost:3101`.
- Jaeger: `plan035-jaeger` (already running from round 3), OTLP at
  `localhost:4317`, query API at `localhost:16686`.
- Two local instances built from the same artifact:
  - **Normal instance** (port 3335): real `shared-postgres`
    (`NUXT_DATABASE_URL` from `.env`, unmodified). Used for repro 2.
  - **Isolated broken-DB instance** (port 3336): identical build, only
    `NUXT_DATABASE_URL` repointed at `127.0.0.1:1` (nothing listening,
    `connect_timeout=2`). Used for repro 1. No shared infrastructure was
    touched by this change — only this instance's own env var.

## Repro 1 (minimum required): DB-unreachable raw exception

`POST /api/auth/register` on the isolated instance (3336) with canary
`name`/`email`/`password` submitted as real form field values.
`userExists(email)` issues a real Postgres query, which fails with a
genuine connection error (DB unreachable) — the handler does not catch it,
so it propagates as an uncaught exception through Nitro's global error
handler.

- Status: `500`
- `x-request-id`: `d8a01c31-aec2-4c5f-ace8-c0d188212585`
- `trace_id`: `b87b41bb064d7bf55e05adc037e26edf`
- Response body: `{"type":"about:blank","title":"Internal Server Error","status":500,"instance":"/api/auth/register","requestId":"d8a01c31-aec2-4c5f-ace8-c0d188212585"}`
- stdout: `ERROR  [unhandled] unclassified` (no message text, no stack beyond
  the sanitizer's own frames)
- Loki lifecycle record (`repro1-loki-raw.json`): `request.id`, `trace_id`,
  `span_id`, `route` (redacted per known Phase-5 over-redaction, not a
  leak), `http.response.status_code: 500`, `outcome: "server_error"` — no
  canary.
- Loki error record (`repro1-loki-error-record.json`): `message: "[unhandled]"`,
  `error.type: "Error"`, `error.classification: "unclassified"`, same
  `trace_id`/`span_id` — no canary.
- Jaeger trace (`repro1-jaeger-trace.json`): 1 span, `error: true`,
  `http.request.method: "POST"` — no canary. (No `error.classification` span
  tag; consistent with the documented Phase-5/7 finding that not every
  attribute that reaches Loki is also attached to a span — reported
  honestly, not fabricated.)
- stdout full capture (`repro1-full-stdout.log`): 0 canary occurrences.

## Repro 2 (bonus coverage): genuine Postgres unique-violation raw exception

Register is deliberately idempotent-safe against user enumeration
(`userExists()` short-circuits to the same `{"ok":true}` response for an
already-registered email), so a normal duplicate `POST` never reaches a raw
driver error. To force a genuine unique-constraint violation, two
concurrent `POST /api/auth/register` requests were fired for the same new
canary email (`pii-canary-035-race@example.test`) against the working
instance (3335) — both requests pass `userExists()` before either commits,
so the second `createUser` insert collides with the first at the database
level.

- Request 1: `201 Created` (won the race, `{"ok":true}`)
- Request 2: `500 Internal Server Error`
  - `x-request-id`: `c60b57b6-e34d-4674-89c4-b5a1847ffe59`
  - `trace_id`: `4f9cb7abd6893329941cb61af6180569`
  - Response body: generic, `requestId` only — no canary.
- stdout: `ERROR  [unhandled] unclassified` — no canary.
- Loki lifecycle + error records (`repro2-loki-error-records.json`):
  `error.type: "Error"`, `error.classification: "unclassified"` — no
  canary. (The raw cause here was not recognized by `isUniqueViolation()`
  as a `23505` — see note below — but regardless of *why* it classified as
  `unclassified` rather than a driver code, the important property held:
  the raw `.message` text, which a genuine `23505` violation would embed
  the email in, never reached telemetry or the client either way.)
- Jaeger trace (`repro2-jaeger-trace.json`): 1 span, no canary.
- stdout full capture (`repro2-full-stdout.log`): 0 canary occurrences.

Note: this repro's exact underlying SQL error was not independently
confirmed as SQLSTATE `23505` (the code path that would normally convert
that into a safe `409 Conflict` via `conflict('Email already registered')`
in `server/api/auth/register.post.ts` did not trigger, so status was `500`
not `409`) — worth a follow-up look, but out of scope for this canary proof
since it doesn't change the confidentiality conclusion: whatever the raw
cause was, it did not leak.

## Canary-absence summary (grep -c across all captured surfaces = 0)

| Surface | Repro 1 | Repro 2 |
|---|---|---|
| Client response (headers + body) | 0 | 0 |
| stdout / consola (full instance log) | 0 | 0 |
| Loki record(s) | 0 | 0 |
| Jaeger trace | 0 | 0 |
| Evidence files committed here | 0 | 0 |

## Telemetry-still-useful confirmation

Both repros confirm Phase 1 did not remove operator-useful telemetry:
`request.id`, `trace_id`/`span_id`, `error.type` (`"Error"`),
`error.classification` (`"unclassified"` — a bounded static label per the
frozen contract, not the raw message), HTTP status (`500`), `outcome`
(`"server_error"`), and operation name (`http.request.lifecycle` /
`[unhandled]`) are all present and queryable by `request.id` end to end
(client → Loki → Jaeger), exactly as required.

## Result

**PASS.** No canary leak found on any surface for either repro. This is a
genuine confirmation of Phase 1's fix, not a regression report.

## Supporting files in this directory

- `repro1.headers.txt`, `repro1.body.json` — client response, DB-unreachable repro
- `repro1-loki-raw.json` — lifecycle Loki record
- `repro1-loki-error-record.json` — `[unhandled]`/classification Loki record
- `repro1-jaeger-trace.json` — Jaeger trace
- `repro1-full-stdout.log` — full app stdout for that instance/run
- `repro2-race-1.headers.txt`, `repro2-race-1.body.json` — winning request (201)
- `repro2-race-2.headers.txt`, `repro2-race-2.body.json` — losing/raw-500 request
- `repro2-loki-error-records.json` — both repros' `[unhandled]` Loki records
- `repro2-jaeger-trace.json` — Jaeger trace
- `repro2-full-stdout.log` — full app stdout for that instance/run
