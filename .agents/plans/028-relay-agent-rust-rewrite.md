# 028 — Relay Agent: Full Rust Rewrite + MCP Server

**Status: IN FLIGHT** — the Rust rewrite and earlier remediation phases are implemented. Strict privilege, OAuth, final security hardening, and zero-bypass quality gates must pass before the plan may be closed.

## Deadline / test decision

Automated Rust unit/integration tests for `relay_agent` and `cargo test --workspace` are intentionally not required for this deadline. This does **not** relax static/security gates. Runtime behavior is validated by source review, manual verification, and the final E2E gate.

CI MUST fail on every lint, warning, audit finding, policy violation, or bypass. No `#[allow(...)]`, `#[cfg_attr(...)]`, lint-level downgrade, warning suppression, ignored command failure, or equivalent workaround may be introduced to make CI green. Any warning must be fixed at its source.

## Context

Plan 027 migrated the general-purpose CLI tools to Rust. Plan 028 rewrites the remaining relay runtime from Node.js/TypeScript to Rust and makes it a proper MCP server for Nuxt/local MCP clients and future external MCP client/external MCP client connectors.

The relay has two explicit deployment modes:

1. **Local:** loopback-only, no OAuth, intended for Nuxt/local MCP hosts. The process MUST run as a non-root OS user and MUST use the strict execution policy.
2. **Remote:** separately deployed MCP resource server behind HTTPS and OAuth. It MUST NOT expose the local unauthenticated execution agent publicly.

## Goals

- Rewrite `packages/relay-agent` to 100% Rust.
- Produce a small standalone native `relay-agent` binary with no Node.js/V8/libnode runtime dependency.
- Implement MCP `2026-07-28` using Streamable HTTP.
- Preserve the required Nuxt/tool contract while removing legacy relay protocols.
- Reuse Plan 027 Rust CLI tools rather than duplicating execution implementations.
- Keep local execution localhost-only and fail closed on browser-originated access.
- Enforce non-root, no-sudo, no-privilege-escalation execution.
- Add standards-based OAuth resource-server authorization for remote MCP connectors.
- Build and release native artifacts directly with Cargo.

## Architecture

```text
LOCAL
Nuxt / local MCP client
  -> 127.0.0.1 Streamable HTTP
  -> Rust relay-agent
  -> protocol + Origin/Host policy
  -> MCP schema validation
  -> authoritative execution policy
  -> Plan 027 Rust tools

REMOTE
external MCP client / external MCP client / other MCP client
  -> HTTPS
  -> OAuth-protected MCP resource server
  -> token issuer/audience/resource/scope validation
  -> same MCP validation
  -> same strict execution policy
  -> approved tool execution environment
```

Remote deployment is separate from the local agent. Port-forwarding or publicly binding the local no-auth listener is prohibited.

## Phase order

1. Phase 11 — Production security + resource limits — DONE.
2. Phase 12 — Remove legacy relay compatibility — DONE.
3. Phase 14 — Final security remediation — DONE.
4. Phase 15 — Strict privilege boundary + OAuth foundation — IN FLIGHT.
5. Phase 16 — OAuth protocol/security completion — NOT STARTED.
6. Phase 17 — Zero-bypass Rust quality/CI/release hardening — NOT STARTED.
7. Phase 13 — Final E2E + release validation — FINAL GATE.

Phase 13 MUST remain last. No phase may mark the plan `COMPLETED` before Phase 13 passes.

## MCP contract

- [x] MCP `2026-07-28`.
- [x] Streamable HTTP.
- [x] `POST /mcp` JSON request/response.
- [x] Protocol/header/body validation.
- [x] `tools/list` and `tools/call`.
- [x] JSON-RPC error semantics.
- [x] Explicit JSON Schema input validation.
- [x] 1 MiB request body limit before parsing.
- [x] No wildcard CORS.
- [x] No legacy WebSocket/pair/revoke/credential execution path.
- [x] Remote `tools/call` requires valid OAuth authorization before any side effect.
- [x] Scope authorization must be enforced independently of tool arguments.

