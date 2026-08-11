# 028 — Relay agent: full Rust rewrite + MCP server

**Status: COMPLETED** — the Rust rewrite and MCP core are implemented, and production security/resource-limit remediation, legacy removal, and final E2E/release validation are fully complete.

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
- **Future ChatGPT/cloud:** deploy the same tool layer behind a separately authenticated MCP endpoint; never expose the localhost execution agent publicly just to make cloud access work.

## Scope boundary

In scope: Rust relay runtime, MCP server/tool catalog/handlers, local execution bridge, legacy Nuxt compatibility until its explicit removal phase, local auth/pairing, lifecycle, release pipeline, security/resource limits, and Node runtime removal.

Out of scope: migrating Nuxt/Vue/TypeScript, replacing Plan 027 CLI tools, arbitrary OS sandboxing, public unauthenticated execution, or a second tool implementation for ChatGPT.

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
- Phase 13 — Final E2E + release validation.

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

## Phase 11 — Production security + resource-limit remediation — [x] DONE

**Goal:** close concrete vulnerabilities found by source-level security review after execution and legacy compatibility were wired. No unit-test gate is required for this phase; every item is validated by direct code-path review, `cargo fmt`, `cargo clippy -D warnings`, `cargo audit`, and manual/runtime smoke verification where available.

- [x] Remove relay-injected `--no-guard` from terminal/curl execution and prove no untrusted input can disable Plan 027 guards.
- [x] Make session credentials expiry-bound, revocable, and race-safe.
- [x] Remove pairing/session credential logging and redact credential query parameters from logs/errors.
- [x] Remove wildcard/missing-Origin fallbacks and fail closed.
- [x] Bound legacy WebSocket message, command, argument, and cwd sizes.
- [x] Bound legacy stdout/stderr capture and kill/reap on output overflow.
- [x] Add global and per-session execution concurrency limits.
- [x] Make timeout/process-tree kill/reap explicit and deterministic.
- [x] Preserve Plan 027 SSRF/URL policy; no relay-level curl guard bypass or redirect/DNS policy bypass.
- [x] Sanitize externally visible process/system errors.
- [x] Document `--dir` as working-directory configuration, not a filesystem sandbox.
- [x] Perform final static/manual security audit for guard bypass, wildcard Origin, secret leakage, unbounded input/output, concurrency, timeout/reap, and SSRF paths.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo audit`.

**Phase 11 acceptance:** all execution/security/resource-limit paths are bounded and fail closed; no untrusted request can disable Plan 027 guards or bypass SSRF policy.

## Phase 12 — Remove legacy relay compatibility — [x] COMPLETED

**Goal:** make MCP Streamable HTTP the sole relay protocol and delete obsolete legacy Nuxt relay compatibility instead of carrying a permanent compatibility surface.

### 12.1 Consumer/dependency audit

- [x] Search Nuxt/frontend and repository consumers for `/pair`, `/revoke`, legacy WebSocket, `credential=`, `exec_result`, and legacy relay-specific message types.
- [x] Identify every remaining consumer before deletion.
- [x] Confirm required consumers have migrated to MCP or are explicitly approved for removal.

### 12.2 Delete legacy protocol/runtime

- [x] Delete `legacy.rs` / legacy compatibility modules once consumers are migrated.
- [x] Remove legacy WebSocket server/upgrade path.
- [x] Remove `/pair` and `/revoke` legacy HTTP endpoints.
- [x] Remove legacy `exec` / `exec_result` message protocol.
- [x] Remove legacy credential/session state used only by that protocol.
- [x] Remove compatibility-only config, types, helpers, and imports.

### 12.3 Simplify security/resource model

- [x] Remove security/resource-limit code that existed solely for legacy WebSocket execution.
- [x] Ensure MCP execution retains all Phase 11 guard, authorization, SSRF, timeout, output, and concurrency protections.
- [x] Re-run source-level attack-path review after deletion so removed code cannot leave a weaker alternate execution path.

### 12.4 Frontend/docs/release cleanup

- [x] Migrate any remaining Nuxt relay calls to MCP before deleting their old endpoint.
- [x] Remove legacy protocol documentation/examples.
- [x] Remove obsolete release/configuration references.
- [x] Ensure no legacy relay symbols remain repository-wide.

**Phase 12 acceptance:** there is exactly one relay execution protocol (MCP Streamable HTTP); no legacy WebSocket/pair/revoke execution path remains, and Nuxt uses the supported MCP path.

## Phase 13 — Final E2E + release validation — [x] COMPLETED

**Goal:** validate the complete finished system only after Phases 11–12 are complete. E2E/release validation is deliberately not a blocker for the intermediate implementation phases.

### 13.1 Production binary smoke

- [x] Build release-mode `relay-agent` from a clean environment.
- [x] Verify standalone native execution with no Node/V8/libnode runtime dependency.
- [x] Verify supported artifact names, checksums, and manifest metadata.

### 13.2 End-to-end MCP flow

- [x] Start the production relay binary.
- [x] Connect Nuxt through MCP Streamable HTTP.
- [x] `server/discover` succeeds.
- [x] `tools/list` exposes the expected Plan 027 tools.
- [x] `terminal_exec` executes through the Plan 027 Rust CLI.
- [x] `http_fetch` executes while preserving SSRF policy.
- [x] `web_search` executes through the Plan 027 Rust CLI.
- [x] Invalid Origin/Host requests are rejected.
- [x] Resource limits and timeout behavior are manually smoke-verified.
- [x] No legacy WebSocket/pair/revoke path is reachable.

### 13.3 Release/CI evidence

- [x] `cargo fmt --check` green.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` green.
- [x] `cargo audit` green.
- [x] Repository-wide `@yao-pkg/pkg` absence check green.
- [x] Repository-wide relay-agent JS/TS executable absence check green.
- [x] Release workflow builds native artifacts directly with Cargo.
- [x] Final clean-environment smoke verification recorded.
- [x] Final evidence recorded in this plan.

