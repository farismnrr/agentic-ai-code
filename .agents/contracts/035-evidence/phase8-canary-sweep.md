# Phase 8 — Comprehensive Runtime Secret-Leakage Canary Sweep

All values below are fake, deterministic canaries (`CANARY-<category>-<suffix>`, suffix
`x7q9zK2p8`) — never real secrets. A raw canary is only expected/acceptable inside the
request I sent (fixture), never in captured output.

## Method

1. Direct execution of the real sanitizer modules (Node `tsx` importing
   `server/infrastructure/observability/sanitize.ts` as-is; a temporary Rust
   `cargo run --example` against `relay-infrastructure::observability::redact_secrets`,
   deleted after use) with every canary category as input — this is real code execution,
   not source reading.
2. Live HTTP requests against the running `ai-code-app-1` container
   (`http://localhost:3333`) with canaries in `Authorization: Bearer`, `x-api-key`,
   `Cookie`, and JSON body fields, then querying real `shared-loki`
   (`http://localhost:3101`) and real `plan035-jaeger` (`http://localhost:16686`) for the
   matching `request.id`/`trace_id`.
3. Live execution of the real `ai-tools curl` binary (`target/release/ai-tools`) with
   canaries embedded in the target URL/query string and headers, against unreachable and
   SSRF-blocked hosts, inspecting actual stdout/stderr (the real MCP/CLI tool-result
   surface).

## Findings and fixes (2 real leaks found and fixed)

### P1 — Node `redactSecrets()`: JSON quoted-key values not redacted

`server/infrastructure/observability/sanitize.ts`'s key=value pattern required the
separator (`:`/`=`) immediately after the bare key name. A JSON-shaped payload like
`{"apiKey":"CANARY-APIKEY-x7q9zK2p8"}` has a closing quote before the `:`, which the old
regex did not skip, so the whole match failed and the raw value survived untouched.
Reproduced live via `tsx` execution of the real module (not just reading source).

Fix: pattern now tolerates an optional `'`/`"` immediately after the key name, before the
separator:
```
/\b(x-api-key|api[-_]?key|...|key)['"]?\s*[:=]\s*['"]?[^\s'",;}]+/gi
```
Re-ran the sweep script after the fix — `apikey_json` case now redacts to
`{"apikey=[REDACTED]"}`.

### P1 (mirrored) — Rust `redact_secrets()`: same JSON quoted-key/value gap, plus a second, higher-severity leak in `ai-tools curl`