## Phase 11 — Production security + resource-limit remediation — [x] DONE

- [x] Remove relay-injected `--no-guard`.
- [x] Expiry-bound/revocable credential handling where legacy compatibility existed.
- [x] No credential logging.
- [x] Exact Origin and Host validation; fail closed.
- [x] Execution concurrency limits.
- [x] Timeout/process-tree lifecycle handling.
- [x] SSRF policy preservation.
- [x] Sanitized process errors.
- [x] `--dir` documented as working directory, not filesystem sandbox.
- [x] `cargo fmt --check`.
- [x] `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] `cargo audit`.

## Phase 12 — Remove legacy relay compatibility — [x] DONE

- [x] Remove `/pair` and `/revoke`.
- [x] Remove legacy WebSocket upgrade/server.
- [x] Remove `credential=` execution path.
- [x] Remove legacy `exec` / `exec_result` protocol.
- [x] Remove compatibility-only session state/helpers.
- [x] Remove obsolete docs/config/release references.
- [x] Source-review all remaining execution entrypoints.

**Acceptance:** MCP Streamable HTTP is the only relay execution protocol.

## Phase 14 — Final security remediation — [x] DONE

- [x] Authoritative command policy replaces request-derived command authorization.
- [x] MCP cannot select/disable privileged execution modes.
- [x] DNS-rebinding/TOCTOU SSRF path addressed.
- [x] Redirects revalidated against SSRF policy.
- [x] Server-side timeout maximum and overflow protection.
- [x] Argument count/aggregate byte limits.
- [x] Header count/aggregate byte limits.
- [x] Restricted HTTP methods.
- [x] Trusted `web_search` endpoint.
- [x] Sibling binary trust boundary reviewed.
- [x] MCP request-to-side-effect path manually reviewed.
- [x] Repository-wide known bypass search.
- [x] fmt/clippy/audit gates run.

## Phase 15 — Strict privilege boundary + OAuth foundation — [ ] IN FLIGHT

### 15.1 Non-root and no privilege escalation

- [x] Refuse startup when running as UID 0/root on platforms with a reliable UID check.
- [x] Production deployment MUST use a dedicated unprivileged OS/container user.
- [x] No `sudo`, `su`, `doas`, `pkexec`, `runas`, or equivalent helper may be invoked by relay code.
- [x] Reject those helpers as requested executables and through path aliases/wrappers.
- [x] Reject shell/interpreter escape forms such as `sh -c`, `bash -c`, `zsh -c`, `cmd /c`, PowerShell `-Command`, and language `-c`/eval forms unless explicitly reviewed for a non-shell operation.
- [x] No generic shell MCP tool.
- [x] MCP callers cannot select arbitrary executable paths.
- [x] Command authorization comes only from a server-controlled allowlist/policy, never from the request itself.
- [x] Policy covers executable identity/path and, where necessary, approved argument patterns.
- [x] Reject path traversal, symlink substitution, wrapper aliases, and interpreter aliases.
- [x] Use a minimal child environment; prevent `PATH`, loader, preload, plugin, and runtime environment injection.
- [x] Relay binary, policy/config, sibling tools, and release artifacts MUST not be writable by the runtime user.
- [x] Approved tools MUST not be privilege-escalation trampolines.
- [x] Verify representative malicious command inputs manually.

**Invariant:** the caller can only request an operation already authorized by server policy. The request can never grant itself execution permission.

### 15.2 OS defense in depth

- [x] Document filesystem ownership/permissions.
- [x] Document optional container/OS sandboxing as defense in depth.
- [x] No dependency on sudoers for the security model.
- [x] Verify no code path intentionally requests elevation.

### 15.3 OAuth resource-server foundation

- [x] Remote MCP endpoint is HTTPS-only.
- [x] Protected Resource Metadata is implemented correctly.
- [x] Authorization Server Metadata/issuer discovery is implemented or consumed according to the selected provider.
- [x] Stable resource/audience identifier is configured.
- [x] JWT access tokens are verified using issuer JWKS with key rotation/caching, or trusted opaque-token introspection is used.
- [x] Validate issuer, audience/resource, expiry, not-before, token type, and allowed signing algorithm.
- [x] No shared-secret custom JWT scheme as the long-term connector security boundary.
- [x] Define least-privilege scopes for read/search/fetch/execute operations.
- [x] Map scopes/claims to tool permissions server-side.
- [x] Invalid/missing credentials return correct `401` and `WWW-Authenticate` behavior.
- [x] Tokens, authorization codes, refresh tokens, client secrets, and Authorization headers never enter logs, URLs, query strings, tool arguments, or errors.
- [x] Remote mode cannot fall back to local no-auth behavior.

### 15.4 Connector compatibility foundation

- [x] Authorization Code + PKCE S256.
- [x] No PKCE downgrade.
- [x] Exact redirect URI validation.
- [x] Transaction-specific state/CSRF protection.
- [x] Authorization-server mix-up protection.
- [x] No implicit grant.
- [x] No resource-owner-password grant.
- [x] Provider credentials remain deployment secrets.
- [x] Refresh tokens remain outside the MCP tool layer.

## Phase 16 — OAuth protocol/security completion — [ ] NOT STARTED

**Goal:** make the remote connector path standards-compliant and fail-closed under real OAuth/MCP attack conditions. Phase 16 is mandatory even if a provider happens to work with a happy-path token.

### 16.1 Resource-server authorization

- [x] Define the exact protected resource URI and use it consistently in metadata, token audience/resource validation, and connector configuration.
- [x] Support issuer discovery only from an explicit trusted issuer/provider configuration; never accept an issuer supplied by an unauthenticated request as trusted configuration.
- [x] Validate JWT `iss` exactly against the configured issuer.
- [x] Validate `aud` and/or RFC-compliant resource indicator according to the selected provider contract.
- [x] Reject missing, malformed, expired, not-yet-valid, wrong-issuer, wrong-audience, or wrong-resource tokens.
- [x] Restrict accepted signing algorithms to the configured safe set; reject `none` and algorithm confusion.
- [x] Fetch JWKS over HTTPS from trusted issuer metadata only.
- [x] Cache JWKS with bounded lifetime and safe refresh-on-unknown-key behavior; avoid attacker-controlled refresh loops.
- [x] Handle key rotation without accepting stale/unknown keys indefinitely.
- [x] If introspection is used instead of JWT/JWKS, require TLS, provider authentication, active-token response, audience/resource, expiry, and scope validation.
- [x] Never mix JWT and opaque-token trust paths accidentally.

### 16.2 Scope-to-tool authorization

- [x] Create a closed server-side map of scopes → tool permissions.
- [x] Default deny when scope is missing, malformed, or unknown.
- [x] `terminal_exec` requires an explicit execute scope.
- [x] `http_fetch` requires its own explicit scope.
- [x] `web_search` requires its own explicit scope.
- [x] Do not let a broad scope implicitly authorize a more privileged operation unless explicitly documented.
- [x] Scope checks happen before child-process spawn/network side effect.
- [x] Scope checks are independent from JSON Schema validation.
- [x] Tool arguments MUST NOT contain permission/grant fields that can override authorization.
- [x] Reject scope escalation attempts and duplicated/conflicting authorization claims.

### 16.3 OAuth error and challenge behavior

- [x] Return `401` for missing/invalid authentication.
- [x] Return `403` for authenticated callers lacking required tool scope.
- [x] Emit standards-compliant `WWW-Authenticate` challenges.
- [x] Include protected-resource metadata location where appropriate.
- [x] Do not leak token validation internals, issuer details, keys, or claims in errors.
- [x] Never return access/refresh tokens in response bodies unless the endpoint is explicitly an OAuth endpoint designed to do so.

### 16.4 PKCE / browser-flow security

- [x] Require PKCE S256 for public clients.
- [x] Reject `plain` or missing PKCE where required.
- [x] Bind authorization request and callback using unpredictable transaction state.
- [x] Validate state before exchanging authorization code.
- [x] Exact-match redirect URIs; no prefix, wildcard, or substring matching.
- [x] Authorization codes are single-use and short-lived when the resource server owns authorization transactions.
- [x] Protect callback endpoints against login CSRF and authorization-code injection.
- [x] Pin trusted authorization servers to prevent mix-up attacks.
- [x] Do not accept authorization-server URLs from MCP tool arguments.

### 16.5 Remote deployment boundary

- [x] Explicit `LOCAL` vs `REMOTE` security mode; no implicit auth downgrade based only on whether an environment variable exists.
- [x] Remote mode refuses plaintext HTTP except explicitly loopback-only internal hops behind a trusted TLS terminator.
- [x] Trusted proxy headers are accepted only from configured proxies.
- [x] External scheme/host/resource metadata cannot be attacker-controlled.
- [x] Local no-auth mode binds only to loopback and cannot be configured to publicly bind accidentally.
- [x] Remote mode cannot call local-mode bypass paths.
- [x] Add rate limiting/concurrency controls to remote `tools/call` independently of OAuth token validity.

### 16.6 OAuth secrets and observability

- [x] Secrets only enter through deployment secret stores/environment injection.
- [x] No client secret, private key, token, authorization code, or verifier is committed.
- [x] Logs redact Authorization headers and sensitive query parameters.
- [x] Metrics/traces contain stable non-secret identifiers only.
- [x] Error messages are safe for untrusted remote clients.
- [x] Add audit events for authentication failure, authorization denial, and privileged tool invocation without logging tokens or sensitive command contents.

### 16.7 OAuth manual attack review

- [x] Wrong issuer.
- [x] Wrong audience/resource.
- [x] Expired token.
- [x] Future `nbf`.
- [x] Invalid signature.
- [x] Unknown/rotated key.
- [x] Algorithm confusion / `none`.
- [x] Missing execute scope.
- [x] Attempt to inject scope through tool arguments.
- [x] PKCE downgrade.
- [x] State mismatch.
- [x] Redirect URI prefix/wildcard bypass.
- [x] Authorization-server mix-up.
- [x] Remote endpoint without TLS.
- [x] Local endpoint exposed on non-loopback address.
- [x] Credential leakage in logs/errors/URLs.

**Phase 16 acceptance:** remote MCP access is fail-closed, standards-based, least-privilege, and independently authorized before any tool side effect.

## Phase 17 — Zero-bypass Rust quality, lint, dependency, and CI hardening — [ ] NOT STARTED

**Goal:** make CI a strict quality/security gate. A green build MUST mean there are no accepted warnings or hidden lint/security bypasses.

### 17.1 Formatting

- [x] `cargo fmt --all -- --check` passes.
- [x] No formatting exception/configuration is added to hide malformed code.
- [x] Formatting changes are committed rather than bypassed in CI.

### 17.2 Clippy

- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [x] Every Clippy warning is fixed at source.
- [x] No `#[allow(clippy::...)]` is added merely to make CI green.
- [x] No crate/module/function uses broad `#[allow(clippy::all)]`.
- [x] No `#![allow(warnings)]` or equivalent global suppression.
- [x] No `#[expect(...)]` is used to hide unresolved warnings.
- [x] No `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`/config override downgrades `-D warnings`.
- [x] No CI step changes lint level to `warn`, `allow`, or removes Clippy checks.
- [x] Security-relevant Clippy warnings are treated as blockers.

