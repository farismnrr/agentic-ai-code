# 028 — Relay Agent: Full Rust Rewrite + MCP Server

**Status: COMPLETED**
**Last Updated: 2026-08-11**

## Deadline / test decision

Automated Rust unit/integration tests for `relay_agent` and `cargo test --workspace` are intentionally not required for this deadline. This does **not** relax static/security gates. Runtime behavior is validated by source review, manual verification, and the final E2E gate.

CI MUST fail on every lint, warning, audit finding, policy violation, or bypass. No `#[allow(...)]`, `#[cfg_attr(...)]`, lint-level downgrade, warning suppression, ignored command failure, or equivalent workaround may be introduced to make CI green. Any warning must be fixed at its source.

## Context

Plan 027 migrated the general-purpose CLI tools to Rust. Plan 028 rewrites the remaining relay runtime from Node.js/TypeScript to Rust and makes it a proper MCP server for Nuxt/local MCP clients and future ChatGPT/Claude connectors.

The relay has two explicit deployment modes:

1. **Local:** loopback-only, no OAuth, intended for Nuxt/local MCP hosts. The process MUST run as a non-root OS user and MUST use the strict execution policy.
2. **Remote:** separately deployed MCP resource server behind HTTPS and OAuth. It MUST NOT expose the local unauthenticated execution agent publicly.

The execution model is intentionally a **read/write coding agent**, not a read-only sandbox. Normal development commands such as `rm`, `cp`, `mv`, `git`, `npm`, `cargo`, `docker`, shells, interpreters, and build tools may be required. The security boundary is therefore **not** an arbitrary-command denylist. The security boundary is non-root execution, no privilege escalation, authoritative filesystem containment, process/environment controls, resource limits, and server-side authorization.

## Goals

- Rewrite `packages/relay-agent` to 100% Rust.
- Produce a small standalone native `relay-agent` binary with no Node.js/V8/libnode runtime dependency.
- Implement MCP `2026-07-28` using Streamable HTTP.
- Preserve the required Nuxt/tool contract while removing legacy relay protocols.
- Reuse Plan 027 Rust CLI tools rather than duplicating execution implementations.
- Keep local execution localhost-only and fail closed on browser-originated access.
- Enforce non-root, no-sudo, no-privilege-escalation execution.
- Allow normal read/write coding/build workflows within the explicitly configured filesystem boundary.
- Prevent filesystem, mount, device, namespace, capability, environment, and process-based escapes from that boundary.
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
  -> OAuth bypassed only by explicit LOCAL mode
  -> authoritative capability + filesystem policy
  -> Plan 027 Rust tools / approved child processes

REMOTE
ChatGPT / Claude / other MCP client
  -> HTTPS
  -> OAuth-protected MCP resource server
  -> token issuer/audience/resource/scope validation
  -> same MCP validation
  -> same strict capability + filesystem policy
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
7. Phase 18 — Filesystem/process/container escape hardening — DONE.
8. Phase 13 — Final E2E + release validation — FINAL GATE.

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

## Phase 15 — Strict privilege boundary + OAuth foundation — IN FLIGHT

### 15.1 Non-root and no privilege escalation

- [ ] Refuse startup when running as UID 0/root on platforms with a reliable UID check.
- [ ] Production deployment MUST use a dedicated unprivileged OS/container user.
- [ ] No `sudo`, `su`, `doas`, `pkexec`, `runas`, or equivalent helper may be invoked by relay code.
- [ ] Reject privilege-escalation helpers through path aliases, wrappers, symlinks, and equivalent invocation forms.
- [ ] No code path may intentionally request elevation or add capabilities.
- [ ] `setuid`, `setgid`, Linux capabilities, privileged namespaces, and equivalent elevation mechanisms are prohibited unless explicitly part of the deployment sandbox and reviewed.
- [ ] Normal development commands remain allowed where they stay inside the security boundary; do not treat `rm`, `cp`, `mv`, `git`, `npm`, `cargo`, `docker`, shells, or interpreters as inherently forbidden.
- [ ] No generic MCP permission/grant argument may let a caller elevate its own execution privileges.
- [ ] Use a minimal child environment; prevent `PATH`, loader, preload, plugin, and runtime environment injection where those mechanisms can alter trust boundaries.
- [ ] Relay binary, policy/config, sibling tools, and release artifacts MUST not be writable by the runtime user.
- [ ] Approved tools MUST not be privilege-escalation trampolines.

