# Plan 028 — Phase 19: Final Security Boundary, CI Integrity & Release Verification

**Status: NOT STARTED**

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

- [ ] Define explicit allowed filesystem roots independently from `cwd`; `--dir` is a working-directory default, not a sandbox by itself.
- [ ] Resolve relative paths against the configured allowed root.
- [ ] Reject absolute paths outside all allowed roots.
- [ ] Reject `..` traversal after resolution.
- [ ] Use component-aware containment; never use naive string-prefix checks.
- [ ] Handle non-existent destination paths safely.
- [ ] Validate both source and destination for rename/move/copy operations.
- [ ] Prevent symlink escapes, including symlink replacement/races between validation and use.
- [ ] Review hardlink attacks and prevent modification of protected/out-of-root files through links.
- [ ] Review TOCTOU races and use race-resistant OS primitives where practical.
- [ ] Keep recursive delete/copy operations inside the allowed roots.
- [ ] Keep archive extraction inside the allowed roots.
- [ ] Explicitly review `/proc`, `/sys`, `/dev`, `/run`, mounted filesystems, and other special filesystem paths.
- [ ] Ensure child processes cannot bypass the intended filesystem policy simply because they can issue arbitrary OS file operations.
- [ ] Document behavior when an allowed root is itself a symlink, mount point, or unavailable.

## 19.2 Privilege boundary

- [ ] Relay and child processes run as an explicitly non-root UID/GID.
- [ ] Refuse startup as UID 0/root where a reliable platform check exists, unless an explicitly reviewed privilege-drop design is used before accepting requests.
- [ ] No `sudo`, `su`, `doas`, `pkexec`, `runas`, or equivalent elevation helper may be invoked by relay-controlled execution.
- [ ] Reject privilege-escalation helpers through aliases, wrappers, symlinks, and equivalent forms where applicable.
- [ ] No code path intentionally requests elevation or adds capabilities.
- [ ] Prevent setuid/setgid privilege acquisition.
- [ ] Prevent unintended Linux capability inheritance/acquisition.
- [ ] Verify effective UID/GID and relevant capabilities at runtime rather than relying only on executable-name checks.
- [ ] Treat privileged host groups and host-management daemons as privilege boundaries.
- [ ] Document that ordinary development commands remain allowed; command names alone are not the primary security boundary.

## 19.3 Docker/container boundary

Docker is intentionally allowed for coding/build workflows, but it must not silently become a host-root escape.

- [ ] Explicitly document whether the configured Docker daemon is trusted and what that means for the security boundary.
- [ ] If filesystem confinement is mandatory, prevent arbitrary host bind mounts outside allowed roots.
- [ ] Block or explicitly policy-gate `--privileged`.
- [ ] Policy-gate arbitrary `--cap-add`, device mappings, and security-opt changes that weaken isolation.
- [ ] Policy-gate host PID/IPC/network namespace modes when they violate the intended boundary.
- [ ] Prevent unauthorized access to the host Docker socket or equivalent runtime sockets.
- [ ] Review `docker exec` against privileged/root containers.
- [ ] Review alternate container runtimes/daemon APIs if supported.
- [ ] Document that unrestricted control of a host Docker daemon can be effectively equivalent to host-root access.

## 19.4 Process and environment containment

- [ ] Every process-spawn path uses the same security policy; no secondary path bypasses it.
- [ ] Use a trusted deterministic `PATH`; never let writable workspace content shadow security-sensitive executables unintentionally.
- [ ] Remove loader/injection variables such as `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_*`, and language-specific execution hooks where applicable.
- [ ] Do not inherit unnecessary host secrets, credentials, OAuth tokens, or agent secrets into child processes.
- [ ] Child cwd is always inside the allowed boundary.
- [ ] Prevent uncontrolled daemon/background-process escape from timeout and cleanup policy.
- [ ] Terminate the complete process tree, not only the direct parent.
- [ ] Bound process/fork behavior where supported.
- [ ] Bound memory, CPU, file descriptors, output size, and execution duration.
- [ ] Resource limits are server-controlled and cannot be disabled by request arguments.

## 19.5 MCP authorization

- [ ] Remote deployments require authentication; local no-auth mode remains explicitly loopback-only.
- [ ] A valid bearer token is not sufficient authorization for every tool.
- [ ] Implement a closed server-side `scope -> tool/capability` map.
- [ ] Default deny for missing/unknown scopes.
- [ ] Terminal execution requires an explicit execute scope.
- [ ] Other privileged tools use their own least-privilege scopes.
- [ ] Authorization occurs before any filesystem/process/network side effect.
- [ ] Return `401` for missing/invalid auth and `403` for insufficient scope where applicable.
- [ ] Tool arguments cannot grant capabilities, disable guards, change policy roots, or override authorization.
- [ ] Every MCP execution entrypoint reaches the same authorization/execution policy.

