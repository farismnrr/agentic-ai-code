# 028 — Relay agent: full Rust rewrite + MCP server

**Status: IN FLIGHT** — the Rust rewrite and MCP core are implemented, but production security/resource-limit remediation, legacy removal, and final E2E/release validation remain before Plan 028 can be closed.

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

- **Local Nuxt/browser:** Streamable HTTP to `127.0.0.1:<port>` plus the retained legacy compatibility path where required until Phase 12 removes it.
- **Local MCP hosts:** use standard MCP transport semantics.
- **Future external MCP client/cloud:** deploy the same tool layer behind a separately authenticated MCP endpoint; never expose the localhost execution agent publicly just to make cloud access work.

## Scope boundary

In scope: Rust relay runtime, MCP server/tool catalog/handlers, local execution bridge, legacy Nuxt compatibility until its explicit removal phase, local auth/pairing, lifecycle, release pipeline, security/resource limits, and Node runtime removal.

Out of scope: migrating Nuxt/Vue/TypeScript, replacing Plan 027 CLI tools, arbitrary OS sandboxing, public unauthenticated execution, or a second tool implementation for external MCP client.

## Architecture

```text
Nuxt / MCP client
       │ Streamable HTTP / legacy compatibility (removed in Phase 12)
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

## Current phase order

- Phase 11 — Production security + resource-limit remediation.
- Phase 12 — Remove legacy relay compatibility.
- Phase 13 — Final E2E + release validation (**final gate; not a blocker for Phases 11–12**).

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

## Phase 11 — Production security + resource-limit remediation — [ ] IN FLIGHT

**Goal:** close concrete vulnerabilities found by source-level security review after execution and legacy compatibility were wired. No unit-test gate is required for this phase; every item is validated by direct code-path review, `cargo fmt`, `cargo clippy -D warnings`, `cargo audit`, and manual/runtime smoke verification where available.

- [ ] Remove relay-injected `--no-guard` from terminal/curl execution and prove no untrusted input can disable Plan 027 guards.
- [ ] Make session credentials expiry-bound, revocable, and race-safe.
- [ ] Remove pairing/session credential logging and redact credential query parameters from logs/errors.
- [ ] Remove wildcard/missing-Origin fallbacks and fail closed.
- [ ] Bound legacy WebSocket message, command, argument, and cwd sizes.
- [ ] Bound legacy stdout/stderr capture and kill/reap on output overflow.
- [ ] Add global and per-session execution concurrency limits.
- [ ] Make timeout/process-tree kill/reap explicit and deterministic.
- [ ] Preserve Plan 027 SSRF/URL policy; no relay-level curl guard bypass or redirect/DNS policy bypass.
- [ ] Sanitize externally visible process/system errors.
- [ ] Document `--dir` as working-directory configuration, not a filesystem sandbox.
- [ ] Perform final static/manual security audit for guard bypass, wildcard Origin, secret leakage, unbounded input/output, concurrency, timeout/reap, and SSRF paths.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo audit`.

**Phase 11 acceptance:** all execution/security/resource-limit paths are bounded and fail closed; no untrusted request can disable Plan 027 guards or bypass SSRF policy.

## Phase 12 — Remove legacy relay compatibility — [ ] NOT STARTED

**Goal:** make MCP Streamable HTTP the sole relay protocol and delete obsolete legacy Nuxt relay compatibility instead of carrying a permanent compatibility surface.

### 12.1 Consumer/dependency audit

- [ ] Search Nuxt/frontend and repository consumers for `/pair`, `/revoke`, legacy WebSocket, `credential=`, `exec_result`, and legacy relay-specific message types.
- [ ] Identify every remaining consumer before deletion.
- [ ] Confirm required consumers have migrated to MCP or are explicitly approved for removal.

### 12.2 Delete legacy protocol/runtime

- [ ] Delete `legacy.rs` / legacy compatibility modules once consumers are migrated.
- [ ] Remove legacy WebSocket server/upgrade path.
- [ ] Remove `/pair` and `/revoke` legacy HTTP endpoints.
- [ ] Remove legacy `exec` / `exec_result` message protocol.
- [ ] Remove legacy credential/session state used only by that protocol.
- [ ] Remove compatibility-only config, types, helpers, and imports.

