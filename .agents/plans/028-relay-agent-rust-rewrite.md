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
- [ ] Remote `tools/call` requires valid OAuth authorization before any side effect.
- [ ] Scope authorization must be enforced independently of tool arguments.

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

- [ ] Refuse startup when running as UID 0/root on platforms with a reliable UID check.
- [ ] Production deployment MUST use a dedicated unprivileged OS/container user.
- [ ] No `sudo`, `su`, `doas`, `pkexec`, `runas`, or equivalent helper may be invoked by relay code.
- [ ] Reject those helpers as requested executables and through path aliases/wrappers.
- [ ] Reject shell/interpreter escape forms such as `sh -c`, `bash -c`, `zsh -c`, `cmd /c`, PowerShell `-Command`, and language `-c`/eval forms unless explicitly reviewed for a non-shell operation.
- [ ] No generic shell MCP tool.
- [ ] MCP callers cannot select arbitrary executable paths.
- [ ] Command authorization comes only from a server-controlled allowlist/policy, never from the request itself.
- [ ] Policy covers executable identity/path and, where necessary, approved argument patterns.
- [ ] Reject path traversal, symlink substitution, wrapper aliases, and interpreter aliases.
- [ ] Use a minimal child environment; prevent `PATH`, loader, preload, plugin, and runtime environment injection.
- [ ] Relay binary, policy/config, sibling tools, and release artifacts MUST not be writable by the runtime user.
- [ ] Approved tools MUST not be privilege-escalation trampolines.
- [ ] Verify representative malicious command inputs manually.

**Invariant:** the caller can only request an operation already authorized by server policy. The request can never grant itself execution permission.

### 15.2 OS defense in depth

- [ ] Document filesystem ownership/permissions.
- [ ] Document optional container/OS sandboxing as defense in depth.
- [ ] No dependency on sudoers for the security model.
- [ ] Verify no code path intentionally requests elevation.

### 15.3 OAuth resource-server foundation

- [ ] Remote MCP endpoint is HTTPS-only.
- [ ] Protected Resource Metadata is implemented correctly.
- [ ] Authorization Server Metadata/issuer discovery is implemented or consumed according to the selected provider.
- [ ] Stable resource/audience identifier is configured.
- [ ] JWT access tokens are verified using issuer JWKS with key rotation/caching, or trusted opaque-token introspection is used.
- [ ] Validate issuer, audience/resource, expiry, not-before, token type, and allowed signing algorithm.
- [ ] No shared-secret custom JWT scheme as the long-term connector security boundary.
- [ ] Define least-privilege scopes for read/search/fetch/execute operations.
- [ ] Map scopes/claims to tool permissions server-side.
- [ ] Invalid/missing credentials return correct `401` and `WWW-Authenticate` behavior.
- [ ] Tokens, authorization codes, refresh tokens, client secrets, and Authorization headers never enter logs, URLs, query strings, tool arguments, or errors.
- [ ] Remote mode cannot fall back to local no-auth behavior.

### 15.4 Connector compatibility foundation

- [ ] Authorization Code + PKCE S256.
- [ ] No PKCE downgrade.
- [ ] Exact redirect URI validation.
- [ ] Transaction-specific state/CSRF protection.
- [ ] Authorization-server mix-up protection.
- [ ] No implicit grant.
- [ ] No resource-owner-password grant.
- [ ] Provider credentials remain deployment secrets.
- [ ] Refresh tokens remain outside the MCP tool layer.

## Phase 16 — OAuth protocol/security completion — [ ] NOT STARTED

**Goal:** make the remote connector path standards-compliant and fail-closed under real OAuth/MCP attack conditions. Phase 16 is mandatory even if a provider happens to work with a happy-path token.

### 16.1 Resource-server authorization

- [ ] Define the exact protected resource URI and use it consistently in metadata, token audience/resource validation, and connector configuration.
- [ ] Support issuer discovery only from an explicit trusted issuer/provider configuration; never accept an issuer supplied by an unauthenticated request as trusted configuration.
- [ ] Validate JWT `iss` exactly against the configured issuer.
- [ ] Validate `aud` and/or RFC-compliant resource indicator according to the selected provider contract.
- [ ] Reject missing, malformed, expired, not-yet-valid, wrong-issuer, wrong-audience, or wrong-resource tokens.
- [ ] Restrict accepted signing algorithms to the configured safe set; reject `none` and algorithm confusion.
- [ ] Fetch JWKS over HTTPS from trusted issuer metadata only.
- [ ] Cache JWKS with bounded lifetime and safe refresh-on-unknown-key behavior; avoid attacker-controlled refresh loops.
- [ ] Handle key rotation without accepting stale/unknown keys indefinitely.
- [ ] If introspection is used instead of JWT/JWKS, require TLS, provider authentication, active-token response, audience/resource, expiry, and scope validation.
- [ ] Never mix JWT and opaque-token trust paths accidentally.

### 16.2 Scope-to-tool authorization