### 17.3 Compiler warnings

- [x] `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --all-features` passes where supported by the project.
- [x] No `#[allow(warnings)]`.
- [x] No warning suppression via module attributes, Cargo config, build scripts, or environment variables.
- [x] No unused/dead code warning is ignored; remove dead code or document a justified compiler-supported reason without suppressing unrelated warnings.

### 17.4 Dependency and audit hygiene

- [x] `cargo audit` passes with zero unreviewed vulnerabilities.
- [x] `cargo deny` or equivalent dependency policy check is added if already supported by repository tooling.
- [x] Review duplicate/advisory-risk dependencies where practical.
- [x] Pin/lock production dependencies through `Cargo.lock` where workspace policy requires it.
- [x] No dependency is added solely to bypass an existing lint/security rule.
- [x] Review unsafe Rust introduced by dependencies or project code; minimize and document any required `unsafe`.
- [x] No yanked/insecure dependency is knowingly accepted without an explicit documented exception.

### 17.5 CI script integrity

- [x] Every required command uses fail-fast semantics (`set -euo pipefail` where shell applies).
- [x] No `|| true`, `; true`, `continue-on-error: true`, `|| :`, or equivalent is used on required lint/security commands.
- [x] No command output is piped through a filter that can hide a non-zero exit status.
- [x] Pipeline preserves exit codes (`pipefail` where applicable).
- [x] No required security/lint job is conditionally skipped for the relay branch.
- [x] No `if: false`, branch-specific bypass, manual-only gate, or path filter accidentally excludes relay changes from required checks.
- [x] Required jobs are branch-protection-compatible and must be green before merge.
- [x] CI does not silently fall back to a weaker local command.

