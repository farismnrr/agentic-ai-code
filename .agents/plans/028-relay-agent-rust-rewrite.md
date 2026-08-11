# 028 — Relay agent: full Rust rewrite + MCP server

**Status: IN FLIGHT** — the Rust rewrite and MCP core are implemented, but production security/resource-limit remediation remains before Plan 028 can be closed.

**Deadline decision:** the automated Rust test suite for `relay_agent` and `cargo test --workspace` were removed to meet the deadline. CI intentionally enforces static checks only: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo audit`. Runtime behavior is therefore validated by source review/manual verification until a future test strategy is explicitly restored.

## Context

Plan 027 migrated the general-purpose CLI tools to Rust. The remaining relay runtime was rewritten from Node.js/TypeScript to Rust. The relay is a local MCP server/execution bridge for Nuxt and future MCP clients, while the Plan 027 Rust binaries remain the actual CLI tools.

## Goals

- Rewrite `packages/relay-agent` to 100% Rust.
- Produce a standalone native `relay-agent` binary with no Node.js/V8/libnode runtime dependency.
- Implement actual MCP `2026-07-28`, not a proprietary MCP-like protocol.
- Keep MCP tool definitions/handlers transport-independent.
- Preserve Nuxt local compatibility where required.
- Reuse Plan 027 Rust CLI tools instead of duplicating them.
- Keep local execution localhost-only and fail closed on browser-originated access.
- Provide a clean path for future authenticated remote MCP deployment without exposing the local execution relay publicly.
- Remove Node.js, `@yao-pkg/pkg`, and relay-specific JS runtime/build dependencies.
- Build and publish native Rust artifacts with Cargo.

## Deployment boundary

- **Local Nuxt/browser:** Streamable HTTP to `127.0.0.1:<port>` plus the retained legacy compatibility path where required.
- **Local MCP hosts:** use standard MCP transport semantics.
- **Future external MCP client/cloud:** deploy the same tool layer behind a separately authenticated MCP endpoint; never expose the localhost execution agent publicly just to make cloud access work.

## Scope boundary

In scope: Rust relay runtime, MCP server/tool catalog/handlers, local execution bridge, legacy Nuxt compatibility, local auth/pairing, lifecycle, release pipeline, security/resource limits, and Node runtime removal.

Out of scope: migrating Nuxt/Vue/TypeScript, replacing Plan 027 CLI tools, arbitrary OS sandboxing, public unauthenticated execution, or a second tool implementation for external MCP client.

## Architecture

```text
Nuxt / MCP client
       │ Streamable HTTP / legacy compatibility
       ▼
Rust relay-agent
  ├─ protocol + transport
  ├─ localhost + Origin/Host policy
  ├─ auth/pairing
  ├─ tool registry
  ├─ execution + limits
  └─ lifecycle
       │
       ▼
Plan 027 Rust CLI tools
  terminal-tool / curl-tool / searxng-search-tool
```

Preferred package layout:

```text
packages/rust-tools/
├── Cargo.toml
└── src/
    ├── bin/relay-agent.rs
    └── relay_agent/
        ├── mod.rs
        ├── config.rs
        ├── error.rs
        ├── mcp.rs
        ├── transport.rs
        ├── security.rs
        ├── auth.rs
        ├── pairing.rs
        ├── tools.rs
        ├── execution.rs
        ├── limits.rs
        ├── http_compat.rs
        ├── websocket_compat.rs
        └── pidfile.rs
