# 028 — Relay agent: full Rust rewrite + MCP server

**Status: IN FLIGHT** — the Rust rewrite is implemented, but a final security-remediation phase remains before the plan can be closed.

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
- Provide a clean path for future authenticated remote MCP deployment without exposing the localhost execution relay publicly.
- Remove Node.js, `@yao-pkg/pkg`, and relay-specific JS runtime/build dependencies.
- Build and publish native Rust artifacts with Cargo.

## Deployment boundary

- **Local Nuxt/browser:** Streamable HTTP to `127.0.0.1:<port>`.
- **Local MCP hosts:** use standard MCP transport semantics.
- **Future ChatGPT/cloud:** deploy the same tool layer behind a separately authenticated MCP endpoint; never expose the localhost execution agent publicly just to make cloud access work.

## Scope boundary

In scope: Rust relay runtime, MCP server/tool catalog/handlers, local execution bridge, local lifecycle, release pipeline, security/resource limits, and Node runtime removal.

Out of scope: migrating Nuxt/Vue/TypeScript, replacing Plan 027 CLI tools, arbitrary OS sandboxing, public unauthenticated execution, or a second tool implementation for ChatGPT.

## Architecture

```text
Nuxt / MCP client
       │ Streamable HTTP
       ▼
Rust relay-agent
  ├─ protocol + transport
  ├─ localhost + Origin/Host policy
  ├─ tool registry
  ├─ execution + limits
  └─ lifecycle
       │
       ▼
Plan 027 Rust CLI tools
  terminal-tool / curl-tool / searxng-search-tool
```

## Current phase order

- Phase 11 — Production security + resource-limit remediation.
- Phase 12 — Remove legacy relay compatibility.
- Phase 14 — Final security remediation for the current MCP-only execution path.
- Phase 13 — Final E2E + release validation (**final gate**).

Phase 13 is intentionally deferred until all implementation/security/removal work is complete. Do not block incremental development on E2E/release validation before the final phase.

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
- [ ] Phase 14 must ensure all privileged execution paths preserve tool guards/policy.

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
  ├─ loopback binding
  ├─ Host policy
  └─ exact Origin policy
          │
          ▼
MCP request
  ├─ protocol/version/header validation
  └─ tool argument/schema validation
          │
          ▼
Execution policy
  ├─ Plan 027 tool guards
  ├─ resource limits
  └─ process lifecycle
```

No public unauthenticated execution is permitted.

## Phase 11 — Production security + resource-limit remediation — [x] DONE

- [x] Remove relay-injected `--no-guard` from terminal/curl execution and prove no untrusted input can disable Plan 027 guards.
- [x] Make retained session credentials expiry-bound, revocable, and race-safe.
- [x] Remove credential logging and redact credential query parameters from logs/errors.
- [x] Remove wildcard/missing-Origin fallbacks and fail closed.
- [x] Bound retained legacy execution input/output and process lifecycle where applicable.
- [x] Add global and per-session execution concurrency limits.
- [x] Make timeout/process-tree kill/reap explicit and deterministic.
- [x] Preserve Plan 027 SSRF/URL policy at the relay boundary.
- [x] Sanitize externally visible process/system errors.
- [x] Document `--dir` as working-directory configuration, not a filesystem sandbox.
- [x] Perform static/manual security audit for guard bypass, wildcard Origin, secret leakage, unbounded input/output, concurrency, timeout/reap, and SSRF paths.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo audit`.

## Phase 12 — Remove legacy relay compatibility — [x] COMPLETED

- [x] Audit repository consumers for `/pair`, `/revoke`, legacy WebSocket, `credential=`, `exec_result`, and legacy relay message types.
- [x] Migrate/remove remaining legacy consumers.
- [x] Delete legacy WebSocket server/upgrade path.
- [x] Remove `/pair` and `/revoke` endpoints.
- [x] Remove legacy `exec` / `exec_result` protocol.
- [x] Remove legacy credential/session state and compatibility-only helpers.
- [x] Remove obsolete docs/config/release references.
- [x] Re-run source-level attack-path review after deletion.