- [ ] Create a closed server-side map of scopes → tool permissions.
- [ ] Default deny when scope is missing, malformed, or unknown.
- [ ] `terminal_exec` requires an explicit execute scope.
- [ ] `http_fetch` requires its own explicit scope.
- [ ] `web_search` requires its own explicit scope.
- [ ] Do not let a broad scope implicitly authorize a more privileged operation unless explicitly documented.
- [ ] Scope checks happen before child-process spawn/network side effect.
- [ ] Scope checks are independent from JSON Schema validation.
- [ ] Tool arguments MUST NOT contain permission/grant fields that can override authorization.
- [ ] Reject scope escalation attempts and duplicated/conflicting authorization claims.

### 16.3 OAuth error and challenge behavior

- [ ] Return `401` for missing/invalid authentication.
- [ ] Return `403` for authenticated callers lacking required tool scope.
- [ ] Emit standards-compliant `WWW-Authenticate` challenges.
- [ ] Include protected-resource metadata location where appropriate.
- [ ] Do not leak token validation internals, issuer details, keys, or claims in errors.
- [ ] Never return access/refresh tokens in response bodies unless the endpoint is explicitly an OAuth endpoint designed to do so.

### 16.4 PKCE / browser-flow security

- [ ] Require PKCE S256 for public clients.
- [ ] Reject `plain` or missing PKCE where required.
- [ ] Bind authorization request and callback using unpredictable transaction state.
- [ ] Validate state before exchanging authorization code.
- [ ] Exact-match redirect URIs; no prefix, wildcard, or substring matching.
- [ ] Authorization codes are single-use and short-lived when the resource server owns authorization transactions.
- [ ] Protect callback endpoints against login CSRF and authorization-code injection.
- [ ] Pin trusted authorization servers to prevent mix-up attacks.
- [ ] Do not accept authorization-server URLs from MCP tool arguments.

### 16.5 Remote deployment boundary

- [ ] Explicit `LOCAL` vs `REMOTE` security mode; no implicit auth downgrade based only on whether an environment variable exists.
- [ ] Remote mode refuses plaintext HTTP except explicitly loopback-only internal hops behind a trusted TLS terminator.
- [ ] Trusted proxy headers are accepted only from configured proxies.
- [ ] External scheme/host/resource metadata cannot be attacker-controlled.
- [ ] Local no-auth mode binds only to loopback and cannot be configured to publicly bind accidentally.
- [ ] Remote mode cannot call local-mode bypass paths.
- [ ] Add rate limiting/concurrency controls to remote `tools/call` independently of OAuth token validity.

### 16.6 OAuth secrets and observability

- [ ] Secrets only enter through deployment secret stores/environment injection.
- [ ] No client secret, private key, token, authorization code, or verifier is committed.
- [ ] Logs redact Authorization headers and sensitive query parameters.
- [ ] Metrics/traces contain stable non-secret identifiers only.
- [ ] Error messages are safe for untrusted remote clients.
- [ ] Add audit events for authentication failure, authorization denial, and privileged tool invocation without logging tokens or sensitive command contents.

### 16.7 OAuth manual attack review

- [ ] Wrong issuer.
- [ ] Wrong audience/resource.
- [ ] Expired token.
- [ ] Future `nbf`.
- [ ] Invalid signature.
- [ ] Unknown/rotated key.
- [ ] Algorithm confusion / `none`.
- [ ] Missing execute scope.
- [ ] Attempt to inject scope through tool arguments.
- [ ] PKCE downgrade.
- [ ] State mismatch.
- [ ] Redirect URI prefix/wildcard bypass.
- [ ] Authorization-server mix-up.
- [ ] Remote endpoint without TLS.
- [ ] Local endpoint exposed on non-loopback address.
- [ ] Credential leakage in logs/errors/URLs.

**Phase 16 acceptance:** remote MCP access is fail-closed, standards-based, least-privilege, and independently authorized before any tool side effect.

## Phase 17 — Zero-bypass Rust quality, lint, dependency, and CI hardening — [ ] NOT STARTED

**Goal:** make CI a strict quality/security gate. A green build MUST mean there are no accepted warnings or hidden lint/security bypasses.

### 17.1 Formatting

- [ ] `cargo fmt --all -- --check` passes.
- [ ] No formatting exception/configuration is added to hide malformed code.
- [ ] Formatting changes are committed rather than bypassed in CI.

### 17.2 Clippy

- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] Every Clippy warning is fixed at source.
- [ ] No `#[allow(clippy::...)]` is added merely to make CI green.
- [ ] No crate/module/function uses broad `#[allow(clippy::all)]`.
- [ ] No `#![allow(warnings)]` or equivalent global suppression.
- [ ] No `#[expect(...)]` is used to hide unresolved warnings.
- [ ] No `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`/config override downgrades `-D warnings`.
- [ ] No CI step changes lint level to `warn`, `allow`, or removes Clippy checks.
- [ ] Security-relevant Clippy warnings are treated as blockers.

### 17.3 Compiler warnings