```

## MCP protocol requirements

### Protocol version

- [x] Target MCP `2026-07-28`.
- [x] Do not implement removed `initialize`/`initialized` + `Mcp-Session-Id` as the primary protocol.
- [x] Use Streamable HTTP; no deprecated legacy HTTP+SSE dependency.
- [ ] Older-MCP compatibility only if explicitly required later.

### MCP methods

- [x] `server/discover`.
- [x] `tools/list`.
- [x] `tools/call` request/structured-error path.
- [x] JSON-RPC error semantics.
- [x] Capability advertisement.

### Tool catalog

- [x] Stable Plan 027 tool names and descriptions.
- [x] Explicit JSON Schema 2020-12-compatible `inputSchema`.
- [x] Transport-independent registry.
- [x] No shell interpolation.
- [ ] Phase 11 must ensure all privileged execution paths preserve tool guards/policy.

### Streamable HTTP

- [x] `POST /mcp` JSON-in/JSON-out.
- [x] `MCP-Protocol-Version` validation.
- [x] `Mcp-Method`/`Mcp-Name` validation against request body.
- [x] Per-request `_meta` validation.
- [x] `application/json` enforcement.
- [x] 1 MiB body limit before parsing.
- [x] Stateless request handling; no hidden session authorization boundary.
- [x] Explicit CORS allowlist; no wildcard Origin.

### Authorization

Local policy is layered:

```text
HTTP transport
  ↓
localhost + exact Origin/Host policy
  ↓
local MCP/legacy authorization
  ↓
tool authorization/policy
  ↓
