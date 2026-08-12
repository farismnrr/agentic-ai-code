# Plan 028 — Phase 19: Final Security Boundary, CI Integrity & Release Verification

**Status: COMPLETED**

> Status reconciled with the recorded checklist, final E2E gate, Plan 028 closeout, and durable security memories. The previous `NOT STARTED` header was stale metadata left behind after implementation.

## Objective

Finish the final production-security pass for the MCP coding agent. The relay is intentionally a **read/write coding agent**, not a read-only sandbox: normal development commands such as `rm`, `cp`, `mv`, `git`, `npm`, `cargo`, `docker`, shells, interpreters, and build tools are allowed when they remain inside the configured security boundary.

The security boundary is **not** an arbitrary-command denylist. It is:

- non-root execution;
- no privilege escalation;
- authoritative filesystem containment;
- process/environment containment;
- resource limits;
- server-side MCP/OAuth authorization;
- strict CI/release enforcement with no bypasses.

## 19.1 Filesystem containment — authoritative boundary

- [x] Define explicit allowed filesystem roots independently from `cwd`; `--dir` is a working-directory default, not a sandbox by itself.
- [x] Resolve relative paths against the configured allowed root.
- [x] Reject absolute paths outside all allowed roots.
- [x] Reject `..` traversal after resolution.
- [x] Use component-aware containment; never use naive string-prefix checks.
- [x] Handle non-existent destination paths safely.
- [x] Validate both source and destination for rename/move/copy operations.
- [x] Prevent symlink escapes, including symlink replacement/races between validation and use.
- [x] Review hardlink attacks and prevent modification of protected/out-of-root files through links.
- [x] Review TOCTOU races and use race-resistant OS primitives where practical.
- [x] Keep recursive delete/copy operations inside the allowed roots.
- [x] Keep archive extraction inside the allowed roots.
- [x] Explicitly review `/proc`, `/sys`, `/dev`, `/run`, mounted filesystems, and other special filesystem paths.
- [x] Ensure child processes cannot bypass the intended filesystem policy simply because they can issue arbitrary OS file operations.
- [x] Document behavior when an allowed root is itself a symlink, mount point, or unavailable.

## 19.2 Privilege boundary

- [x] Relay and child processes run as an explicitly non-root UID/GID.
- [x] Refuse startup as UID 0/root where a reliable platform check exists, unless an explicitly reviewed privilege-drop design is used before accepting requests.
- [x] No `sudo`, `su`, `doas`, `pkexec`, `runas`, or equivalent elevation helper may be invoked by relay-controlled execution.
- [x] Reject privilege-escalation helpers through aliases, wrappers, symlinks, and equivalent forms where applicable.
- [x] No code path intentionally requests elevation or adds capabilities.
- [x] Prevent setuid/setgid privilege acquisition.
- [x] Prevent unintended Linux capability inheritance/acquisition.
- [x] Verify effective UID/GID and relevant capabilities at runtime rather than relying only on executable-name checks.
- [x] Treat privileged host groups and host-management daemons as privilege boundaries.
- [x] Document that ordinary development commands remain allowed; command names alone are not the primary security boundary.

## 19.3 Docker/container boundary

Docker is intentionally allowed for coding/build workflows, but it must not silently become a host-root escape.

- [x] Explicitly document whether the configured Docker daemon is trusted and what that means for the security boundary.
- [x] If filesystem confinement is mandatory, prevent arbitrary host bind mounts outside allowed roots.
- [x] Block or explicitly policy-gate `--privileged`.
- [x] Policy-gate arbitrary `--cap-add`, device mappings, and security-opt changes that weaken isolation.
- [x] Policy-gate host PID/IPC/network namespace modes when they violate the intended boundary.
- [x] Prevent unauthorized access to the host Docker socket or equivalent runtime sockets.
- [x] Review `docker exec` against privileged/root containers.
- [x] Review alternate container runtimes/daemon APIs if supported.
- [x] Document that unrestricted control of a host Docker daemon can be effectively equivalent to host-root access.

## 19.4 Process and environment containment

- [x] Every process-spawn path uses the same security policy; no secondary path bypasses it.
- [x] Use a trusted deterministic `PATH`; never let writable workspace content shadow security-sensitive executables unintentionally.
- [x] Remove loader/injection variables such as `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_*`, and language-specific execution hooks where applicable.
- [x] Do not inherit unnecessary host secrets, credentials, OAuth tokens, or agent secrets into child processes.
- [x] Child cwd is always inside the allowed boundary.
- [x] Prevent uncontrolled daemon/background-process escape from timeout and cleanup policy.
- [x] Terminate the complete process tree, not only the direct parent.
- [x] Bound process/fork behavior where supported.
- [x] Bound memory, CPU, file descriptors, output size, and execution duration.
- [x] Resource limits are server-controlled and cannot be disabled by request arguments.

## 19.5 MCP authorization

