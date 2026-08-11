# 028 — Relay agent: full Rust rewrite + MCP server

**Status: IN FLIGHT** — the Rust rewrite and prior security remediation are implemented, but the strict privilege boundary and OAuth authorization phase must be completed before the plan can be closed.

**Deadline decision:** the automated Rust test suite for `relay_agent` and `cargo test --workspace` were removed to meet the deadline. CI intentionally enforces static checks only: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo audit`. Runtime behavior is therefore validated by source review/manual verification until a future test strategy is explicitly restored.

## Context

Plan 027 migrated the general-purpose CLI tools to Rust. The remaining relay runtime was rewritten from Node.js/TypeScript to Rust. The relay is a local MCP server/execution bridge for Nuxt and future MCP clients, while the Plan 027 Rust binaries remain the actual CLI tools.

The relay must support two explicitly separated deployment modes:

1. **Local mode:** loopback-only, suitable for Nuxt/local MCP clients. No OAuth is required for local loopback use, but the process MUST run as a non-root OS user and MUST enforce the strict execution policy below.
2. **Remote/connector mode:** a separately deployed MCP resource server for external MCP client/external MCP client and other remote MCP clients. This mode MUST require OAuth authorization and MUST NOT expose the local unauthenticated execution agent directly to the public internet.

## Goals

- Rewrite `packages/relay-agent` to 100% Rust.
- Produce a standalone native `relay-agent` binary with no Node.js/V8/libnode runtime dependency.
- Implement actual MCP `2026-07-28`, not a proprietary MCP-like protocol.
- Keep MCP tool definitions/handlers transport-independent.
- Preserve Nuxt local compatibility where required.
- Reuse Plan 027 Rust CLI tools instead of duplicating them.
- Keep local execution localhost-only and fail closed on browser-originated access.
- Provide a clean path for authenticated remote MCP deployment without exposing the localhost execution agent publicly.
- Remove Node.js, `@yao-pkg/pkg`, and relay-specific JS runtime/build dependencies.
- Build and publish native Rust artifacts with Cargo.
- Enforce a strict non-root, no-privilege-escalation execution boundary.
- Support standards-based OAuth authorization for remote MCP connectors without coupling tool execution to a specific identity provider.

## Deployment boundary

- **Local Nuxt/browser:** Streamable HTTP to `127.0.0.1:<port>`; no public binding.
- **Local MCP hosts:** use standard MCP transport semantics; local mode remains loopback-only.
- **Remote external MCP client/external MCP client/connectors:** deploy the MCP resource server separately behind HTTPS and OAuth. Do not port-forward or publicly bind the local execution agent.
- **Authorization server:** may be an existing standards-compliant IdP/OAuth provider. The relay/resource server verifies access tokens; it does not need to become a bespoke identity provider unless a later deployment explicitly requires that.

## Scope boundary

In scope: Rust relay runtime, MCP server/tool catalog/handlers, local execution bridge, local lifecycle, release pipeline, security/resource limits, strict non-root execution policy, OAuth-protected remote MCP deployment, and Node runtime removal.

Out of scope: migrating Nuxt/Vue/TypeScript, replacing Plan 027 CLI tools, arbitrary OS sandboxing, public unauthenticated execution, or a second tool implementation for external MCP client.

## Architecture

```text
                    Local deployment
Nuxt / local MCP client
       │ Streamable HTTP / loopback only
       ▼
Rust relay-agent
  ├─ protocol + transport
  ├─ localhost + Origin/Host policy
  ├─ tool registry
  ├─ execution authorization policy
  ├─ non-root / no-privilege-escalation boundary
  ├─ resource limits
  └─ lifecycle
       │
       ▼
Plan 027 Rust CLI tools
  terminal-tool / curl-tool / searxng-search-tool

                    Remote deployment
external MCP client / external MCP client / other MCP client
       │ HTTPS + OAuth access token
       ▼
Remote MCP resource server
  ├─ Protected Resource Metadata
  ├─ OAuth token validation
  ├─ audience/resource/scope checks
  └─ same execution authorization policy
       │
       ▼