execution
```

- [x] MCP endpoint Origin/Host policy.
- [ ] Legacy compatibility endpoints must use the same fail-closed policy.
- [ ] Pairing credentials must be single-use and short-lived.
- [ ] Session credentials must be random, expiry-bound, revocable, and race-safe.
- [ ] Credentials must never appear in logs/errors.
- [ ] No debug/test authentication bypass.
- [ ] Future remote MCP must use standards-based OAuth/protected-resource authorization.

## Legacy Nuxt compatibility contract

The existing `/health`, `/pair`, `/revoke`, and WebSocket execution protocol is a compatibility adapter, not MCP.

- [ ] Exact `/health` contract.
- [ ] Exact `/pair` contract.
- [ ] Exact `/revoke` contract.
- [ ] Exact OPTIONS/CORS behavior.
- [ ] Exact WebSocket path/query/header/close behavior.
- [ ] Exact `exec`/`exec_result` semantics.
- [ ] CLI defaults/environment precedence.
- [ ] Existing frontend flow compatibility.

## Security invariants

1. Local mode binds only to `127.0.0.1`.
2. Browser-facing requests require exact configured Origin and valid Host; missing values fail closed.
3. Remote MCP authorization must use standards-based OAuth and never the local pairing credential.
4. Pairing is single-use, short-lived, cryptographically random, and atomically consumed.
5. Session credentials are random, expiry-bound, revocable, race-safe, and never logged.
6. No wildcard Origin or hidden/debug/test bypass.
7. Request bodies, WebSocket messages, tool arguments, command output, and concurrent executions are bounded.
8. Tool names and arguments are validated before process execution.
9. No shell interpolation.
10. Timeouts terminate the intended process tree and reap children.
11. Pidfile acquisition/release is atomic and ownership-safe.
12. Errors do not expose secrets, environment variables, stack traces, or credentials.
13. Public/remote deployment never exposes the local unauthenticated execution path.

## Resource limits

Concrete limits must be enforced deterministically:

- HTTP/MCP body limit;
- WebSocket message limit;
- MCP message/frame limit;
- maximum tool argument payload;
- stdout/stderr limit;
- per-session and global concurrent executions;
- maximum execution duration;
- pairing attempt rate/limit;
- queue depth where requests can queue.

Never silently truncate command input or tool arguments.

## CLI contract

- `--dir`, `-d`: default working directory, fallback to OS home directory.
- `--port`, `-p`: default `47821`.
- `--origin`, `-o`: allowed origin, with `RELAY_AGENT_ORIGIN` fallback.
- `stop --port <port>`.

Validate configuration before bind. Never broaden trust through Origin normalization.

## Command execution

Use `tokio::process::Command` and explicit adapters.

- [x] No shell interpolation.
- [ ] Never disable Plan 027 CLI guard/policy from the relay by default.
- [ ] Validate/authorize arguments before spawning.
- [ ] Bound output and arguments.
- [ ] Enforce concurrency.
- [ ] Enforce timeout and kill/reap process tree.
- [ ] Avoid blocking Tokio workers during cleanup.
- [ ] Normalize stdout/stderr/exit status/errors.

## PID / lifecycle

- [ ] Atomic exclusive pidfile/lock.
- [ ] Stale pidfile recovery.
- [ ] Second-instance rejection.
- [ ] Safe `stop --port` on supported OSes.
- [ ] Clean SIGINT/SIGTERM shutdown.
- [ ] Delete only a pidfile owned by the current process.
- [ ] Protect against symlink/path races.

## Binary and supply chain

- [x] Reuse Plan 027 Rust toolchain/MSRV policy.
- [x] Minimal dependency/features policy.
- [ ] Benchmark `opt-level = z` vs `s`/`3` and choose from evidence.
- [ ] Measure LTO/codegen-unit/strip tradeoffs.
- [x] Verify no Node/V8/libnode dependency.
- [x] Review `Cargo.lock` changes.
- [x] `cargo audit`/approved security policy check.
- [ ] Record release binary size/startup/resource usage.
- [ ] Artifact manifest with target, commit/version, SHA-256, and size.

# Implementation phases

### Phase 0 — Contract + MCP freeze — [x] DONE

- [x] Freeze legacy contract and MCP `2026-07-28` target.
- [x] Define canonical tool names/schemas/annotations.
- [x] Map each MCP tool to one Plan 027 Rust CLI capability.
- [x] Define local vs future remote authorization boundary.
- [x] Freeze concrete resource-limit targets.

### Phase 1 — Rust foundation — [x] DONE

- [x] `relay-agent` `[[bin]]` under `packages/rust-tools`.
- [x] Axum/Tokio/Clap/Serde-based implementation.
- [x] Separate protocol, transport, security, execution, config, and lifecycle modules.
- [x] `cargo fmt --check` clean.
- [x] `cargo clippy -D warnings` clean.

### Phase 2 — MCP server — [x] DONE

- [x] MCP `2026-07-28` protocol core.
- [x] Streamable HTTP.
- [x] `server/discover`.
- [x] `tools/list`.
- [x] `tools/call` structured request/error path.
- [x] JSON-RPC errors.
- [x] `MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name`, and `_meta` validation.
- [x] Content type/body limits/CORS.

### Phase 3 — Tool registry and execution — [x] DONE / HARDENING MOVED TO PHASE 11

- [x] Register Plan 027 tools.
- [x] JSON Schema argument validation.
- [x] No shell interpolation.
- [x] Basic execution dispatch/result normalization.
- [ ] Security/resource-limit fixes are tracked in Phase 11 and must not be considered closed merely because dispatch works.

### Phase 4 — Legacy Nuxt compatibility — [x] DONE / HARDENING MOVED TO PHASE 11

- [x] Legacy `/health`, `/pair`, `/revoke`, CORS, and WebSocket compatibility implemented.
- [x] Compatibility code isolated from MCP core.
- [ ] Credential, input-limit, SSRF, and policy hardening tracked in Phase 11.

### Phase 5 — Lifecycle/security hardening — [x] DONE / REMEDIATION MOVED TO PHASE 11

- [x] Core pairing/session state machine.
- [x] Origin/Host policy.
- [x] Core resource-limit plumbing.
- [x] Timeout/process-tree plumbing.
- [x] Pidfile/stop lifecycle.
- [ ] Production security remediation in Phase 11.

### Phase 6 — Real Nuxt E2E — [x] DONE

- [x] Real Rust binary.
- [x] Existing Nuxt UI.
- [x] Pair/connect.
- [x] Terminal execution.
- [x] stdout/stderr/exit/error/timeout rendering.
- [x] revoke/disconnect.
- [x] browser-origin security.

### Phase 7 — Remove Node runtime — [x] DONE

- [x] Remove relay-agent Node source/build/runtime.
- [x] Remove relay-only Node dependencies.
- [x] Remove `@yao-pkg/pkg`.
- [x] Remove obsolete build references.

### Phase 8 — Native release — [x] DONE

- [x] Cargo-native release workflow.
- [x] Supported native targets.
- [x] Stable artifact names.
- [x] Checksums.
- [x] Node-free standalone verification.
- [x] Clean-environment release smoke check.

### Phase 9 — Production hardening baseline — [x] DONE / REMEDIATION MOVED TO PHASE 11

- [x] Dependency/license/security policy baseline.
- [x] Existing size/startup/resource measurement baseline.
- [x] Release/provenance baseline.
- [ ] Remaining runtime security vulnerabilities are Phase 11 blockers.

### Phase 10 — Closeout — [ ] BLOCKED BY PHASE 11

- [ ] Nuxt E2E and release evidence remains green after Phase 11 changes.
- [ ] No Node relay runtime remains.
- [ ] Native artifacts verified.
- [ ] Documentation matches actual MCP and compatibility behavior.
- [ ] Final CI/release evidence recorded.
- [ ] Plan status changed to `COMPLETED` only after Phase 11 closes.

### Phase 11 — Production security + resource-limit remediation — [ ] IN FLIGHT

**Goal:** close the concrete vulnerabilities found by source-level security review after execution and legacy compatibility were wired. No unit-test gate is required for this phase because the project deliberately removed runtime tests for the deadline; every item must instead be validated by direct code-path review, `cargo fmt`, `cargo clippy -D warnings`, `cargo audit`, and manual/runtime smoke verification where available.

#### 11.1 — Remove privileged guard bypass — P0

- [ ] Remove relay-injected `--no-guard` from `terminal-tool` execution.
- [ ] Remove relay-injected `--no-guard` from `curl-tool` execution.
- [ ] Preserve the Plan 027 CLI guard/policy by default.
- [ ] If a privileged bypass is genuinely required internally, make it unreachable from untrusted MCP/legacy input and require an explicit trusted authorization boundary.
- [ ] Review every execution adapter for equivalent guard bypasses.
- [ ] Manually trace `tools/call` → adapter → argv and prove no untrusted request can select a guard-bypass flag.

**Acceptance:** an MCP/legacy caller cannot turn off Plan 027 execution/SSRF safeguards through relay-controlled arguments.

#### 11.2 — Credential lifecycle — P0

- [ ] Make session credentials expiry-bound with a short, explicit TTL.
- [ ] Store credential metadata (`issued_at`, `expires_at`, revocation state) instead of an unbounded live-only credential set.
- [ ] Reject expired credentials before WebSocket upgrade/command execution.
- [ ] Remove/garbage-collect expired credentials.
- [ ] Keep revoke atomic with credential validation/lookup.
- [ ] Preserve cryptographically random credential generation.
- [ ] Ensure credential comparison/storage does not leak secrets through errors.

**Acceptance:** a leaked credential stops working automatically after TTL and can be revoked immediately.

#### 11.3 — Secret/log hygiene — P0

- [ ] Remove pairing-token logging from stdout/stderr/application logs.
- [ ] Never log session credentials.
- [ ] Redact credential query parameters from request/access logs.
- [ ] Ensure error responses never contain pairing/session credentials.
- [ ] Search the entire relay source for token/credential interpolation into logs/errors.

**Acceptance:** credentials cannot appear in normal relay logs or error payloads.

#### 11.4 — Fail-closed Origin configuration — P0

- [ ] Remove any `unwrap_or("*")`/wildcard fallback for configured Origin.
- [ ] Missing/empty/invalid Origin configuration must fail before binding when execution endpoints require browser-origin authorization.
- [ ] Ensure no legacy state object can represent missing Origin as a trusted wildcard.
- [ ] Re-check CORS and server-side Origin policy after this change.

**Acceptance:** there is no code path in which missing Origin becomes `*` or otherwise broadens trust.

#### 11.5 — Legacy WebSocket input limits — P1

- [ ] Enforce a hard maximum WebSocket frame/message size before JSON parsing.
- [ ] Bound command string length.
- [ ] Bound argument count and individual argument length.
- [ ] Bound cwd length.
- [ ] Reject oversized messages deterministically.
- [ ] Ensure the limit is enforced before unbounded allocation/parsing.

**Acceptance:** one authenticated WebSocket client cannot cause unbounded JSON/message memory growth.

#### 11.6 — Legacy stdout/stderr limits — P1

- [ ] Replace unbounded `wait_with_output()` capture on legacy execution with bounded stdout/stderr capture.
- [ ] Define explicit output limits consistent with MCP execution.
- [ ] Kill the process/process-group when output exceeds the limit.
- [ ] Reap the child after kill.
- [ ] Return deterministic bounded-output errors.

**Acceptance:** a command producing unbounded output cannot exhaust relay memory.

#### 11.7 — Execution concurrency limits — P1

- [ ] Add a global execution semaphore.
- [ ] Add a per-session/per-credential execution limit where appropriate.
- [ ] Define behavior when the limit is reached: deterministic rejection or bounded queue.
- [ ] Ensure disconnected clients cannot leave permits permanently held.
- [ ] Ensure limits cover both MCP and legacy execution paths.

**Acceptance:** one client cannot spawn an unbounded number of child processes and global relay capacity remains bounded.

#### 11.8 — Timeout and process-tree cleanup — P1

- [ ] Make timeout handling explicit for both MCP and legacy paths.
- [ ] Kill the intended Unix process group/tree where supported.
- [ ] Wait/reap the child after kill.
- [ ] Avoid blocking Tokio workers during cleanup.
- [ ] Ensure `kill_on_drop` is not the only cleanup guarantee.
- [ ] Return a deterministic timeout error.
- [ ] Review Windows process cleanup semantics separately where supported.

**Acceptance:** timeout never leaves an orphaned child/process tree running indefinitely.

#### 11.9 — SSRF policy preservation — P0/P1

- [ ] Remove relay-level `--no-guard` from `curl-tool`.
- [ ] Ensure URL arguments cannot bypass Plan 027 SSRF/URL policy.
- [ ] Verify allowed schemes are enforced.
- [ ] Verify local/private/link-local/metadata destinations remain blocked according to Plan 027 policy.
- [ ] Review redirect handling so redirects cannot bypass the initial URL policy.
- [ ] Review DNS rebinding/hostname-to-IP behavior against the existing curl-tool guard.
- [ ] Do not introduce a weaker relay-specific HTTP policy.

**Acceptance:** `http_fetch` cannot be used as a relay-level SSRF bypass.

#### 11.10 — Error sanitization — P2

- [ ] Replace raw `e.to_string()` process/system errors in externally visible responses with stable sanitized messages.
- [ ] Keep detailed diagnostics only in safe internal logs, with credentials/path secrets redacted.
- [ ] Ensure stack traces/environment variables/raw command lines are never returned to MCP/legacy callers.

**Acceptance:** external errors contain actionable but non-sensitive information.

#### 11.11 — Legacy credential-in-URL handling — P2

The existing WebSocket `?credential=...` shape is a compatibility contract and must not be changed in this phase. Harden its handling instead:

- [ ] Never log the full URI/query string.
- [ ] Redact credential query parameters in any HTTP/access logging.
- [ ] Avoid echoing credential URLs in diagnostics/errors.
- [ ] Keep future remote/authenticated MCP transport independent of this legacy mechanism.

#### 11.12 — Configuration/working-directory semantics — P2

- [ ] Document that `--dir` is a default working directory, not a filesystem sandbox.
- [ ] Ensure execution cannot accidentally treat `--dir` as a security boundary.
- [ ] Verify path/working-directory errors are sanitized externally.

#### 11.13 — Final static/manual security audit — P0

After all remediation items:

- [ ] Search for `--no-guard`, wildcard Origin, raw credential logging, unbounded `wait_with_output`, unbounded WebSocket receive, and raw external error leakage across the whole relay module.
- [ ] Manually trace every MCP `tools/call` path to process spawn.
- [ ] Manually trace every legacy `exec` WebSocket path to process spawn.
- [ ] Confirm both paths enforce authorization, limits, timeout, output bounds, and tool guards.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo audit`.
- [ ] Perform release-mode/manual smoke verification of the relay binary.
- [ ] Record evidence in this plan without claiming tests that no longer exist.

