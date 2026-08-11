# Plan 028 — Phase 12: Remove legacy relay compatibility

**Status: COMPLETED**

## Goal

Remove the legacy Nuxt relay/WebSocket compatibility layer so `relay-agent` has one current protocol surface: the Rust MCP server. This phase supersedes the need to harden legacy `/pair`, `/revoke`, and WebSocket execution paths when those paths are no longer required by the product.

The desired end state is a smaller, easier-to-audit Rust relay with MCP as the sole execution API. Nuxt continues to use MCP over Streamable HTTP; Nuxt/Vue/TypeScript itself is not migrated.

## Scope

### 12.1 — Confirm consumers before deletion — P0

- [x] Search the repository for `/pair`, `/revoke`, `/health`, legacy WebSocket paths, `exec_result`, and `credential=` usage.
- [x] Confirm Nuxt no longer depends on the legacy relay contract.
- [x] Confirm no release script, documentation, smoke script, or deployment configuration requires the legacy endpoints.
- [x] Record any remaining consumer and either migrate it to MCP or explicitly remove it from scope.

**Acceptance:** repository-wide search shows no required runtime consumer of the legacy relay protocol.

### 12.2 — Delete legacy implementation — P0

- [x] Remove `legacy.rs` / `http_compat.rs` / `websocket_compat.rs` or their current equivalents.
- [x] Remove `LegacyState` and legacy pairing/session state that exists only for the compatibility adapter.
- [x] Remove `/health` compatibility handler if it is not required by the current MCP deployment contract.
- [x] Remove `/pair` endpoint.
- [x] Remove `/revoke` endpoint.
- [x] Remove legacy WebSocket endpoint and `exec` / `exec_result` protocol handling.
- [x] Remove legacy-only request/response types.
- [x] Remove dead imports, modules, dependencies, and configuration introduced solely for legacy compatibility.

**Acceptance:** no legacy relay execution code remains in the Rust binary.

### 12.3 — Simplify authorization model — P0

- [x] Remove local pairing-token generation/consumption if it has no remaining consumer.
- [x] Remove legacy session credentials if they are no longer needed by MCP.
- [x] Remove credential-in-URL handling and all related redaction code that exists solely for the legacy path.
- [x] Keep MCP local Origin/Host policy fail-closed.
- [x] Keep future remote MCP authorization explicitly separate from local execution policy.
- [x] Ensure deletion does not create a new unauthenticated execution path.

**Acceptance:** MCP remains protected by the intended local security boundary and no obsolete credential mechanism remains.

### 12.4 — Simplify resource and execution controls — P1

- [x] Remove legacy-only WebSocket message limits after the endpoint is deleted.
- [x] Remove legacy-only execution concurrency bookkeeping if it is no longer needed.
- [x] Retain global/per-session limits required by MCP execution.
- [x] Retain bounded stdout/stderr, argument limits, timeout, process-tree kill/reap, and Plan 027 tool guards for MCP.
- [x] Remove dead constants and configuration associated only with legacy execution.

**Acceptance:** every retained execution path still has explicit authorization, argument/output limits, concurrency bounds, timeout, cleanup, and Plan 027 guard enforcement.

### 12.5 — Remove obsolete frontend compatibility — P1

- [x] Update Nuxt client code to use the current MCP transport if it still calls legacy endpoints.
- [x] Remove legacy relay client code from Nuxt only where it is specifically tied to the deleted protocol.
- [x] Do not migrate Nuxt/Vue/TypeScript generally; only remove obsolete relay integration.
- [x] Update frontend error handling to the MCP contract.

**Acceptance:** current Nuxt flow works through MCP without legacy pairing/WebSocket APIs.

### 12.6 — Documentation and release cleanup — P1

- [x] Remove legacy compatibility claims from Plan 028 main plan and architecture docs.
- [x] Remove legacy endpoint examples from README/docs/scripts.
- [x] Update deployment documentation to state MCP is the sole relay protocol.
- [x] Remove obsolete release smoke checks for legacy endpoints.
- [x] Update artifact/runtime documentation if module layout changes.

### 12.7 — Static/manual verification — P0

No unit-test gate is required under the current deadline decision.

- [x] Repository-wide search confirms no legacy endpoint/handler remains.
- [x] Repository-wide search confirms no `legacy`, `pair`, `revoke`, `exec_result`, or `credential=` runtime path remains unless explicitly documented as non-runtime history.
- [x] Manually trace MCP `tools/call` to Plan 027 Rust CLI execution.
- [x] Confirm no unauthenticated route can reach execution.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo audit`.
- [x] Perform minimal release/manual smoke verification of MCP discovery, tool listing, and one safe execution path.

## Completion gate

Phase 12 is complete only when the legacy relay code and its runtime contract are deleted, all required Nuxt consumers use MCP, and the remaining MCP-only execution path passes the static/manual security review.

After Phase 12 completes:

- Phase 11 items that existed solely for legacy compatibility may be marked **N/A / removed by design** rather than implemented.
- Phase 10 closeout must be re-run against the MCP-only architecture.
- Plan 028 can only become `COMPLETED` after the main plan's remaining MCP/security/release gates are satisfied.
