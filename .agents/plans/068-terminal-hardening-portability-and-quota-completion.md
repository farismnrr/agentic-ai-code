# Plan 068 — Terminal Hardening Portability and Quota Completion

**Status:** READY — follow-up to the scoped Linux/modern closure of Plan 067
**Predecessor:** [Plan 067](067-terminal-first-multi-os-hardening.md)
**Baseline:** `plan/067-terminal-first-hardening` at the Plan 067 closure commit

## Objective

Complete the terminal hardening rows that cannot be truthfully closed by the
Linux/Bubblewrap and modern MCP slice: protocol compatibility, bounded resource
enforcement, profile probing, lost-response persistence, and reviewed native
macOS/Windows containment. No platform or protocol is considered supported
until it passes the same positive and negative evidence matrix.

## Non-negotiable boundaries

- Never replace Bubblewrap with a raw host shell or weaken credential,
  privilege, SSH, agent-socket, Docker, Tailscale, or D-Bus boundaries.
- A platform without an equivalent reviewed primitive fails closed.
- A protocol version is supported only when its complete handshake, task
  lifecycle, authorization, and error semantics are implemented and tested;
  accepting an initialize version without its follow-up lifecycle is not
  compatibility.
- Resource limits must be enforced before spawn, observable in bounded task
  state, and released on completion, timeout, cancellation, panic, and relay
  shutdown.

## Workstreams

### 068-A — MCP protocol compatibility and conformance

- Decide whether stateful `2025-11-25` remains a supported inbound protocol or
  is removed from the advertised contract; do not advertise a half-supported
  version.
- If retained, implement stateful session handling, legacy `tasks/get`,
  `tasks/result`, `tasks/cancel`, initialized notifications, and the exact
  underlying-result/error semantics.
- Add a modern wire harness for `2026-07-28` and run the official MCP
  conformance scenarios that apply to the negotiated version. Keep Inspector
  CLI smoke checks as a developer tool, never as the only security proof.
- Test malformed headers/meta, notification handling, duplicate IDs, unknown
  methods, invalid task IDs, mismatched task routing headers, and auth context
  binding without leaking existence or private details.

### 068-B — Linux resource controls and lifecycle stress

- Add operator-bounded CPU, memory, process-count, file-descriptor, output,
  per-owner/global concurrency, poll-rate, total-retention, and task-TTL
  controls using cgroups/rlimits or an explicitly reviewed equivalent.
- Expose only bounded, non-sensitive limit/status metadata in task results and
  diagnostics. Fail closed when a required limit cannot be installed.
- Add behavior tests for fork bombs, descriptor floods, memory/CPU pressure,
  oversized output, poll floods, concurrent owners, cancellation races, timeout
  races, and relay shutdown/reaping of descendant process trees.
- Verify no duplicate execution after an initial response loss and that keyed
  retries replay or conflict deterministically.

### 068-C — Filtered profile discovery

- Implement bounded, sandboxed probes for Unix `.profile`, `.bash_profile`,
  `.bashrc`, Fish `config.fish`, and Windows PowerShell profiles.
- Import only validated executable directories and an explicit non-secret
  variable allowlist; never source profiles in the relay process or inherit
  arbitrary environment values.
- Test probe timeout, bounded output, malformed output, alias/function
  confusion, world-writable directories, symlink/junction/reparse escapes,
  replacement races, PATHEXT resolution, Conda/Node/npm/pnpm/Cargo/Python and
  OS-native package-manager fixtures.

### 068-D — macOS containment

- Evaluate a signed App Sandbox/helper architecture with security-scoped file
  access, child lifetime/parent-death behavior, network policy, protected-path
  masking, and privilege-broker denial.
- Do not use deprecated `sandbox-exec` as a production substitute.
- Run the complete terminal security matrix on every claimed macOS version;
  otherwise keep the relay unavailable on macOS and document the bounded
  refusal.

### 068-E — Windows containment

- Evaluate AppContainer or an equivalent explicit resource broker, with
  restricted tokens only as defense in depth and Job Objects for process-tree,
  memory, CPU, handle, and kill-on-close limits.
- Test ACL/reparse-point/junction escapes, inherited handles, child breakaway,
  token privilege state, cancellation, timeout, and protected credential paths.
- Keep unsupported Windows combinations fail-closed and report a bounded
  capability reason.

### 068-F — Client persistence and context recovery

- Persist accepted task IDs and bounded output through stream abort, model-step
  timeout, context compaction, client restart, and relay restart boundaries.
- Ensure a child agent can access only an inherited parent-owned task reference
  and cannot widen tools/effects or poll another owner/session.