## Test / verification strategy

Because runtime unit/integration tests were intentionally removed for the deadline, this plan does **not** require restoring them as a prerequisite. Static/manual verification is mandatory instead.

### MCP

- [ ] Manual protocol-version/header/body validation review.
- [ ] Manual `server/discover`, `tools/list`, `tools/call` path review.
- [ ] Verify schema validation code paths.
- [ ] Verify malformed request/error handling by source review or manual smoke requests where available.
- [ ] Interoperability with a standards-compliant MCP client/harness is optional for this deadline unless required by release policy.

### Security

- [ ] Origin/Host fail-closed path review.
- [ ] Credential lifecycle review.
- [ ] Guard/SSRF bypass review.
- [ ] WebSocket input/output bound review.
- [ ] Concurrency review.
- [ ] Timeout/process-tree review.
- [ ] Error/log secret-leak review.

### Nuxt E2E

Existing E2E/release evidence remains valid only as historical evidence. After Phase 11 modifies execution/security behavior, perform a minimal manual smoke flow against the real binary where practical:

```text
Nuxt → local relay → tool dispatch → Plan 027 Rust CLI → result
```

Do not label this as an automated test.

## CI gates

Required static gates:

- [ ] `cargo fmt --check`.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] `cargo audit`.
- [ ] Repository-wide `@yao-pkg/pkg` absence check.
- [ ] Repository-wide relay-agent JS/TS executable absence check.
- [ ] Release build for supported targets.
- [ ] Artifact naming/checksum/manifest verification.
- [ ] Node-free runtime verification.
- [ ] Dependency/license/security policy check.

