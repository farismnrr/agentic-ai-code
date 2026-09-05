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

## Required evidence and closure

Run the repository Rust/web full guards plus the applicable official MCP
conformance scenarios. Produce positive and negative evidence for every row in
Plan 067's final acceptance matrix on every claimed platform. If any backend,
protocol version, or quota primitive cannot meet the matrix, leave that
combination explicitly unsupported; do not close the plan by weakening the
test or documentation.