### 12.3 Simplify security/resource model

- [ ] Remove security/resource-limit code that existed solely for legacy WebSocket execution.
- [ ] Ensure MCP execution retains all Phase 11 guard, authorization, SSRF, timeout, output, and concurrency protections.
- [ ] Re-run source-level attack-path review after deletion so removed code cannot leave a weaker alternate execution path.

### 12.4 Frontend/docs/release cleanup

- [ ] Migrate any remaining Nuxt relay calls to MCP before deleting their old endpoint.
- [ ] Remove legacy protocol documentation/examples.
- [ ] Remove obsolete release/configuration references.
- [ ] Ensure no legacy relay symbols remain repository-wide.

**Phase 12 acceptance:** there is exactly one relay execution protocol (MCP Streamable HTTP); no legacy WebSocket/pair/revoke execution path remains, and Nuxt uses the supported MCP path.

## Phase 13 — Final E2E + release validation — [ ] NOT STARTED / FINAL GATE

**Goal:** validate the complete finished system only after Phases 11–12 are complete. E2E/release validation is deliberately not a blocker for the intermediate implementation phases.

### 13.1 Production binary smoke

- [ ] Build release-mode `relay-agent` from a clean environment.
- [ ] Verify standalone native execution with no Node/V8/libnode runtime dependency.
- [ ] Verify supported artifact names, checksums, and manifest metadata.

### 13.2 End-to-end MCP flow

- [ ] Start the production relay binary.
- [ ] Connect Nuxt through MCP Streamable HTTP.
- [ ] `server/discover` succeeds.
- [ ] `tools/list` exposes the expected Plan 027 tools.
- [ ] `terminal_exec` executes through the Plan 027 Rust CLI.
- [ ] `http_fetch` executes while preserving SSRF policy.
- [ ] `web_search` executes through the Plan 027 Rust CLI.
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

Because runtime unit/integration tests were intentionally removed for the deadline, this plan does not require restoring them as a prerequisite. Static/manual verification is mandatory during Phases 11–12. Full E2E/release validation is intentionally deferred to Phase 13.

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
- [ ] Nuxt compatibility is preserved through the final MCP path.
- [ ] Legacy compatibility is removed in Phase 12.
- [ ] Origin/Host/auth security is fail-closed.
- [ ] Pairing/session lifecycle is single-use, expiry-bound, revocable, and race-safe where retained before legacy removal.
- [ ] Tool guards cannot be disabled by untrusted relay input.
- [ ] Resource limits and process cleanup are enforced.
- [ ] SSRF policy cannot be bypassed through `http_fetch`.
- [ ] Errors/logs do not leak credentials or sensitive internals.
- [ ] Node.js/TypeScript relay runtime and `@yao-pkg/pkg` are removed.
- [ ] Release CI builds native binaries directly with Cargo.
- [ ] Published artifacts are standalone, checksummed, and smoke-verified.
- [ ] Phase 11 is fully checked off.
- [ ] Phase 12 is fully checked off.
- [ ] Phase 13 final E2E/release gate is fully checked off.

## Rollback

Keep the known-good release available until the Rust relay, Nuxt migration, security remediation, legacy removal, and native artifacts pass final Phase 13 verification. If remediation fails, keep Plan 028 `IN FLIGHT`, restore the known-good release, and repeat the appropriate phase gate.

## Evidence log

- MCP protocol implementation: implemented in Rust and manually reviewed; automated relay tests were intentionally removed.
- Origin/Host policy: implemented and must be re-verified after Phase 11/12 changes.
- Execution: implemented in Rust; Phase 11 is the production-hardening gate for guard bypass, output bounds, concurrency, timeout/reap, and SSRF preservation.
- Legacy compatibility: temporary compatibility layer; Phase 12 removes it.
- Node source/runtime removal: completed.
- `@yao-pkg/pkg` removal: completed.
- Cargo release workflow: completed.
- Final CI/release/E2E evidence: recorded only in Phase 13 after all implementation and removal work is complete.