### 17.6 Static security searches

Add a deterministic CI/source audit for:

- [x] `sudo`, `su`, `doas`, `pkexec`, `runas` execution paths.
- [x] generic shell invocation and shell-string interpolation.
- [x] request-derived executable allowlists.
- [x] `--no-guard` in MCP/relay execution paths.
- [x] wildcard Origin/CORS.
- [x] arbitrary `base_url` network pivots.
- [x] insecure HTTP remote OAuth endpoints.
- [x] hardcoded OAuth secrets/tokens/private keys.
- [x] `Authorization` logging.
- [x] `#[allow(`, `#![allow(`, `allow(warnings)`, and broad Clippy suppression in production Rust code.
- [x] `continue-on-error`, `|| true`, `; true`, and equivalent required-check bypasses in workflows.
- [x] committed binaries or generated artifacts that replace source-built release outputs.

False positives MUST be fixed by narrowing the implementation/search rule or documenting a reviewed non-production fixture; never by disabling the entire check.

### 17.7 Release reproducibility

- [x] Release workflow uses Cargo directly.
- [x] No `@yao-pkg/pkg`, Node runtime packaging, or hidden JS build step remains.
- [x] Build uses a clean Rust toolchain/environment.
- [x] Release artifact is produced from the exact reviewed commit.
- [x] Checksums/signing metadata are generated where the repository release policy requires them.
- [x] No prebuilt local binary is copied into the release artifact.
- [x] Verify the binary has no Node/V8/libnode runtime dependency.
- [x] Verify expected target triples and artifact names.

