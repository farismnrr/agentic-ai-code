# Plan 029 freezes the external MCP client MCP target around stateless `POST /mcp`, Auth0-backed OAuth, and `relay.coding`.

Plan 029 should stay anchored to the current MCP `2026-07-28` transport and the relay's existing Resource Server model instead of reviving the old SSE story.

## Durable decisions

- external MCP client write-capable E2E should target a Business workspace with developer mode enabled and a custom MCP app, but only if the live tenant actually exposes full MCP write/modify actions.
- If the selected Business tenant does not expose write/modify actions, that is a blocker, not something to paper over with a read-only confirmation flow.
- The external OAuth/OIDC provider is frozen as Auth0, using a user-defined OAuth client as the baseline registration mode.
- The relay stays a JWKS-backed Resource Server and does not grow its own OAuth client-registration database.
- The canonical MCP resource identifier is the externally reachable HTTPS `/mcp` URL, not localhost and not an SSE message endpoint.
- `relay.coding` is the default full-coding resource scope for the complete toolset, including `terminal_exec`.
- Optional narrow scopes are not worth supporting in the first production profile because they add complexity without materially isolating a toolset that already includes terminal execution.
- Current external MCP client setup facts that are exact: developers create/test/deploy MCP apps in developer mode; setup asks for an endpoint and required metadata; OAuth flows require callback URL configuration when applicable; app permissions and Action control influence when external MCP client asks before using actions.
- Current external MCP client setup facts that are only inferred until captured from the live UI: literal field labels, field ordering, conditional visibility, the connector callback URI, CIMD/DCR selectors, and any auto-discovered values.

## Rationale

- A single explicit `relay.coding` grant is more honest than pretending `terminal_exec` can be tightly partitioned into weaker OAuth scopes.
- Auth0 is a pragmatic baseline because it provides a conventional external Authorization Server shape and keeps the relay out of client-registration complexity.
- The protected-resource `resource` value must match the actual public relay endpoint so issuer/audience/resource checks stay consistent.
- The prior SSE assumption is stale for this plan: the relay already implements stateless MCP over `POST /mcp`, and `/.well-known/oauth-protected-resource` is the discovery surface it owns.

## Phase 0 audit evidence

Audit date: 2026-08-12. The current branch was checked against the relay source,
CI workflow, release workflow, and the Plan 028 sandbox implementation. The
following matrix is the Phase 0 freeze record: every later implementation task
is represented by a concrete `PARTIAL` or `MISSING` gap, while existing Plan
028 capabilities are not scheduled for reimplementation.

| Gap | Status | Evidence and mapped implementation work |
| --- | --- | --- |
| P1 tool contract | PARTIAL | `packages/rust-tools/src/relay_agent/mcp.rs` has the three intended tools, input-schema validation, deterministic ordering/cache metadata, and annotations. The remaining Phase 1 work is to complete the title/schema/description/limit audit and verify the `http_fetch` mutation hint against runtime behavior. |
| P2 resource metadata and registration | PARTIAL | `transport.rs` owns `/.well-known/oauth-protected-resource` and emits issuer/resource values, but the Auth0 discovery values, `relay.coding` advertisement, PKCE/refresh metadata, user-defined-client flow, CIMD/DCR behavior, token-auth method, and live external MCP client callback still require deployment verification. No registration database is needed. |
| P3 coding authorization | PARTIAL | Remote JWT validation, issuer/audience checks, JWKS TTL/unknown-`kid` refresh, and pre-dispatch authorization exist in `transport.rs`. The current scope mapping still uses `read`/`fetch`/`execute` (around the `tools/call` authorization path), so the frozen `relay.coding` owner/capability model and negative checks remain implementation gaps. |
| P4 remote exposure | PARTIAL | Local Origin/Host checks and explicit Local/Remote configuration exist in `security.rs` and `config.rs`. Secure MCP Tunnel/direct-HTTPS deployment wiring, trusted proxy identity, concurrency/abuse controls, and live burst verification remain open. |
| P5 operations | MISSING | The relay has bounded execution output/timeouts, but Plan 029 still lacks the documented structured correlation, privacy-safe outcome/latency logging, redaction, retention, and diagnostic taxonomy. |
| P6 live coding E2E | MISSING | No repository evidence records a Business/Enterprise/Edu developer-mode app, successful Scan Tools, OAuth refresh, `relay.coding` grant, or the inspect-edit-build and negative boundary scenarios. This must be validated in the live product, not replaced with unit tests. |
| P7 published-app lifecycle | MISSING | No canonical tool-catalog hash/snapshot or external MCP client refresh/action-review procedure is recorded. The stable public tool names are present, but publication-change safety is not yet implemented/documented. |
| P8 release/conformance gates | PARTIAL | `.github/workflows/ci.yml` already runs locked Rust formatting/check/clippy/audit and makes release depend on JS/Rust jobs. Deterministic connector checks, explicit warning/bypass review, and the remaining Plan 029 conformance assertions are not yet present. |

The matrix also records two Phase 0 freeze decisions. Plan 029 schedules no
Rust or JavaScript unit-test work and adds no command denylist or other
restriction merely because the coding terminal can edit/run code. Existing
Plan 028 filesystem, process, privilege, container, timeout, and output
boundaries remain authoritative; ordinary shells, interpreters, Git, package
managers, compilers, and in-workspace file mutation remain in scope.

## Phase 9 production-readiness evidence

Audit date: 2026-08-12. The repository gates passed:

- `scripts/phase6-external-mcp-e2e.sh` (static acceptance; live probe unavailable)
- `scripts/phase7-external-mcp-contract.sh`
- `scripts/phase8-zero-bypass.sh`
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --all-features --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo audit`

The catalog snapshot hash remains
`6fd916285e1c8f1f0f7195ff7ef8bd696590ca1f0ad0ebb1b0c49d562a190ea6`.
No deployed relay URL, external MCP client workspace, OAuth tenant, or callback
credentials were available. Therefore Scan Tools, OAuth registration/PKCE/
refresh/OIDC, live coding E2E, and live negative-boundary evidence are
explicitly unverified; this is an accepted release limitation and does not
constitute fabricated evidence.