**Phase 12 acceptance:** MCP Streamable HTTP is the sole relay execution protocol and no legacy execution path remains.

## Phase 14 — Final security remediation — [ ] IN FLIGHT

**Goal:** address the remaining concrete findings discovered after Phase 12 removed the legacy path. This phase is intentionally before the final E2E/release gate.

### 14.1 Terminal execution policy

- [x] Resolve the `terminal-tool` guard/execution-policy contradiction: relay must not pass `--no-guard`, but normal guarded execution must be capable of performing an approved command rather than always rejecting it.
- [x] Define the single authoritative execution policy between relay and Plan 027 `terminal-tool`.
- [x] Verify untrusted MCP arguments cannot select or disable a privileged execution mode.
- [x] Re-run source-level command-injection/argument-boundary review after the policy change.

### 14.2 SSRF / DNS rebinding

- [x] Eliminate DNS TOCTOU in `http_fetch`: validation must apply to the addresses actually used for the outbound connection, not a separate preliminary lookup.
- [x] Preserve scheme, private/link-local/loopback/metadata-address policy after DNS resolution.
- [x] Ensure redirects are revalidated against the same SSRF policy.
- [x] Ensure IPv4/IPv6, DNS aliases, and hostname edge cases cannot bypass the policy.
- [x] Manually review the complete `url -> resolve -> connect -> redirect` path.

### 14.3 Timeout and execution resource bounds

- [x] Add a server-side maximum for `timeout_ms`; schema validation alone is not sufficient.
- [x] Prevent integer overflow when applying timeout grace periods.
- [x] Add an explicit maximum argument count for `terminal_exec`.
- [x] Add aggregate argument-byte limits in addition to per-item limits.
- [x] Add maximum header count and aggregate header-byte limits for `http_fetch`.
- [x] Confirm process/output/concurrency limits are enforced independently of client-supplied schemas.

### 14.4 Tool-specific network policy

- [x] Restrict `http_fetch` to explicitly supported HTTP methods; reject unsafe/unneeded methods such as `CONNECT`/`TRACE` unless there is a documented requirement.
- [x] Restrict `web_search.base_url` to a trusted configured endpoint rather than allowing an MCP caller to select an arbitrary network destination.
- [x] Apply the same outbound network policy to every redirect and secondary request.

### 14.5 Sibling binary trust boundary

- [ ] Verify `terminal-tool`, `curl-tool`, and `searxng-search-tool` resolved from the relay binary directory cannot be replaced by an untrusted local user.
- [ ] Ensure release/install directories have appropriate ownership and executable permissions.
- [ ] Document the sibling-binary trust assumption and installation requirements.
- [ ] Consider integrity verification only if the deployment threat model requires protection against local binary tampering.

### 14.6 Final static security gate

