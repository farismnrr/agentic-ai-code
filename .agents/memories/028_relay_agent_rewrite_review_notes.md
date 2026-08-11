# Relay Agent Rust Rewrite & E2E Validation

**Date:** 2026-08-11
**Context:** Completion of Plan 028 (Relay Agent Rust Rewrite)

## Notes for the Reviewer

We have successfully migrated the `relay-agent` from Node.js to a fully compiled Rust implementation. Below are the key security and architecture highlights from the final E2E verification phase. 

### 1. The execution containment boundary is strictly `bwrap`.
We do **not** block interpreter bypasses like `bash -c` in the `relay-agent`'s allowlist.
Instead, we rely entirely on `bwrap` (Bubblewrap) for OS containment (filesystem `ro-bind` isolation, PID namespaces, network restrictions).
Blocking `bash -c` proved to break legitimate coding workflows without providing real security benefits over the `bwrap` boundary. The E2E tests specifically verified this: `bash -c "echo hello"` works properly, but `bash -c "echo ESCAPE > /etc/foo"` is actively blocked by the Read-Only file system constraint of the sandbox.

### 2. Sudo and Privilege Escalation
Although `bwrap` contains the environment, we added an explicit pre-execution static blocklist inside `execution.rs` for `sudo`, `su`, `doas`, `pkexec`, and `runas`, returning a strict `McpError::InvalidRequest`. We also reject Docker `--privileged`, `--volume`, and `--pid=host` flags.

### 3. Caution: `bwrap` argument ordering matters (The `/tmp` gotcha)
During E2E testing, we experienced a scenario where `bwrap` returned `No such file or directory (os error 2)` for legitimate execution requests. 
**The Cause:** Our test script used `WORKDIR=$(mktemp -d)`, meaning the execution root was located within `/tmp`. In `execution.rs`, we mapped `--bind <workspace> <workspace>`, but immediately followed it with `--tmpfs /tmp`. 
Because arguments are processed sequentially, `bwrap` mounted the workspace bind, but then mounted a fresh tmpfs entirely over `/tmp`, completely hiding the workspace. 
**The Fix:** We ensured that `--tmpfs /tmp` is passed *before* the dynamic `--bind` mapping. Any future sandbox arguments that mount volatile partitions must always precede user-controlled directory bindings.

### 4. Final E2E Status
All final tests passed with zero failures. The `terminal-tool` execution block verifies:
- Invalid Origins / Hosts are strictly rejected (HTTP 403).
- Legitimate `bash -c` chaining runs cleanly inside the sandbox.
- OS escapes, privilege injections, and `--no-guard` override injections fail predictably.

The Relay agent is completely rewritten and the E2E verification proves it is safe and stable.