- [ ] `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets --all-features` passes where supported by the project.
- [ ] No `#[allow(warnings)]`.
- [ ] No warning suppression via module attributes, Cargo config, build scripts, or environment variables.
- [ ] No unused/dead code warning is ignored; remove dead code or document a justified compiler-supported reason without suppressing unrelated warnings.

### 17.4 Dependency and audit hygiene

- [ ] `cargo audit` passes with zero unreviewed vulnerabilities.
- [ ] `cargo deny` or equivalent dependency policy check is added if already supported by repository tooling.
- [ ] Review duplicate/advisory-risk dependencies where practical.
- [ ] Pin/lock production dependencies through `Cargo.lock` where workspace policy requires it.
- [ ] No dependency is added solely to bypass an existing lint/security rule.
- [ ] Review unsafe Rust introduced by dependencies or project code; minimize and document any required `unsafe`.
- [ ] No yanked/insecure dependency is knowingly accepted without an explicit documented exception.

### 17.5 CI script integrity

- [ ] Every required command uses fail-fast semantics (`set -euo pipefail` where shell applies).
- [ ] No `|| true`, `; true`, `continue-on-error: true`, `|| :`, or equivalent is used on required lint/security commands.
- [ ] No command output is piped through a filter that can hide a non-zero exit status.
- [ ] Pipeline preserves exit codes (`pipefail` where applicable).
- [ ] No required security/lint job is conditionally skipped for the relay branch.
- [ ] No `if: false`, branch-specific bypass, manual-only gate, or path filter accidentally excludes relay changes from required checks.
- [ ] Required jobs are branch-protection-compatible and must be green before merge.
- [ ] CI does not silently fall back to a weaker local command.

### 17.6 Static security searches

Add a deterministic CI/source audit for:

- [ ] `sudo`, `su`, `doas`, `pkexec`, `runas` execution paths.
- [ ] generic shell invocation and shell-string interpolation.
- [ ] request-derived executable allowlists.
- [ ] `--no-guard` in MCP/relay execution paths.
- [ ] wildcard Origin/CORS.
- [ ] arbitrary `base_url` network pivots.
- [ ] insecure HTTP remote OAuth endpoints.
- [ ] hardcoded OAuth secrets/tokens/private keys.
- [ ] `Authorization` logging.
- [ ] `#[allow(`, `#![allow(`, `allow(warnings)`, and broad Clippy suppression in production Rust code.
- [ ] `continue-on-error`, `|| true`, `; true`, and equivalent required-check bypasses in workflows.
- [ ] committed binaries or generated artifacts that replace source-built release outputs.

False positives MUST be fixed by narrowing the implementation/search rule or documenting a reviewed non-production fixture; never by disabling the entire check.

### 17.7 Release reproducibility

- [ ] Release workflow uses Cargo directly.
- [ ] No `@yao-pkg/pkg`, Node runtime packaging, or hidden JS build step remains.
- [ ] Build uses a clean Rust toolchain/environment.
- [ ] Release artifact is produced from the exact reviewed commit.
- [ ] Checksums/signing metadata are generated where the repository release policy requires them.
- [ ] No prebuilt local binary is copied into the release artifact.
- [ ] Verify the binary has no Node/V8/libnode runtime dependency.
- [ ] Verify expected target triples and artifact names.

### 17.8 Manual no-bypass review

- [ ] Search all Rust crates/modules for lint suppression.
- [ ] Search all workflows for failure masking.
- [ ] Search scripts/Makefiles/package scripts for ignored exit codes.
- [ ] Review Cargo config, rust-toolchain files, build scripts, and workspace lints.
- [ ] Verify CI commands executed locally are identical in enforcement strength to CI commands.
- [ ] Confirm every security gate is fail-closed.

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

- [ ] Relay is 100% Rust and standalone.
- [ ] MCP Streamable HTTP is the sole relay protocol.
- [ ] Local mode is loopback-only and non-root.
- [ ] No sudo/privilege-escalation path exists.
- [ ] Command authorization is server-controlled and deny-by-default.
- [ ] SSRF/resource/process/output policies are enforced server-side.
- [ ] Remote mode is HTTPS + OAuth protected.
- [ ] OAuth uses standards-based resource-server validation and least-privilege scopes.
- [ ] OAuth cannot grant execution beyond the server-side command policy.
- [ ] No credentials/secrets/tokens are logged or exposed.
- [ ] No legacy Node relay or `@yao-pkg/pkg` remains.
- [ ] Clippy/compiler warnings are zero with no suppression/bypass.
- [ ] fmt, clippy, audit, dependency, CI-integrity, and security-search gates pass.
- [ ] No required CI command ignores failures or uses `continue-on-error`.
- [ ] Phase 15 is complete.
- [ ] Phase 16 is complete.
- [ ] Phase 17 is complete.
- [ ] Phase 13 final E2E/release gate is complete.

## Rollback

Keep the known-good release available until the final gate passes. If any security, OAuth, privilege, lint, audit, or E2E gate fails, keep the plan `IN FLIGHT`, do not weaken the gate, and fix the underlying issue before proceeding.