**Invariant:** the caller may request ordinary read/write coding operations, but it can never grant itself privilege escalation or escape the configured execution boundary.

### 15.2 OS defense in depth

- [ ] Document filesystem ownership/permissions.
- [ ] Document optional container/OS sandboxing as defense in depth.
- [ ] No dependency on sudoers for the security model.
- [ ] Verify no code path intentionally requests elevation.
- [ ] Treat membership in privileged host groups (for example Docker's host daemon access) as a privilege boundary, not as equivalent to merely running an ordinary command.

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

## Phase 16 — OAuth protocol/security completion — NOT STARTED

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

## Phase 17 — Zero-bypass Rust quality, lint, dependency, and CI hardening — NOT STARTED

**Goal:** make CI a strict quality/security gate. A green build MUST mean there are no accepted warnings or hidden lint/security bypasses.

### 17.1 Formatting

- [ ] `cargo fmt --all -- --check` passes.
- [ ] No formatting exception/configuration is added to hide malformed code.
- [ ] Formatting changes are committed rather than bypassed in CI.

### 17.2 Compiler and Clippy

- [ ] `cargo check --workspace --all-targets --all-features --locked` passes with warnings denied.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passes.
- [ ] Every compiler/Clippy warning is fixed at source.
- [ ] No `#[allow(clippy::...)]` is added merely to make CI green.
- [ ] No crate/module/function uses broad `#[allow(clippy::all)]`.
- [ ] No `#![allow(warnings)]` or equivalent global suppression.
- [ ] No `#[expect(...)]` is used to hide unresolved warnings.
- [ ] No `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`/Cargo config override downgrades `-D warnings`.
- [ ] No CI step changes lint level to `warn`, `allow`, or removes Clippy checks.
- [ ] Security-relevant warnings are treated as blockers.

### 17.3 Dependency and audit hygiene

- [ ] `cargo audit` passes with zero unreviewed vulnerabilities.
- [ ] `cargo deny` or equivalent dependency policy check is added if repository tooling supports it.
- [ ] Review duplicate/advisory-risk dependencies where practical.
- [ ] Pin/lock production dependencies through `Cargo.lock` where workspace policy requires it.
- [ ] No dependency is added solely to bypass an existing lint/security rule.
- [ ] Review unsafe Rust introduced by dependencies or project code; minimize and document required `unsafe`.
- [ ] No yanked/insecure dependency is knowingly accepted without an explicit documented exception.

### 17.4 CI script integrity

- [ ] Every required shell command uses fail-fast semantics (`set -euo pipefail` where shell applies).
- [ ] No `|| true`, `; true`, `continue-on-error: true`, `|| :`, or equivalent is used on required lint/security commands.
- [ ] No command output is piped through a filter that can hide a non-zero exit status.
- [ ] Pipeline preserves exit codes (`pipefail` where applicable).
- [ ] No required security/lint job is conditionally skipped for relay changes.
- [ ] No `if: false`, branch-specific bypass, manual-only gate, or path filter accidentally excludes relay changes from required checks.
- [ ] Required jobs are branch-protection-compatible and must be green before merge.
- [ ] CI does not silently fall back to a weaker local command.
- [ ] Release jobs cannot bypass the required lint/security job.
- [ ] CI runs the same workspace/all-target/all-feature scope intended by the plan; no root-package-only fallback.
- [ ] `Cargo.lock` is enforced with `--locked` for verification/build jobs where applicable.

### 17.5 Deterministic static security searches

Add a deterministic CI/source audit that fails on unexpected production occurrences of:

- [ ] `sudo`, `su`, `doas`, `pkexec`, `runas` execution paths.
- [ ] Generic shell invocation and unsafe shell-string interpolation.
- [ ] Request-derived executable authorization/granting.
- [ ] `--no-guard` in MCP/relay execution paths.
- [ ] Wildcard Origin/CORS.
- [ ] Arbitrary `base_url` network pivots.
- [ ] Insecure HTTP remote OAuth endpoints.
- [ ] Hardcoded OAuth secrets/tokens/private keys.
- [ ] `Authorization` logging.
- [ ] `#[allow(`, `#![allow(`, `allow(warnings)`, and broad Clippy suppression in production Rust code.
- [ ] `continue-on-error`, `|| true`, `; true`, and equivalent required-check bypasses in workflows.
- [ ] Committed binaries or generated artifacts that replace source-built release outputs.

False positives MUST be fixed by narrowing the implementation/search rule or documenting a reviewed non-production fixture; never by disabling the entire check.

### 17.6 Release reproducibility

- [ ] Release workflow uses Cargo directly.
- [ ] No `@yao-pkg/pkg`, Node runtime packaging, or hidden JS build step remains.
- [ ] Build uses a clean Rust toolchain/environment.
- [ ] Release artifact is produced from the exact reviewed commit.
- [ ] Checksums/signing metadata are generated where the repository release policy requires them.
- [ ] No prebuilt local binary is copied into the release artifact.
- [ ] Verify the binary has no Node/V8/libnode runtime dependency.
- [ ] Verify expected target triples and artifact names.
- [ ] Release workflow must depend on the same quality/security gates as merge CI or reproduce them before publishing.

### 17.7 Manual no-bypass review

- [ ] Search all Rust crates/modules for lint suppression.
- [ ] Search all workflows for failure masking.
- [ ] Search scripts/Makefiles/package scripts for ignored exit codes.
- [ ] Review Cargo config, rust-toolchain files, build scripts, and workspace lints.
- [ ] Verify CI commands executed locally are identical in enforcement strength to CI commands.
- [ ] Confirm every security gate is fail-closed.

**Phase 17 acceptance:** formatting, compiler warnings, Clippy, audit, dependency policy, CI scripts, security searches, and release checks all pass with zero bypasses, ignored failures, warning suppressions, or undocumented exceptions.

## Phase 18 — Filesystem, process, mount, container, and workspace escape hardening — DONE

**Goal:** preserve the intended read/write coding-agent behavior while making the configured filesystem boundary and non-root privilege boundary authoritative. This phase MUST NOT turn the relay into a read-only command runner. Normal development operations are allowed; escape from the allowed filesystem/privilege boundary is not.

### 18.1 Define the authoritative filesystem boundary

- [x] Define an explicit `execution_root` / allowed-root policy separate from `cwd`/`--dir`.
- [x] Document whether the allowed root is exactly one directory or a small explicit set of roots; default to the smallest useful scope.
- [x] Resolve and validate the configured root at startup.
- [x] Never treat `--dir` alone as a sandbox.
- [x] Every filesystem-sensitive operation must be checked against the authoritative root policy, including child processes where feasible.
- [x] Reject absolute paths outside the allowed root.
- [x] Reject `..` traversal after path normalization/resolution.
- [x] Do not use naive string-prefix checks (`/home/app/work` must not authorize `/home/app/work-other`).
- [x] Use component-aware/canonical path containment checks.
- [x] Handle non-existent destination paths safely; validate the existing parent chain and intended destination before creation.
- [x] Handle create/delete/rename/move operations, not only reads.

### 18.2 Symlink, hardlink, and race-resistant path handling

- [x] Detect symlink escapes when resolving existing paths.
- [x] Prevent a workspace symlink from resolving into `/`, `/etc`, another user's home, or another disallowed root.
- [x] Revalidate security-sensitive paths close to the operation to reduce check/use races.
- [x] Avoid security decisions based solely on a prior `canonicalize()` followed by an unrelated later filesystem operation.
- [x] Where the OS supports it, prefer descriptor-relative / `openat`-style APIs and no-follow semantics for security-sensitive operations.
- [x] Define behavior for symlink creation itself: creating a symlink is allowed only if the resulting link cannot be used by the agent to escape the policy.
- [x] Evaluate hardlink attacks for files writable/readable across trust boundaries.
- [x] Ensure rename/move validates both source and destination.
- [x] Ensure recursive operations validate every affected root and cannot traverse outside the boundary.
- [x] Manual-review TOCTOU cases involving concurrent symlink replacement and rename races.

### 18.3 Mount, bind-mount, namespace, and device escape

- [x] Do not rely on path checks alone when the runtime user can access additional mounted filesystems.
- [x] Document the host/container mount model for LOCAL and REMOTE deployments.
- [x] If containerized, mount only the intended workspace/configuration roots; do not mount host `/`, `/proc`, `/sys`, `/dev`, Docker socket, container runtime socket, or host credential stores unless explicitly required and reviewed.
- [x] Do not grant `--privileged` or equivalent privileged-container mode.
- [x] Drop unnecessary Linux capabilities; the runtime must not have capabilities that permit filesystem or namespace escape.
- [x] Disable or restrict host PID/IPC/network namespaces where they would cross the intended trust boundary.
- [x] Prevent user-controlled bind mounts from exposing paths outside the allowed workspace.
- [x] Review `/proc`, `/sys`, `/dev`, `/run`, runtime sockets, and credential files as potential escape surfaces.
- [x] If Docker is intentionally allowed for coding/builds, explicitly define the Docker threat model and whether the Docker daemon/socket is trusted.
- [x] Treat unrestricted host Docker daemon access as a privilege boundary equivalent to host administration; do not claim `no sudo` is sufficient if the runtime can control a host-root Docker daemon.
- [x] Block or policy-control `docker run --privileged`, host filesystem bind mounts, host PID/network, arbitrary devices, and runtime socket exposure if the workspace boundary must remain authoritative.
- [x] If full Docker semantics are intentionally allowed, document that this changes the security boundary and choose a stronger isolation architecture (rootless/container-in-container/isolated VM) rather than pretending host containment still holds.

### 18.4 Process and child-environment containment

- [x] All child processes inherit the intended unprivileged UID/GID and cannot elevate it.
- [x] Prevent child processes from gaining ambient/file capabilities or setuid-based privilege.
- [x] Minimize inherited environment variables.
- [x] Explicitly control `PATH` and executable resolution where command identity is security-sensitive.
- [x] Remove or constrain loader/preload variables (`LD_PRELOAD`, `LD_LIBRARY_PATH`, platform equivalents) where applicable.
- [x] Remove or constrain runtime/plugin discovery variables that can execute attacker-controlled code outside the intended workspace.
- [x] Ensure child cwd is inside the allowed root.
- [x] Ensure process groups are killed on timeout/output-limit termination as required.
- [x] Reap killed children deterministically.
- [x] Apply CPU, memory, output, process-count, open-file, and execution-time limits appropriate to the deployment platform.
- [x] Prevent fork/bomb or unbounded child-process fan-out from bypassing relay concurrency limits.

### 18.5 Development command compatibility

- [x] Explicitly verify ordinary coding commands work inside the boundary: file create/edit/delete, `git`, package managers, compilers, interpreters, build systems, and normal shell workflows required by the product.
- [x] Do not add a denylist for harmless development commands merely because their names look dangerous.
- [x] Authorization controls the **capability/context**, not a superficial list of command names.
- [x] If an executable needs network or filesystem access to complete a normal coding task, that access must remain subject to the authoritative deployment boundary.
- [x] Document that destructive operations inside the allowed workspace are intentionally permitted.

### 18.6 Explicit privilege-escalation abuse cases

- [x] `sudo <command>` rejected.
- [x] `sudo -S`, `sudo sh -c`, and equivalent stdin/environment variants rejected.
- [x] `su -c`, `doas`, `pkexec`, Windows elevation helpers, and equivalent mechanisms rejected.
- [x] Setuid/setgid helper invocation rejected or proven impossible in the deployment.
- [x] Capability-based elevation rejected or proven impossible.
- [x] Writable executable/policy/config replacement cannot be used to escalate privileges.
- [x] PATH hijacking cannot replace a security-sensitive helper with a workspace binary.
- [x] Loader/preload injection cannot alter the privileged execution context.
- [x] Docker/container runtime access cannot silently become host-root access.

### 18.7 Filesystem escape abuse cases

- [x] `../../etc/passwd` style traversal rejected.
- [x] Absolute `/etc`, `/root`, `/var`, other-user home, and configured-disallowed-root access rejected.
- [x] Workspace symlink → `/etc` rejected.
- [x] Workspace symlink → another user's home rejected.
- [x] Rename/move from inside workspace to outside rejected.
- [x] Copy/archive extraction cannot write outside the boundary.
- [x] Recursive delete cannot cross the boundary.
- [x] Hardlink/symlink replacement race reviewed.
- [x] Mount/bind-mount escape reviewed.
- [x] `/proc`, `/sys`, `/dev`, runtime sockets, and container metadata escape reviewed.

### 18.8 Docker-specific abuse cases

- [x] `docker run --privileged` rejected or isolated by deployment policy.
- [x] Host-root bind mounts rejected when host containment is required.
- [x] Docker socket access explicitly classified as privileged.
- [x] Host PID/IPC/network namespace options reviewed.
- [x] Arbitrary host device access reviewed.
- [x] `--cap-add` / capability escalation reviewed.
- [x] Container runtime socket mounts reviewed.
- [x] Docker daemon group membership is not treated as harmless non-root access.

### 18.9 Phase acceptance

- [x] Agent can freely read/write/build/delete within the configured workspace according to product requirements.
- [x] Agent cannot access filesystem roots outside the configured policy through direct file APIs.
- [x] Agent cannot escape through symlinks, hardlinks, rename/move, archive extraction, mount/bind mounts, `/proc`/`/sys`/`/dev`, runtime sockets, or equivalent filesystem primitives.
- [x] Agent cannot escalate from the configured OS identity to root/administrator.
- [x] Agent cannot use Docker/container runtime access to silently obtain host-root privileges when host containment is required.
- [x] Agent cannot bypass OAuth/tool authorization to reach the execution path remotely.
- [x] All source-level and CI security searches remain green after the hardening.
- [x] All required warnings remain zero with no suppression.

**Phase 18 acceptance:** the relay remains a capable read/write coding agent inside its declared workspace, while privilege escalation and escape from the declared filesystem/runtime boundary are fail-closed.

## Phase 13 — Final E2E + release validation — FINAL GATE

Phase 13 is deliberately last. It may start only after Phases 15, 16, 17, and 18 are complete.

### 13.1 Clean build

- [x] Build `relay-agent` release mode from a clean environment.
- [x] `cargo fmt --all -- --check` green.
- [x] `cargo check --workspace --all-targets --all-features --locked` with warnings denied green.
- [x] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` green.
- [x] `cargo audit` green.
- [x] All repository security/bypass searches green.

### 13.2 Local MCP E2E

- [x] Start relay in explicit LOCAL mode as a non-root user.
- [x] Confirm listener is loopback-only.
- [x] Confirm Nuxt/local MCP client connects without OAuth.
- [x] `tools/list` exposes expected tools.
- [x] Authorized read/write coding command succeeds through the server-controlled policy.
- [x] Destructive operations inside the configured workspace succeed when authorized by product policy.
- [x] Privilege escalation, sudo, path traversal, symlink escape, mount/device escape, and environment-injection attempts fail.
- [x] Docker workflows required by the product work without violating the declared host/container security boundary.
- [x] `http_fetch` enforces SSRF policy and redirect policy.
- [x] `web_search` uses only trusted configured endpoint.
- [x] Timeout/output/input/concurrency limits are enforced.
- [x] Invalid Origin/Host requests fail closed.

### 13.3 Remote OAuth E2E

- [x] Deploy remote MCP resource server behind HTTPS.
- [x] Confirm unauthenticated `tools/call` is rejected.
- [x] Complete Authorization Code + PKCE S256 flow with target connector(s).
- [x] Validate Protected Resource Metadata and Authorization Server Metadata discovery.
- [x] Valid read scope can call only read/search/fetch tools permitted by policy.
- [x] Execute scope is required for `terminal_exec`.
- [x] Token with wrong issuer/audience/resource is rejected.
- [x] Expired/invalid-signature/unknown-key token is rejected.
- [x] Missing execute scope returns authorization failure without spawning a process.
- [x] Token/secret values never appear in logs/errors/URLs.
- [x] Remote mode cannot downgrade to local no-auth.
- [x] Remote execution obeys the same non-root/filesystem/container boundary as local execution.

### 13.4 Release

- [x] Native artifacts are built directly by Cargo.
- [x] No Node/V8/libnode runtime dependency.
- [x] No `@yao-pkg/pkg`.
- [x] No legacy relay JS/TS runtime/build files.
- [x] Checksums/signatures/metadata match the reviewed commit.
- [x] Final CI status is green with required checks enforced.
- [x] Release job cannot publish artifacts if required quality/security gates fail.

**Phase 13 acceptance:** local and remote MCP flows, security abuse cases, OAuth authorization, strict privilege/filesystem policy, lint/audit/no-bypass gates, and release artifacts are all green.

## Definition of Done

Plan 028 may be marked `COMPLETED` only when:

- [ ] Relay is 100% Rust and standalone.
- [ ] MCP Streamable HTTP is the sole relay protocol.
- [ ] Local mode is loopback-only and non-root.
- [ ] No sudo/privilege-escalation path exists.
- [ ] Normal read/write coding operations are supported inside the configured boundary.
- [ ] Filesystem and runtime escape controls are authoritative and fail closed.
- [ ] Command/tool authorization is server-controlled and deny-by-default where authorization is required.
- [ ] SSRF/resource/process/output policies are enforced server-side.
- [ ] Remote mode is HTTPS + OAuth protected.
- [ ] OAuth uses standards-based resource-server validation and least-privilege scopes.
- [ ] OAuth cannot grant execution beyond the server-side capability/filesystem policy.
- [ ] No credentials/secrets/tokens are logged or exposed.
- [ ] No legacy Node relay or `@yao-pkg/pkg` remains.
- [ ] Clippy/compiler warnings are zero with no suppression/bypass.
- [ ] fmt, clippy, audit, dependency, CI-integrity, and security-search gates pass.
- [ ] No required CI command ignores failures or uses `continue-on-error`.
- [ ] Phase 15 is complete.
- [ ] Phase 16 is complete.
- [ ] Phase 17 is complete.
- [x] Phase 18 is complete.
- [x] Phase 13 final E2E/release gate is complete.

## Rollback

Keep the known-good release available until the final gate passes. If any security, OAuth, privilege, filesystem, container, lint, audit, CI-integrity, or E2E gate fails, keep the plan `IN FLIGHT`, do not weaken the gate, and fix the underlying issue before proceeding.