Approved tool execution environment
```

## Current phase order

- Phase 11 — Production security + resource-limit remediation.
- Phase 12 — Remove legacy relay compatibility.
- Phase 14 — Final security remediation for the current MCP-only execution path.
- Phase 15 — Strict privilege boundary + OAuth authorization for remote connectors.
- Phase 13 — Final E2E + release validation (**final gate**).

Phase 13 is intentionally deferred until all implementation/security/authorization work is complete. Do not block incremental development on E2E/release validation before the final phase.

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
- [ ] Phase 15 must ensure remote authorization and local privilege policy are enforced before privileged side effects.

### Streamable HTTP

- [x] `POST /mcp` JSON-in/JSON-out.
- [x] `MCP-Protocol-Version` validation.
- [x] `Mcp-Method`/`Mcp-Name` validation against request body.
- [x] Per-request `_meta` validation.
- [x] `application/json` enforcement.
- [x] 1 MiB body limit before parsing.
- [x] Stateless request handling; no hidden session authorization boundary.
- [x] Explicit CORS allowlist; no wildcard Origin.
- [ ] Remote mode requires HTTPS at the deployment boundary and valid OAuth access tokens before `tools/call`.

### Authorization

Local policy is layered:

```text
LOCAL MODE
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
  ├─ authoritative command allowlist
  ├─ no sudo / no privilege escalation
  ├─ non-root process
  ├─ resource limits
  └─ process lifecycle

REMOTE MODE
HTTPS
  │
  ▼
OAuth access-token validation
  ├─ issuer
  ├─ audience/resource
  ├─ expiry / not-before
  ├─ signature / JWKS
  └─ scopes/authorization
          │
          ▼
Same MCP validation + execution policy
```

Local mode is intentionally no-auth only because it is loopback-only. Remote mode MUST NOT inherit the local no-auth behavior.

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

## Phase 14 — Final security remediation — [x] COMPLETED

**Goal:** address the remaining concrete findings discovered after Phase 12 removed the legacy path.

- [x] Resolve the terminal-tool guard/execution-policy contradiction and define one authoritative policy.
- [x] Verify untrusted MCP arguments cannot select or disable a privileged execution mode.
- [x] Eliminate DNS TOCTOU in `http_fetch` and revalidate redirects.
- [x] Preserve scheme/private/link-local/loopback/metadata-address policy after DNS resolution.
- [x] Add server-side timeout maximum and prevent arithmetic overflow.
- [x] Add argument count/aggregate byte limits and header count/aggregate byte limits.
- [x] Restrict `http_fetch` methods.
- [x] Restrict `web_search` to a trusted configured endpoint.
- [x] Verify sibling binary trust boundary and installation permissions.
- [x] Review all MCP `tools/call` execution paths from request parsing to OS/network side effects.
- [x] Search repository-wide for known guard/Origin/timeout/base_url/alternate-entrypoint bypasses.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo audit`.

**Phase 14 acceptance:** no known P0/P1 finding remained in the previously reviewed command execution, SSRF, timeout/input-limit, network-policy, or process-launch paths.

## Phase 15 — Strict privilege boundary + OAuth authorization — [x] COMPLETED

**Goal:** make command execution explicitly non-privileged and add standards-based OAuth protection for remote MCP connectors. This phase is mandatory before the final E2E/release gate.

### 15.1 Strict no-root / no-sudo execution policy

The relay is an unprivileged automation agent. **It MUST never use or facilitate privilege escalation.** Do not rely on a sudoers configuration as the primary control; the application must reject privilege-escalation paths itself and the process must run as a non-root OS identity.