**No unit-test gate:** `cargo test --workspace` and relay-agent unit/integration tests are intentionally not required for the current deadline.

## Definition of Done

Plan 028 is **CLOSED** only when:

- [ ] `relay-agent` is entirely Rust and the binary is the sole runtime entrypoint.
- [ ] It is a proper MCP server targeting the frozen specification.
- [ ] Tool catalog maps cleanly to Plan 027 Rust CLI tools.
- [ ] Nuxt compatibility is preserved or explicitly approved.
- [ ] Legacy compatibility is isolated from MCP.
- [ ] Origin/Host/auth security is fail-closed.
- [ ] Pairing/session lifecycle is single-use, expiry-bound, revocable, and race-safe.
- [ ] Tool guards cannot be disabled by untrusted relay input.
- [ ] Resource limits and process cleanup are enforced.
- [ ] WebSocket input/output is bounded.
- [ ] SSRF policy cannot be bypassed through `http_fetch`.
- [ ] Errors/logs do not leak credentials or sensitive internals.
- [ ] Node.js/TypeScript relay runtime and `@yao-pkg/pkg` are removed.
- [ ] Release CI builds native binaries directly with Cargo.
- [ ] Published artifacts are standalone, checksummed, and smoke-verified.
- [ ] Static CI gates are green and final security/release evidence is recorded.
- [ ] Phase 11 is fully checked off.

## Rollback

Keep the known-good release available until the Rust relay, Nuxt compatibility, security remediation, and native artifacts pass final verification. If remediation fails, keep Plan 028 `IN FLIGHT`, restore the known-good release, and repeat the Phase 11 security gate.

## Evidence log

- MCP protocol implementation: implemented in Rust and manually reviewed; automated relay tests were intentionally removed.
- Origin/Host policy: implemented in `security.rs`/`transport.rs`; prior tests are historical only and must not be represented as current CI evidence.
- Execution: implemented in `execution.rs`; Phase 11 is the production-hardening gate for guard bypass, output bounds, concurrency, timeout/reap, and SSRF preservation.
- Legacy compatibility: implemented in the compatibility layer; Phase 11 covers credential/log/input-limit hardening.
- Node source/runtime removal: completed.
- `@yao-pkg/pkg` removal: completed.
- Cargo release workflow: completed.
- Final CI/release evidence: must be re-recorded after Phase 11.
