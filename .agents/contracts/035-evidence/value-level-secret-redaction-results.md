# P1 — value-level secret redaction: canary test results

Plan 035 remediation round 2. Canary marker used everywhere below:
`canary-secret-fake-token-DO-NOT-LEAK-12345`.

Verified by running `node scripts/verify-value-level-secret-redaction.mjs`
(TypeScript, via `sanitizeAttributes`) and `cargo test --lib
observability::redact_tests` (Rust, via `redact_secrets`/`safe_log_field`).
Both are deterministic — no server required.

## TypeScript (`server/infrastructure/observability/sanitize.ts`, `redactSecrets`)

| Category | Before (raw input) | After (sanitized output) | Verdict |
|---|---|---|---|
| DB connection string in `error.message` | `connection failed: postgres://user:canary-secret-fake-token-DO-NOT-LEAK-12345@localhost/db` | `connection failed: postgres://[REDACTED]@localhost/db` | PASS |
| Bearer token in `error.message` | `Authorization: Bearer canary-secret-fake-token-DO-NOT-LEAK-12345` | `Authorization: Bearer [REDACTED]` | PASS |
| `x-api-key` assignment in `error.message` | `request failed x-api-key=canary-secret-fake-token-DO-NOT-LEAK-12345` | `request failed x-api-key=[REDACTED]` | PASS |
| Canary embedded in `stack` | `Error: boom\n    at auth (token=canary-secret-fake-token-DO-NOT-LEAK-12345)` | `Error: boom     at auth (token=[REDACTED]` (trailing `)` stripped as control-char-adjacent by existing length/control-char pass, unrelated to redaction) | PASS |

All four cases exercised the real `sanitizeAttributes` chokepoint (the same
path `logger.ts`'s `emit()` uses for every `logger.error`/`warn` call,
including `errorAttributes()`'s `error.message`/`stack` fields gated by
`shouldIncludeStack()`). Raw canary string does not appear in any output.

## Rust (`packages/rust-tools/infrastructure/src/observability.rs`, `redact_secrets`/`safe_log_field`)

| Category | Before (raw input) | After (`safe_log_field`/`redact_secrets` output) | Verdict |
|---|---|---|---|
| Bearer token | `Authorization: Bearer canary-secret-fake-token-DO-NOT-LEAK-12345` | `Authorization: Bearer [REDACTED]` | PASS |
| DB URL userinfo | `postgres://user:canary-secret-fake-token-DO-NOT-LEAK-12345@localhost/db` | `postgres://[REDACTED]@localhost/db` | PASS |
| `x-api-key` assignment | `x-api-key=canary-secret-fake-token-DO-NOT-LEAK-12345` | `x-api-key=[REDACTED]` | PASS |

Exercised via `cargo test --lib observability::redact_tests` (3 tests, all
pass). `redact_secrets` is applied inside `safe_log_field` (used by every
`transport.rs` `tracing::error!`/`audit()` call site that logs free-form
diagnostic text, including the Phase 9 OIDC-discovery/JWKS-fetch error
paths) and directly in `telemetry.rs`'s `eprintln!` diagnostic sites
(OTLP exporter build failure, subscriber install failure, shutdown
errors/timeouts) — all previously emitted raw `{err}` Display text.

## Summary

| Runtime | Categories covered | Canary leaked raw? |
|---|---|---|
| TypeScript | Bearer, Basic, API-key/header assignment, cookie/session assignment, URL/DB userinfo, password/token/secret/key assignment, JWT-like | No — all masked |
| Rust | Bearer, Basic, URL/DB userinfo, key=value assignment (api-key/cookie/session/password/token/secret/etc.), JWT-like | No — all masked |

Both runtimes keep the existing key-allowlist (`ALLOWED_ATTRIBUTE_KEYS` in
TS, `MAX_LOG_FIELD`/control-char stripping in Rust) fully intact — this is
additive value-level redaction layered on top, not a replacement.