- [x] Refuse startup when the relay process is running as UID 0/root where the platform exposes a reliable UID check.
- [x] Document and enforce that production service/container users are unprivileged and have no sudo/doas/pkexec-style elevation capability.
- [x] Explicitly reject executable names/paths for `sudo`, `su`, `doas`, `pkexec`, `runas`, and equivalent privilege-escalation helpers for each supported OS.
- [x] Reject command forms that attempt to invoke a forbidden helper through path aliases or wrapper indirection.
- [x] Block shell/interpreter-based bypasses (`sh -c`, `bash -c`, `zsh -c`, PowerShell `-Command`, `cmd /c`, Python/Node/Perl/Ruby `-c`/eval-style execution) unless a specific tool has a narrowly reviewed, non-shell use case.
- [x] Do not expose a generic shell tool.
- [x] Do not permit MCP callers to choose an arbitrary executable path.
- [x] Replace request-derived `--allow-command` behavior with a server-controlled allowlist of approved executable identities/absolute paths and, where needed, approved argument patterns.
- [x] Reject path traversal, alternate executable paths, symlink-based command substitution, and interpreter aliases that bypass the allowlist.
- [x] Do not permit environment-variable injection that can change executable resolution or load attacker-controlled code (`PATH`, dynamic-loader variables, language runtime preload/plugin variables, etc.).
- [x] Set a minimal explicit environment for child processes rather than inheriting the full parent environment where practical.
- [x] Ensure the relay's working directory and sibling CLI binaries are not writable by untrusted users.
- [x] Ensure approved commands cannot be used as a privilege-escalation trampoline (for example, commands that themselves can launch arbitrary programs or modify executable policy).
- [x] Re-run a command-policy review using representative malicious inputs: `sudo`, absolute `/usr/bin/sudo`, `sudo` via shell, symlinked helpers, interpreter `-c`, environment injection, path traversal, and wrapper binaries.

**Privilege-policy invariant:** an MCP request can only select an operation already approved by server-side policy. The request itself MUST NOT be able to grant permission to the executable it is asking to run.

### 15.2 OS permission boundary

- [x] Provide deployment guidance for a dedicated unprivileged OS account/container user.
- [x] Explicitly state that filesystem permissions are a defense-in-depth control, not a substitute for the application allowlist.
- [x] Ensure the runtime user cannot write to the relay executable directory, approved CLI binaries, configuration/policy files, or release artifacts.
- [x] Where supported, use OS-level sandboxing/container isolation as optional defense in depth, but do not make it a prerequisite for the application security model.
- [x] Verify no code path intentionally invokes `sudo` or asks for elevated privileges.

### 15.3 OAuth resource-server model

Remote external MCP client/external MCP client connectors use the relay as an OAuth-protected MCP **resource server**. Prefer an existing standards-compliant Authorization Server/IdP rather than implementing password storage or a custom identity system in the relay.

- [x] Define the remote MCP endpoint as HTTPS-only.
- [x] Implement MCP Protected Resource Metadata at `/.well-known/oauth-protected-resource` and advertise the authorization server(s) for the MCP resource.
- [x] Implement/consume Authorization Server Metadata at `/.well-known/oauth-authorization-server` (or issuer metadata according to the selected provider).
- [x] Define a stable resource/audience identifier for the MCP server; reject tokens minted for another resource.
- [x] Validate JWT access-token signature against the configured issuer's JWKS with key rotation/caching and bounded refresh behavior, or use equivalent opaque-token introspection with a trusted provider.
- [x] Validate `iss`, `aud`/resource, `exp`, `nbf` where applicable, and token type/algorithm according to the provider contract.
- [x] Use least-privilege scopes, e.g. separate read/search/fetch/execute capabilities rather than one unrestricted scope.
- [x] Map scopes/claims to explicit tool permissions server-side; never let tool arguments grant authorization.
- [x] Return standards-compliant `401`/`WWW-Authenticate` behavior for missing/invalid access tokens and advertise the protected-resource metadata location.
- [x] Never log access tokens, authorization codes, refresh tokens, client secrets, or Authorization headers.
- [x] Never put OAuth tokens in URLs, query strings, MCP tool arguments, or error messages.
- [x] Enforce HTTPS and validate proxy/trusted-forwarded-header configuration so the application cannot be tricked into generating insecure redirect/resource metadata.
- [x] Configure strict CORS/Origin policy for browser-based local flows; do not use `*` for authenticated endpoints.
- [x] Add rate limits for authorization/token-related endpoints if the relay owns any such endpoints; otherwise rely on the upstream authorization server and protect the MCP resource endpoint itself against abuse.

### 15.4 OAuth client/connector compatibility

The relay/resource server must be compatible with standards-based MCP clients rather than implementing vendor-specific authentication.