- [ ] Review all MCP `tools/call` execution paths from request parsing to OS/network side effects.
- [ ] Search repository-wide for `--no-guard`, wildcard Origin, unbounded timeout arithmetic, arbitrary `base_url`, and alternate execution entrypoints.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo audit`.

**Phase 14 acceptance:** no known P0/P1 security finding remains in command execution, SSRF, timeout/input limits, network policy, or process-launch paths, and all privileged execution paths have one explicit authoritative policy.

## Phase 13 — Final E2E + release validation — [ ] NOT STARTED / FINAL GATE

**Goal:** validate the complete finished system only after Phases 11, 12, and 14 are complete. E2E/release validation is deliberately not a blocker for the intermediate implementation phases.

### 13.1 Production binary smoke

- [ ] Build release-mode `relay-agent` from a clean environment.
- [ ] Verify standalone native execution with no Node/V8/libnode runtime dependency.
- [ ] Verify supported artifact names, checksums, and manifest metadata.

### 13.2 End-to-end MCP flow

- [ ] Start the production relay binary.
- [ ] Connect Nuxt through MCP Streamable HTTP.
- [ ] `server/discover` succeeds.
- [ ] `tools/list` exposes the expected Plan 027 tools.
- [ ] `terminal_exec` executes through the Plan 027 Rust CLI with the authoritative guard policy.
- [ ] `http_fetch` executes while preserving SSRF policy, including redirects.
- [ ] `web_search` executes only against the configured trusted endpoint.
- [ ] Invalid Origin/Host requests are rejected.
- [ ] Resource limits and timeout behavior are manually smoke-verified.
- [ ] No legacy WebSocket/pair/revoke path is reachable.

### 13.3 Release/CI evidence

- [ ] `cargo fmt --check` green.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` green.
- [ ] `cargo audit` green.
- [ ] Repository-wide `@yao-pkg/pkg` absence check green.
- [ ] Repository-wide relay-agent JS/TS executable absence check green.
- [ ] Release workflow builds native artifacts directly with Cargo.
- [ ] Final clean-environment smoke verification recorded.
- [ ] Final evidence recorded in this plan.

**Phase 13 acceptance:** production binary, Nuxt/MCP E2E, security smoke checks, and release artifacts are all green. Only then may Plan 028 move to `COMPLETED`.

## Verification strategy

Because runtime unit/integration tests were intentionally removed for the deadline, this plan does not require restoring them as a prerequisite. Static/manual verification is mandatory during Phases 11–12 and 14. Full E2E/release validation is intentionally deferred to Phase 13.

## CI gates

Required static gates throughout implementation:

- [ ] `cargo fmt --check`.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] `cargo audit`.
- [ ] Repository-wide `@yao-pkg/pkg` absence check.
- [ ] Repository-wide relay-agent JS/TS executable absence check.

**No unit-test gate:** `cargo test --workspace` and relay-agent unit/integration tests are intentionally not required for the current deadline.

## Definition of Done

Plan 028 is **CLOSED** only when:

- [ ] `relay-agent` is entirely Rust and the binary is the sole runtime entrypoint.
- [ ] It is a proper MCP server targeting the frozen specification.
- [ ] Tool catalog maps cleanly to Plan 027 Rust CLI tools.
- [ ] Nuxt uses the final MCP path.
- [ ] Legacy compatibility is removed.
- [ ] Origin/Host security is fail-closed.
- [ ] Tool guards cannot be disabled by untrusted relay input.
- [ ] Resource limits and process cleanup are enforced.
- [ ] SSRF policy cannot be bypassed through `http_fetch`.
- [ ] Errors/logs do not leak credentials or sensitive internals.
- [ ] Node.js/TypeScript relay runtime and `@yao-pkg/pkg` are removed.
- [ ] Release CI builds native binaries directly with Cargo.
- [ ] Phase 11 is fully checked off.
- [ ] Phase 12 is fully checked off.
- [ ] Phase 14 is fully checked off.
- [ ] Phase 13 final E2E/release gate is fully checked off.

## Rollback

Keep the known-good release available until the Rust relay, Nuxt migration, security remediation, legacy removal, and native artifacts pass final Phase 13 verification. If remediation fails, keep Plan 028 `IN FLIGHT`, restore the known-good release, and repeat the appropriate phase gate.

## Evidence log

- MCP protocol implementation: implemented in Rust and manually reviewed; automated relay tests were intentionally removed.
- Origin/Host policy: implemented and must be re-verified after Phase 14 changes.
- Execution: implemented in Rust; Phase 14 is the final hardening gate for guard policy, output/input bounds, timeout arithmetic, SSRF, network policy, and process-launch trust.
- Legacy compatibility: removed in Phase 12.
- Node source/runtime removal: completed.
- `@yao-pkg/pkg` removal: completed.
- Cargo release workflow: completed.
- Final CI/release/E2E evidence: recorded only in Phase 13 after all implementation and remediation work is complete.