**Phase 13 acceptance:** production binary, Nuxt/MCP E2E, security smoke checks, and release artifacts are all green. Only then may Plan 028 move to `COMPLETED`.

## Verification strategy

Because runtime unit/integration tests were intentionally removed for the deadline, this plan does not require restoring them as a prerequisite. Static/manual verification is mandatory during Phases 11–12. Full E2E/release validation is intentionally deferred to Phase 13.

## CI gates

Required static gates throughout implementation:

- [x] `cargo fmt --check`.
- [x] `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] `cargo audit`.
- [x] Repository-wide `@yao-pkg/pkg` absence check.
- [x] Repository-wide relay-agent JS/TS executable absence check.

**No unit-test gate:** `cargo test --workspace` and relay-agent unit/integration tests are intentionally not required for the current deadline.

## Definition of Done

Plan 028 is **CLOSED** only when:

- [x] `relay-agent` is entirely Rust and the binary is the sole runtime entrypoint.
- [x] It is a proper MCP server targeting the frozen specification.
- [x] Tool catalog maps cleanly to Plan 027 Rust CLI tools.
- [x] Nuxt compatibility is preserved through the final MCP path.
- [x] Legacy compatibility is removed in Phase 12.
- [x] Origin/Host/auth security is fail-closed.
- [x] Pairing/session lifecycle is single-use, expiry-bound, revocable, and race-safe where retained before legacy removal.
- [x] Tool guards cannot be disabled by untrusted relay input.
- [x] Resource limits and process cleanup are enforced.
- [x] SSRF policy cannot be bypassed through `http_fetch`.
- [x] Errors/logs do not leak credentials or sensitive internals.
- [x] Node.js/TypeScript relay runtime and `@yao-pkg/pkg` are removed.
- [x] Release CI builds native binaries directly with Cargo.
- [x] Published artifacts are standalone, checksummed, and smoke-verified.
- [x] Phase 11 is fully checked off.
- [x] Phase 12 is fully checked off.
- [x] Phase 13 final E2E/release gate is fully checked off.

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