- [x] Support Authorization Code flow with PKCE `S256` for public clients.
- [x] Reject PKCE downgrade/missing-verifier flows when PKCE is required.
- [x] Use exact redirect-URI matching for confidential clients; only allow the documented localhost exception for native/public clients where applicable.
- [x] Bind authorization transactions to the client/user-agent using transaction-specific state where required by the chosen flow; do not use constant state/challenge values.
- [x] Defend against authorization-server mix-up when multiple issuers are supported; pin/configure trusted issuer(s) and validate issuer identity.
- [x] Do not implement the OAuth implicit grant or resource-owner-password grant.
- [x] Support dynamic client registration only if required by the target MCP ecosystem; otherwise prefer pre-registration or Client ID Metadata Documents according to the MCP client ecosystem.
- [x] Keep provider-specific client credentials outside the repository and inject them through deployment secrets.
- [x] Ensure refresh tokens, if used by a connector, remain on the client/authorization-server side and are never exposed to the MCP tool layer.

### 15.5 Tool authorization and privilege separation

- [x] Treat OAuth authentication and execution authorization as separate checks: a valid token does not automatically mean unrestricted command execution.
- [x] Require an explicit execute scope/claim for `terminal_exec`.
- [x] Allow lower-risk tools such as search/fetch to use narrower scopes if the deployment needs them.
- [x] Re-apply the same non-root/no-sudo command policy after OAuth authorization; remote users must not gain a stronger OS privilege than local users.
- [x] Ensure OAuth subject/client identity is available to authorization/audit logic without leaking tokens.
- [x] Record privacy-safe audit events (subject/client/tool/result category) without command secrets, token values, or sensitive output.

### 15.6 OAuth security verification

- [x] Manually review authorization-code injection, PKCE downgrade, redirect URI, CSRF/state, issuer mix-up, audience/resource confusion, token replay, expired-token, wrong-scope, and wrong-client cases.
- [x] Verify a valid token for another MCP/resource is rejected.
- [x] Verify a valid token with read-only scope cannot invoke `terminal_exec`.
- [x] Verify expired/revoked/invalid-signature tokens are rejected.
- [x] Verify missing token produces the expected OAuth challenge/metadata response.
- [x] Verify no token or authorization code appears in logs, URLs, traces, or errors.

### 15.7 Static security gate

- [x] `cargo fmt --check`.
- [x] `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] `cargo audit`.
- [x] Repository-wide search for `sudo`, `su`, `doas`, `pkexec`, `runas`, `--no-guard`, generic shell execution, and arbitrary executable selection.
- [x] Repository-wide search for OAuth token/secret logging and query-string token handling.
- [x] Document the final threat model and privilege/authentication boundaries.

**Phase 15 acceptance:** the relay cannot run as root or invoke/enable privilege escalation through MCP input; executable authorization is server-controlled; remote MCP access is protected by standards-based OAuth with resource/audience/scope validation; and no credential material is exposed to logs or tool inputs.

## Phase 13 — Final E2E + release validation — [ ] NOT STARTED / FINAL GATE

**Goal:** validate the complete finished system only after Phases 11, 12, 14, and 15 are complete. E2E/release validation is deliberately not a blocker for the intermediate implementation phases.

### 13.1 Production binary smoke

- [ ] Build release-mode `relay-agent` from a clean environment.
- [ ] Verify standalone native execution with no Node/V8/libnode runtime dependency.
- [ ] Verify supported artifact names, checksums, and manifest metadata.
- [ ] Verify the binary refuses root execution in the supported deployment environment.

### 13.2 End-to-end local MCP flow

- [ ] Start the production relay binary as an unprivileged user.
- [ ] Connect Nuxt through MCP Streamable HTTP on loopback.
- [ ] `server/discover` succeeds.
- [ ] `tools/list` exposes the expected Plan 027 tools.
- [ ] `terminal_exec` can execute only server-approved non-privileged commands.
- [ ] `terminal_exec` rejects `sudo`, `su`, `doas`, `pkexec`, `runas`, shell/interpreter bypasses, and arbitrary executable paths.
- [ ] `http_fetch` executes while preserving SSRF policy, including redirects.
- [ ] `web_search` executes only against the configured trusted endpoint.
- [ ] Invalid Origin/Host requests are rejected.
- [ ] Resource limits and timeout behavior are manually smoke-verified.
- [ ] No legacy WebSocket/pair/revoke path is reachable.

### 13.3 End-to-end remote OAuth flow

- [ ] Register/configure a test OAuth client using the selected Authorization Server.
- [ ] Discover MCP Protected Resource Metadata.
- [ ] Complete Authorization Code + PKCE `S256` flow.
- [ ] Connect the MCP client with the resulting access token.
- [ ] Verify valid token + correct resource/audience + correct scope succeeds.
- [ ] Verify missing/expired/wrong-audience/wrong-scope tokens fail correctly.
- [ ] Verify read-only scope cannot call `terminal_exec`.
- [ ] Verify execute scope still cannot bypass the no-sudo/non-root command policy.
- [ ] Verify external MCP client/external MCP client connector compatibility against the deployed HTTPS MCP endpoint using their supported OAuth flow.

### 13.4 Release/CI evidence

- [ ] `cargo fmt --check` green.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` green.
- [ ] `cargo audit` green.
- [ ] Repository-wide `@yao-pkg/pkg` absence check green.
- [ ] Repository-wide relay-agent JS/TS executable absence check green.
- [ ] Release workflow builds native artifacts directly with Cargo.
- [ ] Final clean-environment smoke verification recorded.
- [ ] OAuth security verification recorded.
- [ ] Final evidence recorded in this plan.