## 19.6 OAuth production gate

- [ ] Do not use a custom shared-secret JWT scheme as the production trust model for third-party connectors.
- [ ] Use a trusted Authorization Server/IdP.
- [ ] Validate JWTs through trusted issuer JWKS with rotation, or trusted opaque-token introspection.
- [ ] Validate issuer, resource/audience, token type, expiry, not-before, signature, and allowed algorithms.
- [ ] Reject `none`, algorithm confusion, malformed tokens, unknown keys, and stale/invalid signatures.
- [ ] Protected Resource Metadata is complete and consistent with the configured resource.
- [ ] Authorization Server Metadata/discovery is trusted and pinned to configured providers.
- [ ] Authorization Code + PKCE S256 is required for public clients.
- [ ] Reject PKCE downgrade.
- [ ] Exact redirect URI validation; no prefix/wildcard matching.
- [ ] State/CSRF protection and authorization-server mix-up protection.
- [ ] Token/secret values never enter logs, URLs, query strings, tool arguments, command lines, child environments, or error messages.
- [ ] Least-privilege scopes are enforced server-side.

## 19.7 Zero-bypass CI and Rust quality

Every warning, lint, audit finding, policy violation, and security check failure is a blocker. Fix the source; never suppress the problem just to make CI green.

- [ ] `cargo fmt --all -- --check`.
- [ ] `cargo check --workspace --all-targets --all-features --locked` with warnings denied.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- [ ] `cargo audit` with zero unreviewed vulnerabilities.
- [ ] Complete workspace/all-target/all-feature coverage; no root-package-only checks.
- [ ] No `#[allow(...)]`/`#[allow(clippy::...)]` used to hide required diagnostics.
- [ ] No `#[allow(clippy::all)]`, `#![allow(warnings)]`, or equivalent blanket suppression.
- [ ] No `#[expect(...)]` used to suppress unresolved warnings.
- [ ] No Cargo config, `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, environment override, or script can downgrade/remove `-D warnings`.
- [ ] No `continue-on-error`, `|| true`, `; true`, swallowed exit code, unconditional fallback, or equivalent failure masking.
- [ ] No path/branch condition can skip relay security/quality checks for relevant changes.
- [ ] Add a deterministic static policy check for CI/lint-bypass patterns; the policy check itself must fail closed.
- [ ] Dependency/security checks cannot be skipped for release commits.

## 19.8 Release gate

- [ ] Release jobs depend on successful quality/security validation or execute equivalent mandatory checks before publishing.
- [ ] No release path can publish after failed/skipped lint, audit, or security gates.
- [ ] Supported release target matrix is explicit and frozen.
- [ ] Artifacts are built from the reviewed commit.
- [ ] Artifacts contain only the intended Rust relay binaries; no Node/pkg relay fallback.
- [ ] Checksums/signatures follow repository release policy.

## 19.9 Final manual source review

Because Rust unit/integration tests are intentionally not required for this deadline, manually review every security-sensitive execution path.

- [ ] MCP request -> auth -> scope authorization -> schema validation -> tool dispatch -> execution policy -> process spawn has exactly one enforced security path.
- [ ] No alternate HTTP/MCP/tool endpoint can spawn commands without the same policy.
- [ ] No legacy endpoint can bypass MCP authorization.
- [ ] No request-controlled flag can disable filesystem, privilege, timeout, resource, or authorization checks.
- [ ] Normal coding workflow still works: inspect, create/edit/delete/move files, git, npm, cargo, shell/interpreters, builds, Docker.
- [ ] Escape attempts fail: `../`, absolute out-of-root paths, symlink escape, hardlink abuse, mount escape, privileged Docker, privilege escalation, and process-tree escape.
- [ ] No sensitive credentials appear in logs, responses, command lines, environments, or release artifacts.

## 19.10 Final E2E completion gate

Only after every Phase 19 item passes:

- [ ] MCP client connects successfully.
- [ ] Local and remote authentication/authorization behave as designed.
- [ ] Real coding workflow succeeds through MCP: inspect repository, edit files, install/build dependencies, run commands, and produce a successful build/run.
- [ ] Security boundaries remain enforced throughout that workflow.
- [ ] CI is green with zero warnings/errors and no bypass/suppression.
- [ ] Release workflow is green and publishes only reviewed Rust artifacts.
- [ ] Plan status is changed to `COMPLETED` only after independent verification of all gates.