- Add end-to-end tests proving unknown post-restart tasks produce a safe
  no-rerun instruction and that ambiguous outcomes never trigger an unkeyed
  second execution.

## 2026-09-06 MCP Audit Reconciliation

The 2026-09-06 MCP security audit evaluated terminal execution boundaries, catalog consistency, and capability handling. This reconciliation classifies each finding into expected design invariants, confirmed verified boundaries, and remaining portability/quota scope.

### 1. Expected Design Behaviors (Not Defects)

- **Read-only `/etc` availability:** Inspecting `/etc` (e.g. `ls /etc` or reading `/etc/resolv.conf` / CA certificates) succeeds because Bubblewrap mounts `/usr`, `/lib`, `/etc`, `/bin`, and `/sbin` read-only (`ro-bind`). This is mandatory so toolchains, runtimes, dynamic linkers, and TLS stacks function. Write operations to `/etc` fail read-only (`Read-only file system`).
- **Terminal network via operator opt-in:** Network commands (e.g. `curl`) succeed only when an operator explicitly passes `--allow-terminal-network` / `RELAY_ALLOW_TERMINAL_NETWORK=true`. By default, network is unshared (`--unshare-net`), blocking all outbound connects and raw sockets. Dedicated HTTP/search tools (`http_fetch`, `web_search`) enforce independent SSRF/allowlist policies.
- **Developer CLI tools survive catalog simplification:** Standard developer tools (Git, Node.js, Python, Cargo, compilers, build systems) are intended terminal fallbacks. Removing high-level dedicated MCP wrappers (such as local Git or LSP wrappers) from the public MCP catalog does not ban standard developer CLI tools from running inside the terminal sandbox.
- **Broad `$HOME` execution root:** Setting `--execution-root "$HOME"` intentionally permits ordinary user-space file operations across sibling projects within `$HOME`. It functions as a hard ceiling, not a single-directory jail. System D-Bus, journal, host processes, `/tmp`, and credentials remain isolated.

### 2. Confirmed and Verified Boundaries (Behavior-Named Evidence)

Automated integration tests under `packages/rust-tools/tests/` verify all core terminal boundaries:

- **Network Layer Isolation (`security/terminal_network.rs`):** Verified that sandbox unshares network by default, rejecting outbound TCP/UDP connects and raw sockets; verified that operator opt-in restores loopback/network access; verified that dedicated HTTP policy remains independent.
- **Filesystem & Ceiling Enforcement (`security/terminal_filesystem.rs`):** Verified read-write within authorized workspace and sibling directories; verified read-only enforcement on `/etc` (`touch /etc/test_file` fails); verified `/tmp` is an isolated tmpfs; verified rejection of paths escaping `execution_root` via cwd or symlink traversal.
- **Recursive Credential & Socket Masking (`security/terminal_sandbox.rs`, `protected_paths.rs`):** Verified that nested `.ssh`, `.aws`, `.cargo/credentials`, `.env.*` (with `.env.example` readable) and nested Unix domain sockets (`.sock`) at arbitrary directory depths are masked with empty mounts before child spawn; verified child environment only retains minimal runtime keys (`LANG`, `PATH`, `TMPDIR`, `HOME`).
- **Privilege Escalation Denial (`terminal_policy.rs`, `security/terminal_sandbox.rs`):** Verified direct blocking and safe-PATH masking of `sudo`, `su`, `doas`, `pkexec`, `runas`, and direct SSH clients; verified `CapEff=0000000000000000` and Linux `PR_SET_NO_NEW_PRIVS`.
- **Wire Discovery & Invocation Consistency (`security/terminal_discovery.rs`):** Verified full modern wire client exchange: Full profile exposes 52 tools, Primary profile exposes 15 tools; tool invocation strictly matches profile allowlist; disabled capabilities return structured `CAPABILITY_REVOKED` errors; unknown tools return 404 errors.

### 3. Retained Scope in Plan 068

- **068-B:** Linux resource controls (cgroups/rlimits for memory, CPU, pids, fds) and lifecycle stress.
- **068-D:** macOS native containment via App Sandbox/helper architecture; fail-closed refusal until verified.
- **068-E:** Windows native containment via AppContainer / Job Objects; fail-closed refusal until verified.
- **068-F:** Client persistence, context recovery, and deterministic task resumption across restarts.

## Required evidence and closure

Run the repository Rust/web full guards plus the applicable official MCP
conformance scenarios. Produce positive and negative evidence for every row in
Plan 067's final acceptance matrix on every claimed platform. If any backend,
protocol version, or quota primitive cannot meet the matrix, leave that
combination explicitly unsupported; do not close the plan by weakening the
test or documentation.
