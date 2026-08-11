# Plan 028 — Phase 12: Remove legacy relay compatibility

**Status: NOT STARTED**

## Goal

Remove the legacy Nuxt relay/WebSocket compatibility layer so `relay-agent` has one current protocol surface: the Rust MCP server. This phase supersedes the need to harden legacy `/pair`, `/revoke`, and WebSocket execution paths when those paths are no longer required by the product.

The desired end state is a smaller, easier-to-audit Rust relay with MCP as the sole execution API. Nuxt continues to use MCP over Streamable HTTP; Nuxt/Vue/TypeScript itself is not migrated.

## Scope

### 12.1 — Confirm consumers before deletion — P0

- [ ] Search the repository for `/pair`, `/revoke`, `/health`, legacy WebSocket paths, `exec_result`, and `credential=` usage.
- [ ] Confirm Nuxt no longer depends on the legacy relay contract.
- [ ] Confirm no release script, documentation, smoke script, or deployment configuration requires the legacy endpoints.
- [ ] Record any remaining consumer and either migrate it to MCP or explicitly remove it from scope.

**Acceptance:** repository-wide search shows no required runtime consumer of the legacy relay protocol.

### 12.2 — Delete legacy implementation — P0

- [ ] Remove `legacy.rs` / `http_compat.rs` / `websocket_compat.rs` or their current equivalents.
- [ ] Remove `LegacyState` and legacy pairing/session state that exists only for the compatibility adapter.
- [ ] Remove `/health` compatibility handler if it is not required by the current MCP deployment contract.
- [ ] Remove `/pair` endpoint.
- [ ] Remove `/revoke` endpoint.
- [ ] Remove legacy WebSocket endpoint and `exec` / `exec_result` protocol handling.
- [ ] Remove legacy-only request/response types.
- [ ] Remove dead imports, modules, dependencies, and configuration introduced solely for legacy compatibility.

**Acceptance:** no legacy relay execution code remains in the Rust binary.

### 12.3 — Simplify authorization model — P0

- [ ] Remove local pairing-token generation/consumption if it has no remaining consumer.
- [ ] Remove legacy session credentials if they are no longer needed by MCP.
- [ ] Remove credential-in-URL handling and all related redaction code that exists solely for the legacy path.
- [ ] Keep MCP local Origin/Host policy fail-closed.
- [ ] Keep future remote MCP authorization explicitly separate from local execution policy.
- [ ] Ensure deletion does not create a new unauthenticated execution path.

**Acceptance:** MCP remains protected by the intended local security boundary and no obsolete credential mechanism remains.

### 12.4 — Simplify resource and execution controls — P1

- [ ] Remove legacy-only WebSocket message limits after the endpoint is deleted.
- [ ] Remove legacy-only execution concurrency bookkeeping if it is no longer needed.
- [ ] Retain global/per-session limits required by MCP execution.
- [ ] Retain bounded stdout/stderr, argument limits, timeout, process-tree kill/reap, and Plan 027 tool guards for MCP.
- [ ] Remove dead constants and configuration associated only with legacy execution.

**Acceptance:** every retained execution path still has explicit authorization, argument/output limits, concurrency bounds, timeout, cleanup, and Plan 027 guard enforcement.

### 12.5 — Remove obsolete frontend compatibility — P1

- [ ] Update Nuxt client code to use the current MCP transport if it still calls legacy endpoints.
- [ ] Remove legacy relay client code from Nuxt only where it is specifically tied to the deleted protocol.
- [ ] Do not migrate Nuxt/Vue/TypeScript generally; only remove obsolete relay integration.
- [ ] Update frontend error handling to the MCP contract.

**Acceptance:** current Nuxt flow works through MCP without legacy pairing/WebSocket APIs.

### 12.6 — Documentation and release cleanup — P1

- [ ] Remove legacy compatibility claims from Plan 028 main plan and architecture docs.
- [ ] Remove legacy endpoint examples from README/docs/scripts.
- [ ] Update deployment documentation to state MCP is the sole relay protocol.
- [ ] Remove obsolete release smoke checks for legacy endpoints.
- [ ] Update artifact/runtime documentation if module layout changes.

### 12.7 — Static/manual verification — P0

No unit-test gate is required under the current deadline decision.

- [ ] Repository-wide search confirms no legacy endpoint/handler remains.
- [ ] Repository-wide search confirms no `legacy`, `pair`, `revoke`, `exec_result`, or `credential=` runtime path remains unless explicitly documented as non-runtime history.
- [ ] Manually trace MCP `tools/call` to Plan 027 Rust CLI execution.
- [ ] Confirm no unauthenticated route can reach execution.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo audit`.
- [ ] Perform minimal release/manual smoke verification of MCP discovery, tool listing, and one safe execution path.

## Completion gate

Phase 12 is complete only when the legacy relay code and its runtime contract are deleted, all required Nuxt consumers use MCP, and the remaining MCP-only execution path passes the static/manual security review.

After Phase 12 completes:

- Phase 11 items that existed solely for legacy compatibility may be marked **N/A / removed by design** rather than implemented.
- Phase 10 closeout must be re-run against the MCP-only architecture.
- Plan 028 can only become `COMPLETED` after the main plan's remaining MCP/security/release gates are satisfied.