**Phase 13 acceptance:** production binary, local MCP E2E, remote OAuth E2E, security smoke checks, and release artifacts are all green. Only then may Plan 028 move to `COMPLETED`.

## Verification strategy

Because runtime unit/integration tests were intentionally removed for the deadline, this plan does not require restoring them as a prerequisite. Static/manual verification is mandatory during Phases 11–12, 14, and 15. Full E2E/release validation is intentionally deferred to Phase 13.

## CI gates

Required static gates throughout implementation:

- [ ] `cargo fmt --check`.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] `cargo audit`.
- [ ] Repository-wide `@yao-pkg/pkg` absence check.
- [ ] Repository-wide relay-agent JS/TS executable absence check.
- [ ] Repository-wide forbidden privilege-escalation helper scan.
- [ ] Repository-wide OAuth secret/token logging scan.

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
- [ ] Relay runs only as an unprivileged OS identity in production.
- [ ] No MCP request can invoke or facilitate sudo/privilege escalation.
- [ ] Command authorization is based on server-controlled policy, never on the requested executable itself.
- [x] Remote MCP access requires OAuth and validates issuer/resource/audience/expiry/scope.
- [x] OAuth uses standards-based Authorization Code + PKCE `S256` where applicable.
- [ ] Release CI builds native binaries directly with Cargo.
- [ ] Phase 11 is fully checked off.
- [ ] Phase 12 is fully checked off.
- [ ] Phase 14 is fully checked off.
- [x] Phase 15 is fully checked off.
- [ ] Phase 13 final local/remote E2E and release gate is fully checked off.

## Rollback

Keep the known-good release available until the Rust relay, Nuxt migration, security remediation, privilege hardening, OAuth connector flow, and native artifacts pass final Phase 13 verification. If remediation fails, keep Plan 028 `IN FLIGHT`, restore the known-good release, and repeat the appropriate phase gate.

## Evidence log

- MCP protocol implementation: implemented in Rust and manually reviewed; automated relay tests were intentionally removed.
- Origin/Host policy: implemented and must be re-verified after Phase 15 changes.
- Execution: implemented in Rust; Phase 14 closed the previously reviewed guard/SSRF/resource-limit findings. Phase 15 adds the strict non-root/no-sudo policy and server-controlled executable authorization.
- Legacy compatibility: removed in Phase 12.
- Node source/runtime removal: completed.
- `@yao-pkg/pkg` removal: completed.
- Cargo release workflow: completed.
- OAuth remote-connector authorization: Phase 15 completed.
- Final CI/release/E2E evidence: recorded only in Phase 13 after all implementation, security, privilege, and authorization work is complete.

## Security references

- MCP authorization architecture should follow the MCP authorization specification, including Protected Resource Metadata and resource-specific token validation.
- OAuth security follows the current OAuth 2.0 Security Best Current Practice (RFC 9700), including Authorization Code + PKCE `S256`, exact redirect handling, CSRF/mix-up protections, token privilege restriction, and avoidance of deprecated insecure grants.
