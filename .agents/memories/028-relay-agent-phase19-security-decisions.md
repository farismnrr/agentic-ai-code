# Phase 19 Security Architecture & Decisions (Plan 028: Relay Agent Rust Rewrite)

## Context
During the final security reviews (Phase 19) of the `relay-agent` rewrite to a standalone Rust binary, several architectural security holes were identified that allowed bypasses of the intended sandboxing and authorization models. This document summarizes the critical decisions made to plug those holes and ensure zero-bypass containment.

## 1. Filesystem Containment via OS-Level Sandbox (Bubblewrap)
**The Problem:** The MCP server allows execution of development commands (`rm`, `cp`, `bash`, `npm`, etc.) via `terminal_exec`. While the command names were validated against an allowlist, the arguments were not. This meant a user could execute `rm -rf /` and easily bypass the intended `execution_root` because naive path-string validation inside Rust is highly vulnerable to argument injection and TOCTOU races.

**The Decision:** We implemented an **OS-Level Sandbox using `bwrap` (Bubblewrap)**. 
Instead of trying to parse arguments in Rust, `terminal-tool` execution is wrapped inside a `bwrap` namespace. The sandbox explicitly binds the `execution_root` to itself and mounts essential binaries/libraries (`/usr`, `/bin`, `/lib`, `/etc`) as read-only. 
**Result:** Any command executed by the relay is structurally confined by the Linux Kernel to the designated root. Path traversal (`../`) or absolute path escapes (`/etc/shadow`) will fail at the syscall level.

## 2. Asymmetric OAuth Validation (JWKS + PKCE)
**The Problem:** The initial OAuth implementation relied on a symmetric shared secret (`HS256`) to validate JWTs. This violated the requirement for a robust production gate using standard Authorization Server models.

**The Decision:** We refactored `transport.rs` to enforce **Asymmetric JWKS Validation**. 
In `REMOTE` mode, the relay requires an `oauth_issuer` configuration. It fetches the `.well-known/jwks.json` from the Identity Provider, caches the public keys, and validates incoming JWT signatures (`RS256` / `ES256`) against the corresponding `kid`. Additionally, the implementation enforces PKCE S256 flows for public clients, removing the risk of secret leakage.

## 3. Strict Docker Containment
**The Problem:** Docker commands were allowed but could be abused to mount the host root (`-v /:/host`) or escalate privileges (`--privileged`).

**The Decision:** We tightened the execution policy to explicitly forbid docker commands that include `--privileged`, `--cap-add`, host namespace flags, and arbitrary bind mounts.

## 4. Zero-Bypass CI & Linting
**The Problem:** Minor linter warnings (e.g., collapsible `if` blocks) were previously being ignored or could potentially introduce CI bypasses.

**The Decision:** All source files were aggressively refactored until they passed `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` and `cargo audit` with absolutely zero warnings and zero `#[allow(clippy::...)]` bypasses.

## Conclusion
The `relay-agent` now operates under a strict, layered security model where:
1. It runs without root privileges (UID > 0).
2. Incoming requests are explicitly authenticated and authorized via JWKS.
3. Process executions are contained at the kernel level using namespaces (`bwrap`), ensuring the agent is safely confined to its workspace.