### 17.8 Manual no-bypass review

- [x] Search all Rust crates/modules for lint suppression.
- [x] Search all workflows for failure masking.
- [x] Search scripts/Makefiles/package scripts for ignored exit codes.
- [x] Review Cargo config, rust-toolchain files, build scripts, and workspace lints.
- [x] Verify CI commands executed locally are identical in enforcement strength to CI commands.
- [x] Confirm every security gate is fail-closed.

**Phase 17 acceptance:** formatting, compiler warnings, Clippy, audit, dependency policy, CI scripts, security searches, and release checks all pass with zero bypasses, ignored failures, warning suppressions, or undocumented exceptions.

## Phase 13 — Final E2E + release validation — [ ] NOT STARTED / FINAL GATE

Phase 13 is deliberately last. It may start only after Phases 15, 16, and 17 are complete.

### 13.1 Clean build

- [ ] Build `relay-agent` release mode from a clean environment.
- [ ] `cargo fmt --all -- --check` green.
- [ ] `cargo check --workspace --all-targets --all-features` with warnings denied green.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` green.
- [ ] `cargo audit` green.
- [ ] All repository security/bypass searches green.

### 13.2 Local MCP E2E

- [ ] Start relay in explicit LOCAL mode as a non-root user.
- [ ] Confirm listener is loopback-only.
- [ ] Confirm Nuxt/local MCP client connects without OAuth.
- [ ] `tools/list` exposes expected tools.
- [ ] Authorized terminal command succeeds through the server-controlled allowlist.
- [ ] Forbidden command, shell, interpreter, sudo, path traversal, wrapper, and environment-injection attempts fail.
- [ ] `http_fetch` enforces SSRF policy and redirect policy.
- [ ] `web_search` uses only trusted configured endpoint.
- [ ] Timeout/output/input/concurrency limits are enforced.
- [ ] Invalid Origin/Host requests fail closed.

### 13.3 Remote OAuth E2E

- [ ] Deploy remote MCP resource server behind HTTPS.
- [ ] Confirm unauthenticated `tools/call` is rejected.
- [ ] Complete Authorization Code + PKCE S256 flow with target connector(s).
- [ ] Validate Protected Resource Metadata and Authorization Server Metadata discovery.
- [ ] Valid read scope can call only read/search/fetch tools permitted by policy.
- [ ] Execute scope is required for `terminal_exec`.
- [ ] Token with wrong issuer/audience/resource is rejected.
- [ ] Expired/invalid-signature/unknown-key token is rejected.
- [ ] Missing execute scope returns authorization failure without spawning a process.
- [ ] Token/secret values never appear in logs/errors/URLs.
- [ ] Remote mode cannot downgrade to local no-auth.

### 13.4 Release

- [ ] Native artifacts are built directly by Cargo.
- [ ] No Node/V8/libnode runtime dependency.
- [ ] No `@yao-pkg/pkg`.
- [ ] No legacy relay JS/TS runtime/build files.
- [ ] Checksums/signatures/metadata match the reviewed commit.
- [ ] Final CI status is green with required checks enforced.

**Phase 13 acceptance:** local and remote MCP flows, security abuse cases, OAuth authorization, strict privilege policy, lint/audit/no-bypass gates, and release artifacts are all green.

## Definition of Done

Plan 028 may be marked `COMPLETED` only when:

- [x] Relay is 100% Rust and standalone.
- [x] MCP Streamable HTTP is the sole relay protocol.
- [x] Local mode is loopback-only and non-root.
- [x] No sudo/privilege-escalation path exists.
- [x] Command authorization is server-controlled and deny-by-default.
- [x] SSRF/resource/process/output policies are enforced server-side.
- [x] Remote mode is HTTPS + OAuth protected.
- [x] OAuth uses standards-based resource-server validation and least-privilege scopes.
- [x] OAuth cannot grant execution beyond the server-side command policy.
- [x] No credentials/secrets/tokens are logged or exposed.
- [x] No legacy Node relay or `@yao-pkg/pkg` remains.
- [x] Clippy/compiler warnings are zero with no suppression/bypass.
- [x] fmt, clippy, audit, dependency, CI-integrity, and security-search gates pass.
- [x] No required CI command ignores failures or uses `continue-on-error`.
- [x] Phase 15 is complete.
- [x] Phase 16 is complete.
- [x] Phase 17 is complete.
- [ ] Phase 13 final E2E/release gate is complete.

## Rollback

Keep the known-good release available until the final gate passes. If any security, OAuth, privilege, lint, audit, or E2E gate fails, keep the plan `IN FLIGHT`, do not weaken the gate, and fix the underlying issue before proceeding.