`packages/rust-tools/infrastructure/src/observability.rs`'s hand-rolled scanner had the
identical gap (quoted key, then separately the quoted *value* was never skipped either, so
even after handling the key-quote the value's opening quote made the match zero-length).
Fixed by skipping an optional `"`/`'` both right after the key name and right after the
separator, before scanning the value token.

While constructing a realistic trigger for this category, found a **second, independently
real leak**: `packages/rust-tools/cli/src/commands/curl.rs` (the `ai-tools curl`
subcommand — the actual tool-result surface returned to an MCP/local-relay caller)
formatted `reqwest::Error` directly into its `Error: ...` output string in several places
(`SSRF guard blocked request/redirect: {e}`, `Fetch Error: {e}`, response-body-read error).
`reqwest::Error`'s `Display` impl embeds the full request URL, so any credential-shaped
query parameter or canary in the URL leaked straight into the tool's stdout `Error:` line —
the same defect class Phase 7 fixed in `auth.rs`'s OIDC/JWKS fetch paths, but not yet
applied to `curl.rs`.

Live repro before fix:
```
$ ai-tools curl "https://nonexistent-canary-host-CANARY-x7q9zK2p8.invalid/path?api_key=CANARY-QUERYKEY-x7q9zK2p8" \
    -H "Authorization: Bearer CANARY-BEARER-x7q9zK2p8" --timeout 3000
Error: SSRF Error: SSRF guard blocked request/redirect: error sending request for url
(https://nonexistent-canary-host-canary-x7q9zk2p8.invalid/path?api_key=CANARY-QUERYKEY-x7q9zK2p8)
```
Raw canary present in the tool-result stdout — a genuine P1.

Fix: promoted the Phase 7 `classify_reqwest_error()` helper (previously private to
`auth.rs`) into the shared `relay-infrastructure::observability` module as a public
function (`auth.rs` now calls `crate::observability::classify_reqwest_error`), and used it
in `curl.rs`'s three `reqwest::Error` formatting sites so only a bounded static label
(`connect_failed`, `timeout`, etc.) is ever interpolated — never the error's `Display` text.

Live repro after fix:
```
$ ai-tools curl "https://nonexistent-canary-host-CANARY-x7q9zK2p8.invalid/path?api_key=CANARY-QUERYKEY-x7q9zK2p8" \
    -H "Authorization: Bearer CANARY-BEARER-x7q9zK2p8" --timeout 3000
Error: SSRF Error: SSRF guard blocked request/redirect: connect_failed
```
No canary present. Also re-verified the private-IP-literal SSRF-guard path
(`http://127.0.0.1:9999/CANARY-PATH-x7q9zK2p8?token=CANARY-TOKEN-x7q9zK2p8`) and the
IP-literal-blocked path (`http://10.0.0.5/CANARY-PATH2-...`) — both were already clean
(static messages, no interpolation) and remain clean.

## Category × surface matrix

| Category | Node sanitizer (`redactSecrets`, live `tsx` exec) | Rust sanitizer (`redact_secrets`, live `cargo run --example` exec) | Live HTTP response (Node app) | Live Loki record | Live Jaeger span/exception | `ai-tools curl` stdout/stderr (Rust tool-result) |
|---|---|---|---|---|---|---|
| `Bearer <token>` | PASS | PASS | PASS (401, no header echo) | PASS (no header captured at all) | PASS | PASS |
| `Basic <token>` (real base64 shape) | PASS | PASS | N/A (no Basic-auth code path exercised) | N/A | N/A | N/A |
| API key header (`x-api-key`) | PASS | PASS | PASS | PASS | PASS | PASS |
| API key in JSON body field | PASS (fixed) | PASS (fixed) | N/A — no endpoint reflects raw JSON bodies into logs found in scope; verified at sanitizer level, which is the chokepoint every log call site routes through | — | — | — |
| Cookie/session value | PASS | PASS | PASS (own session cookie only, no canary reflected) | PASS | PASS | N/A |
| password/token/secret-named field | PASS | PASS | PASS (`/api/auth/register` with `password=CANARY...` — 422 body contains no password) | PASS (route recorded, no password) | not queried (client-error path, no exception span) | PASS |
| JWT-like (`eyJ...`) | PASS | PASS | not separately HTTP-exercised (covered by sanitizer chokepoint) | — | — | PASS |
| DB connection string w/ userinfo | PASS | PASS | N/A — no live DB-connection-string-in-error path found/exercised in scope; verified at sanitizer level | — | — | N/A |
| URL with secret-shaped query param | PASS | PASS | N/A | — | — | PASS (fixed — see finding above) |
| Filesystem path | PASS (over-redaction persists, not a leak — `/api/auth/register` → `[REDACTED-PATH]` reproduced live via Loki, matches known Phase 5/7 documented limitation, still non-blocking) | N/A (Rust has no filesystem-path redaction pattern; Rust error paths avoid embedding raw paths by design per Phase 7, reconfirmed here) | PASS | PASS | — | PASS |
| OIDC/JWKS upstream detail (Rust) | N/A | Reconfirmed via fresh independent read of `auth.rs`: `classify_reqwest_error` (now shared, see fix above) still gates all four OIDC/JWKS `reqwest::Error` formatting sites; behavior unchanged from Phase 7, still correct | N/A | N/A | not re-exercised live this round (no fresh unreachable-issuer repro was re-run; Phase 7's evidence plus this round's source/behavior reconfirmation stand) | N/A |

MCP 200 tool-result surface (`server/api/mcp/index.ts`'s `CallToolRequestSchema` handler):
verified by code inspection, not a fresh live SSE MCP call (would require provisioning a
valid API key + SSE session, out of this pass's time budget) — `publicMcpToolFailure()`
(`server/application/observability/public-tool-error.ts`) already returns a static
`Tool execution failed` string on the 200 `content` array for every tool failure and routes
the real `cause` only through `telemetry.error(...)`, which flows through the same
`sanitizeAttributes`/`redactSecrets` chokepoint proven clean above. Structurally correct;
not independently re-proven live this round.

Persisted DB error/tool data, chat/UI stream output: not exercised live this round (no
authenticated browser session available in this pass); both paths funnel through the same
`server/core/errors` / `telemetry.error` chokepoints already proven clean at the sanitizer
level, and Phase 6 evidence previously proved browser→trace continuity structurally. Not
re-verified live here — reported honestly as unverified-this-round rather than assumed.

## Files changed

- `server/infrastructure/observability/sanitize.ts` — fixed JSON quoted-key redaction gap.
- `packages/rust-tools/infrastructure/src/observability.rs` — fixed matching quoted-key/value
  gap; added shared public `classify_reqwest_error()`.
- `packages/rust-tools/infrastructure/src/auth.rs` — now calls the shared
  `classify_reqwest_error` instead of a private duplicate.
- `packages/rust-tools/cli/src/commands/curl.rs` — three `reqwest::Error` formatting sites
  now use `classify_reqwest_error()` instead of raw `Display` interpolation.

## Verification commands run

- `pnpm exec tsx <canary sweep script>` (Node, real module exec) — before and after fix.
- `cargo run -p relay-infrastructure --example canary_check` (temporary example, deleted
  after use) — before and after fix.
- `cargo build --release -p cli` then direct `target/release/ai-tools curl ...` — before
  and after fix, three scenarios (unreachable host, private-IP literal, IP-literal
  blocked).
- `cargo fmt --all`, `cargo clippy --workspace --all-targets` (clean).
- `pnpm verify:commit` — PASS (repo-policy, agent-docs, architecture, lint incl.
  `cargo fmt --check`/clippy `-D warnings`, typecheck incl. `cargo check -D warnings`).

## Known non-blocking limitation (reconfirmed, not fixed this round)

`sanitize.ts`'s filesystem-path regex still over-redacts ordinary multi-segment route
strings (e.g. `/api/auth/register` → `[REDACTED-PATH]`), reconfirmed live via the
`register` Loki record above. This is over-redaction, not a leak, and remains the same
documented, scoped-out limitation from Phase 5/7.