- [x] Remote deployments require authentication; local no-auth mode remains explicitly loopback-only.
- [x] A valid bearer token is not sufficient authorization for every tool.
- [x] Implement a closed server-side `scope -> tool/capability` map.
- [x] Default deny for missing/unknown scopes.
- [x] Terminal execution requires an explicit execute scope.
- [x] Other privileged tools use their own least-privilege scopes.
- [x] Authorization occurs before any filesystem/process/network side effect.
- [x] Return `401` for missing/invalid auth and `403` for insufficient scope where applicable.
- [x] Tool arguments cannot grant capabilities, disable guards, change policy roots, or override authorization.
- [x] Every MCP execution entrypoint reaches the same authorization/execution policy.

## 19.6 OAuth production gate

- [x] Do not use a custom shared-secret JWT scheme as the production trust model for third-party connectors.
- [x] Use a trusted Authorization Server/IdP.
- [x] Validate JWTs through trusted issuer JWKS with rotation, or trusted opaque-token introspection.
- [x] Validate issuer, resource/audience, token type, expiry, not-before, signature, and allowed algorithms.
- [x] Reject `none`, algorithm confusion, malformed tokens, unknown keys, and stale/invalid signatures.
- [x] Protected Resource Metadata is complete and consistent with the configured resource.
- [x] Authorization Server Metadata/discovery is trusted and pinned to configured providers.
- [x] Authorization Code + PKCE S256 is required for public clients.
- [x] Reject PKCE downgrade.
- [x] Exact redirect URI validation; no prefix/wildcard matching.
- [x] State/CSRF protection and authorization-server mix-up protection.
- [x] Token/secret values never enter logs, URLs, query strings, tool arguments, command lines, child environments, or error messages.
- [x] Least-privilege scopes are enforced server-side.

## 19.7 Zero-bypass CI and Rust quality

Every warning, lint, audit finding, policy violation, and security check failure is a blocker. Fix the source; never suppress the problem just to make CI green.

- [x] `cargo fmt --all -- --check`.
- [x] `cargo check --workspace --all-targets --all-features --locked` with warnings denied.
- [x] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- [x] `cargo audit` with zero unreviewed vulnerabilities.
- [x] Complete workspace/all-target/all-feature coverage; no root-package-only checks.
- [x] No `#[allow(...)]`/`#[allow(clippy::...)]` used to hide required diagnostics.
- [x] No `#[allow(clippy::all)]`, `#![allow(warnings)]`, or equivalent blanket suppression.
- [x] No `#[expect(...)]` used to suppress unresolved warnings.
- [x] No Cargo config, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, environment override, or script can downgrade/remove `-D warnings`.
- [x] No `continue-on-error`, `|| true`, `; true`, swallowed exit code, unconditional fallback, or equivalent failure masking.
- [x] No path/branch condition can skip relay security/quality checks for relevant changes.
- [x] Add a deterministic static policy check for CI/lint-bypass patterns; the policy check itself must fail closed.
- [x] Dependency/security checks cannot be skipped for release commits.

## 19.8 Release gate

- [x] Release jobs depend on successful quality/security validation or execute equivalent mandatory checks before publishing.
- [x] No release path can publish after failed/skipped lint, audit, or security gates.
- [x] Supported release target matrix is explicit and frozen.
- [x] Artifacts are built from the reviewed commit.
- [x] Artifacts contain only the intended Rust relay binaries; no Node/pkg relay fallback.
- [x] Checksums/signatures follow repository release policy.

## 19.9 Final manual source review

Because Rust unit/integration tests are intentionally not required for this deadline, manually review every security-sensitive execution path.

- [x] MCP request -> auth -> scope authorization -> schema validation -> tool dispatch -> execution policy -> process spawn has exactly one enforced security path.
- [x] No alternate HTTP/MCP/tool endpoint can spawn commands without the same policy.
- [x] No legacy endpoint can bypass MCP authorization.
- [x] No request-controlled flag can disable filesystem, privilege, timeout, resource, or authorization checks.
- [x] Normal coding workflow still works: inspect, create/edit/delete/move files, git, npm, cargo, shell/interpreters, builds, Docker.
- [x] Escape attempts fail: `../`, absolute out-of-root paths, symlink escape, hardlink abuse, mount escape, privileged Docker, privilege escalation, and process-tree escape.
- [x] No sensitive credentials appear in logs, responses, command lines, environments, or release artifacts.

## 19.10 Final E2E completion gate

Only after every Phase 19 item passes:

- [x] MCP client connects successfully.
- [x] Local and remote authentication/authorization behave as designed.
- [x] Real coding workflow succeeds through MCP: inspect repository, edit files, install/build dependencies, run commands, and produce a successful build/run.
- [x] Security boundaries remain enforced throughout that workflow.
- [x] CI is green with zero warnings/errors and no bypass/suppression.
- [x] Release workflow is green and publishes only reviewed Rust artifacts.
- [x] Plan status is changed to `COMPLETED` only after independent verification of all gates.
