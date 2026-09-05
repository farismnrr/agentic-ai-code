# Plan 067 — Terminal-First Multi-OS Execution and Context Hardening

**Status:** READY — reviewed for major-change risks, implementation not started
**Baseline:** `main` at `a7e4760a990b8e23afb59149c93d9ec752ebe04a` (Plan 065 closure)

## Goal

Make one logical terminal capability practical for broad, ordinary user-space
work across the operator's configured execution root and supported operating
systems, while retaining hard credential, privilege, SSH, and privileged-socket
boundaries. The terminal becomes the fallback for terminal-native work while
dedicated MCP tools remain the preferred route whenever they fully cover a
request. Replacing structured tools must not make a long command lose its task
identity, output, timeout, cancellation state, or conversational context.

## Audit findings and decisions

The repository was audited read-only before this plan was written. The active
checkout is the expected `masih-awam-ai-code` repository at
`/home/farismnrr/Projects/MasihAwam/ai-code`, with no tracked changes and an
unrelated untracked `agents/` directory that remains out of scope. The normal
Git commands need the repository's bare-worktree form (`--git-dir=.git
--work-tree=.`); the ordinary `git` invocation and `ai-self/tools/workspace-verify`
currently report “must be run in a work tree”. This is a repository/worktree
integration issue to account for in the baseline guard task, not a reason to
weaken the guard.

The current relay already has one authoritative `JobManager` for synchronous
processes, MCP Tasks, and `terminal_job_*` fallback jobs. It retains bounded
stdout/stderr with omission counts, exposes stable task IDs and timestamps,
supports idempotency for async terminal calls, polls with bounded backoff,
propagates explicit cancellation, kills/reaps process groups on timeout and
shutdown, and deliberately does not cancel durable work when an HTTP request
disconnects. The first-party client returns a task handoff on request abort or
poll timeout rather than starting the command again. These are the foundations
to preserve, not replace.

The current context risk is the synchronous path: a client-side MCP round-trip
deadline can expire while a synchronous terminal call is still running, before
the caller receives a task ID. `execution_mode=auto` avoids this for clients
that negotiate Tasks, but explicit sync calls and non-Tasks clients still need a
bounded handoff contract. `terminal_job_start` also has a separate legacy
schema without `execution_mode` and `idempotency_key`, so it cannot express the
same retry-safe lifecycle as `terminal_exec`.

The current task records are process-local and task get/cancel requests address
only a UUID; the record does not bind the task to an authenticated owner or
agent session. That is acceptable for a single-owner local smoke test but is a
cross-user disclosure/cancellation gap for a shared relay. “Durable” in the
current implementation means durable across the initial HTTP request, not across
relay restart. Plan 067 must make both meanings explicit and must not claim
restart recovery unless a separately reviewed persistent ledger is added.

Terminalizing local Git also changes a credential boundary: commands such as
`git remote -v` or `git config --get remote.origin.url` can print a token embedded
in a remote URL even when `.git-credentials` is masked. Environment scrubbing
prevents inherited GitHub tokens, but it does not sanitize secrets already stored
in repository config. The terminal fallback therefore needs URL-aware output
redaction and must keep remote network delivery on the dedicated credential
isolated bridge.

The current child environment is intentionally scrubbed (`env_clear`) and the
safe PATH contains system paths, `.cargo/bin`, `.local/bin`, and explicitly
reviewed toolchain paths. That protects credentials but hides common user
installations such as Conda/Anaconda, nvm/fnm/Volta/asdf, Homebrew, npm/pnpm
global bins, and Fish/Bash startup PATH changes. The terminal must discover
user-owned runtime paths without importing arbitrary secret environment values
or executing untrusted startup code in the relay process.

The usability boundary is deliberate: profile integration means discovering
safe executable/runtime paths and selected non-secret settings, not importing
aliases, shell functions, arbitrary startup side effects, or the entire login
environment. An explicit `bash -lc`, Fish, or PowerShell command may still use
normal shell semantics inside the already restricted sandbox. This keeps the
terminal close to the user's installed toolchain without turning `.bashrc` or
equivalent files into an authority or credential bypass.

The relay command is currently compiled for portable CLI artifacts but refuses
to run on macOS and Windows because the only process sandbox is Linux
Bubblewrap. Therefore “all OS” cannot be claimed by merely widening the Linux
execution-root check. Plan 067 must add an explicit per-OS sandbox backend or
fail closed on an OS where equivalent credential and privilege guarantees
cannot be proven. A raw host shell is not an acceptable fallback.

The current full catalog contains 102 tools and the v14 static contract is
immutable. The agreed terminal-first target is:

| Surface | Keep in current catalog | Reason |
| --- | ---: | --- |
| Terminal lifecycle | 4 | One terminal path plus durable get/cancel fallback |
| Workspace authority | 4 | Explicit scope and revocation remain a security boundary |
| Structured filesystem/search | 7 | Bounded, redacted, atomic and observable semantics |
| Remote Git credential bridge | 4 | Fetch/push/remote branch actions isolate credentials |
| Forge/issues/actions/security | 29 | Provider policy and credential isolation are not generic CLI work |
| Dedicated SSH diagnostics | 1 | Generic `ssh`/`scp`/`sftp` stays blocked |
| Dedicated HTTP/web | 2 | SSRF/search policy remains separate from generic network |
| Telegram messaging | 1 | Relay-owned credential and destination policy |
| **Target total when all optional integrations are enabled** | **52** | 15 core tools plus 37 opt-in integrations |

The 43 local Git wrappers (including local remote/worktree management) and 7
LSP/code-intelligence tools become terminal workflows or build/typecheck/test
validation paths. The implementation must remove their client-visible catalog
entries and then perform an orphan/dead-code sweep; it must not delete shared
security or Git credential-isolation code still used by the retained bridge.
Historic v14 remains unchanged; a new immutable snapshot is required for the
new serialized surface.

Because the platform backend is the largest architectural uncertainty, the
implementation order is intentionally staged: prove terminal lifecycle and
profile discovery first, prove each OS sandbox primitive second, and only then
remove client-visible Git/LSP tools. Catalog pruning must never be used to hide
an incomplete execution backend or a context-handoff regression.

## Industry-standard baseline reviewed 2026-09-06

The implementation must conform to the strongest applicable protocol and OS
guidance rather than inventing a relay-specific interpretation:

- **MCP task protocol:** support the negotiated wire version explicitly. The
  2025-11-25 experimental `tasks.*` flow and the newer
  `io.modelcontextprotocol/tasks` extension are not wire-compatible. A task
  response uses the reserved `resultType: "task"` discriminator, carries a
  receiver-generated high-entropy ID, status, timestamps, TTL, and poll
  interval, and returns the underlying result/error without changing its
  meaning. Every get/result/cancel operation performs authorization in the
  same context; task listing is disabled unless the server can scope it. The
  relay may keep its `terminal_job_*` compatibility surface, but it must be an
  adapter over the negotiated MCP lifecycle and must never advertise a private
  task shape as protocol-native.
- **Task resource controls:** publish and enforce maximum task TTL, retained
  output, concurrent tasks per owner, polling rate, and total retained jobs.
  Expired tasks are deleted promptly; cancellation has deterministic API
  semantics, commits `cancelled` before the response, and cannot move a
  terminal task back to `working`. These limits prevent an otherwise safe async
  API from becoming a memory, process, or task enumeration denial-of-service.
- **Process supervision:** every child is in a killable process group/tree,
  receives a parent-death policy, has bounded stdout/stderr with backpressure,
  and is terminated with a documented graceful-to-forceful sequence. Apply
  operator-bounded CPU, memory, file-descriptor/process-count, output, and
  concurrency limits where the host primitive supports them; where it does
  not, fail closed or record the exact weaker guarantee. A timeout or client
  disconnect must never orphan descendants.
- **Sandbox primitives:** Linux Bubblewrap remains defense in depth, including
  `--new-session` where required for TTY injection protection and
  `--die-with-parent`/equivalent parent-death handling, plus no-new-privs,
  dropped capabilities, namespace isolation, and explicit mounts. macOS App
  Sandbox is an entitlement-based kernel boundary and does not grant broad
  home access by default; a command-line relay therefore needs a reviewed
  signed/helper architecture or must fail closed. `sandbox-exec` is deprecated
  and may not be treated as an industry-supported default. On Windows,
  restricted tokens alone are insufficient for filesystem/process isolation;
  AppContainer or an equivalent explicit resource broker must be evaluated,
  with Job Objects enforcing process-tree and resource limits.
- **Environment and profile loading:** use an explicit allowlist. A profile
  probe is untrusted code and runs in the same restricted, read-only, bounded
  environment as the command. Import only validated executable directories and
  documented non-secret variables; reject world-writable or symlink/junction
  escapes and revalidate paths immediately before spawn. Never inherit
  arbitrary environment, startup side effects, aliases, functions, or
  credential-shaped variables.
- **Idempotent retries and observability:** an idempotency key is paired with a
  server-computed request fingerprint, has a documented expiry, replays the
  original result for completed duplicates, and returns a conflict for a
  concurrent mismatched/in-flight reuse. Structured logs record lifecycle and
  authorization events but sanitize command arguments, tokens, URLs, session
  identifiers, and private paths; untrusted output is also log-injection
  sanitized.

These requirements are acceptance criteria for the phases below. “Evaluate”
means produce a security-reviewed proof or an explicit fail-closed support
decision; it is not permission to ship a weaker substitute.

Reference specifications and platform guidance:

- [MCP Tasks Extension](https://tasks.extensions.modelcontextprotocol.io/specification/draft/tasks)
  and [SEP-2663](https://tasks.extensions.modelcontextprotocol.io/seps/2663-tasks-extension)
- [MCP 2025-11-25 Tasks](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks)
- [Bubblewrap upstream README](https://github.com/containers/bubblewrap/blob/main/README.md)
- [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox)
- [Windows AppContainer isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation),
  [Restricted Tokens](https://learn.microsoft.com/en-us/windows/win32/secauthz/restricted-tokens),
  and [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [IETF Idempotency-Key draft](https://datatracker.ietf.org/doc/html/draft-ietf-httpapi-idempotency-key-header-07)
- [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)

## Non-negotiable security and authority contract

- The configured execution root remains the hard filesystem ceiling. With
  `execution_root=$HOME` and `dir=$HOME`, ordinary non-protected files beneath
  the operator home are usable. Narrower roots remain narrow.
- Linux continues to use Bubblewrap with isolated `/dev`, `/proc`, `/tmp`,
  no-new-privs, dropped capabilities, cleared environment, protected-path
  masking, generic SSH client masking, privilege-broker masking, and explicit
  optional socket binds.
- macOS and Windows may use only a reviewed OS-native sandbox/restricted-token
  backend that proves equivalent containment. If the required primitive is not
  present or cannot mask credentials/privilege brokers safely, the relay must
  refuse to start or refuse execution on that host; it must not run unsandboxed.
- `sudo`, `su`, `doas`, `pkexec`, and `runas` remain denied directly and through
  shell/interpreter wrapping. The sandbox must mask or deny every visible
  spelling of those brokers and must not expose setuid helpers, privileged
  sockets, D-Bus brokers, or equivalent escalation paths.
- `.ssh`, `.gnupg`, `.aws`, `.config/gcloud`, `.config/gh`, `.docker`, `.kube`,
  `.npmrc`, `.netrc`, `.pypirc`, `.git-credentials`, Cargo credentials, and
  `.env`/`.env.*` remain protected. `.env.example` remains intentionally
  readable. Nested copies and credential-agent/keyring sockets remain hidden.
- The child receives only a filtered environment. User PATH/profile discovery
  may contribute executable directories and non-secret runtime settings, never
  arbitrary inherited variables or secret values. If safe protected-path
  discovery cannot complete within bounds, execution fails closed.
- Generic `ssh`, `scp`, and `sftp` stay unavailable from terminal. Remote
  diagnostics continue through `ssh_readonly_exec`; Docker/Tailscale remain
  explicit operator opt-ins.
- Task get/cancel is owner- and session-scoped. A task identifier alone never
  grants another authenticated user or child agent access. Relay restart makes
  in-memory tasks unavailable with a bounded status; the client must not rerun
  an operation automatically.
- Approval, effect intersection, plan/read-only mode, subagent authority, and
  delegation limits are unchanged. Broad terminal visibility does not grant a
  child tools or effects it did not inherit.

## Terminal and context contract

One logical terminal surface consists of `terminal_exec` plus
`terminal_job_get`/`terminal_job_cancel`; `terminal_job_start` remains for
clients that do not negotiate MCP Tasks. Both start paths must use the same
argv, profile, timeout, output, idempotency, and task-state contract.

The compatibility wire contract must be explicit: a start that is accepted for
background execution returns a normalized task envelope (`resultType=task`) with
the task ID and continuation metadata. Legacy clients may receive the existing
text-wrapped representation only through a compatibility adapter, and the
first-party client must normalize both forms before returning a model-visible
tool result. A task is durable for the lifetime of the relay process; restart
recovery is out of scope unless a persistent, owner-scoped ledger is introduced.

For MCP 2025-11-25 clients, preserve the negotiated legacy `tasks/get` and
`tasks/result` behavior; for clients negotiating the `io.modelcontextprotocol/tasks`
extension, use its `tasks/get`, `tasks/update`, and `tasks/cancel` lifecycle and
do not emit the removed `tasks/result` method. Capability negotiation and the
tool's task support must be checked per request. The client must respect the
server-provided poll interval and task TTL, while the relay clamps both to
operator limits. If the transport is Streamable HTTP and the extension is in
use, send the extension's task-routing headers so a stateful task reaches the
instance that owns it. Do not advertise `tasks/list` when authorization context
cannot scope the result.

Every accepted asynchronous operation must hand back a stable `taskId`,
creation/update timestamps, status, bounded output, omission count, exit code
when known, execution status (including timeout), and a safe continuation
message that says how to resume. The handoff must never echo raw command
arguments, credentials, private absolute paths, or provider errors. A task
result must remain understandable after the initial model step, an HTTP
disconnect, a polling deadline, or a model context compaction event.

The implementation must publish bounded limits for per-owner concurrency,
polling frequency, task TTL, retained output, and total in-memory jobs. It must
return the protocol's invalid-parameter shape for unknown or unauthorized task
IDs, reject cancellation after a terminal state, and keep a task cancelled once
that state is committed even if a child exits later. Concurrent idempotency-key
reuse with a different request fingerprint is a conflict; a completed duplicate
replays the original bounded result until the documented key/task expiry.

The relay must distinguish these deadlines:

1. command deadline (`timeout_ms`, capped by operator policy);
2. task retention/TTL and output retention;
3. MCP HTTP round-trip deadline;
4. AI model step/total deadline; and
5. subagent wall-time and tool-call budgets.

No deadline may silently turn an unknown command outcome into permission to
rerun it. Sync calls must either complete inside the negotiated request budget
or return a durable task handoff. The relay must start the job before waiting
and use a bounded synchronous handoff window; when that window expires it
returns the task envelope instead of continuing an untracked HTTP response.
Async retries must use the same idempotency identity. Request disconnects may
stop polling/model work, but never implicitly cancel a relay process that has
already been accepted.

## Phase overview

| Phase | Goal | Depends on | Exit criterion |
| --- | --- | --- | --- |
| 00 | Baseline and authority freeze | — | Identity/status/guard baseline recorded; unrelated files untouched |
| 01 | Unify terminal lifecycle and context handoff | 00 | Sync/async/job-start use one durable contract with no lost task identity |
| 02 | User profile and runtime discovery | 00, 01 | Conda/node/npm/pnpm/cargo and shell profile paths work without secret env inheritance |
| 03 | Per-OS sandbox backends | 00, 01, 02 | Linux remains Bubblewrap; macOS/Windows run only with proven containment |
| 04 | Broad-root credential and privilege hardening | 02, 03 | Home-scale masking and shell-wrapped escalation denial pass on every supported OS |
| 05 | Terminal-first catalog and MCP routing | 01, 03 | 52-tool maximum surface, immutable snapshot, active-tool-derived guidance |
| 06 | Agent/subagent context and fallback ergonomics | 01, 05 | Primary/child agents preserve authority and resumable terminal context |
| 07 | Documentation and focused validation | 02–06 | Docs, tests, catalog and guards reflect final behavior |
| 08 | Orphan sweep and closure | 07 | No stale client surface/dead guidance; truthful closure evidence |

# PHASE-00 — Baseline and contract freeze

## TASK-001 — Revalidate repository identity and worktree form

**Files:** `ai-self/project.yaml`, `ai-self/tools/workspace-verify`, Git metadata

- [ ] Use the repository-native bare-worktree command form where required and
      record exact branch, HEAD, origin, and status.
- [ ] Fix only the verification invocation or documentation if the guard
      assumes a non-bare worktree; do not rewrite repository metadata as part of
      this plan.
- [ ] Preserve the existing untracked `agents/` directory byte-for-byte and
      keep it unstaged.
- [ ] Record the v14 catalog snapshot as immutable baseline.

**Validation:** workspace identity, `git status --short --branch`, origin, and
catalog snapshot checks succeed without modifying unrelated files.

## TASK-002 — Capture current lifecycle and profile behavior

**Files:** `packages/rust-tools/src/application/execution/{jobs.rs,process.rs,requests.rs}`;
`packages/rust-tools/src/infrastructure/transport/{tools.rs,task_calls.rs,task_lifecycle.rs,mcp_http.rs}`;
`server/infrastructure/mcp/{modern-http-client.ts,task-reliability.ts}`;
existing terminal/task tests

- [ ] Record current timeout, cancellation, retention, idempotency, and request
      disconnect behavior with focused tests before changing contracts.
- [ ] Record current safe PATH/toolchain resolution and supported OS behavior.
- [ ] Confirm no existing test depends on the unrelated `agents/` directory.

# PHASE-01 — Unify terminal lifecycle and context handoff

## TASK-010 — Make start paths contract-equivalent

**Files:** `packages/rust-tools/src/application/execution.rs`;
`packages/rust-tools/src/application/execution/requests.rs`;
`packages/rust-tools/src/interfaces/mcp/catalog.rs`;
transport task handlers and Rust task tests

- [ ] Give `terminal_job_start` the same `execution_mode` and
      `idempotency_key` semantics as `terminal_exec`, or make it a thin
      compatibility wrapper over the same validated start function.
- [ ] Define the exact compatibility response for legacy clients: current
      text-wrapped job results may remain accepted for one transition window,
      but `tools/call` and the first-party client must expose one normalized
      task envelope to the model and UI. Add schema and contract tests for both
      forms.
- [ ] Keep direct argv semantics and explicit shell invocation; preserve all
      argument/cwd bounds and approval/effect checks.
- [ ] Ensure sync execution cannot leave an accepted process without a
      recoverable task identity when the client deadline expires. Prefer the
      existing MCP Tasks path when negotiated; otherwise return the normalized
      terminal task handoff before a long wait.
- [ ] Keep idempotency scoped to authenticated owner, tool, and exact argument
      fingerprint; never deduplicate different commands under one key.
- [ ] Register the identity before spawning for keyed sync calls as well as
      async calls, so a timeout-triggered async retry resolves the original job
      instead of starting a second process. Remove the identity only according
      to the existing bounded task-retention policy.
- [ ] Bind every job to the authenticated owner and agent/session scope that
      started it. `tasks/get`, `tasks/cancel`, and `terminal_job_*` must reject
      cross-owner or cross-child access with the same bounded not-found/error
      shape, without revealing whether another owner's task exists.
- [ ] Make the first-party client generate a bounded per-logical-call
      idempotency key for terminal starts when the caller did not supply one;
      preserve an explicit caller key when present. Never derive the key from
      raw secret-bearing arguments in logs or UI.
- [ ] On an initial terminal round-trip timeout after the request may have been
      accepted, retry exactly once with the same key and `execution_mode=async`;
      normalize the existing task or completed result. If the retry cannot
      establish the task, return an explicit unknown-outcome message and never
      issue an unkeyed second execution.

**Tests:** sync short command, long auto command, explicit async retry,
non-Tasks fallback, timeout before response, duplicate key, mismatched key,
request disconnect, owner/session isolation, and no duplicate process execution.

## TASK-011 — Preserve resumable output and safe operation identity

**Files:** `jobs.rs`, `process.rs`, `task_calls.rs`, `task_lifecycle.rs`,
`mcp_http.rs`, `tools.rs`,
`modern-http-client.ts`, `task-reliability.ts`, activity/presentation tests

- [ ] Keep bounded head/tail output behavior and omission counts; add a stable
      non-secret operation label or digest only if needed to identify a task
      after context compaction. Never include raw sensitive args.
- [ ] Ensure timed-out and cancelled tasks retain the last known output and
      explicit state without being presented as successful completion.
- [ ] Ensure polling after request abort returns the latest task envelope and a
      clear “do not rerun” continuation message.
- [ ] Ensure unknown/expired tasks produce bounded, non-sensitive errors and do
      not include command text or environment values.
- [ ] Add URL-aware redaction for Git remote userinfo, bearer/basic-auth URL
      forms, and credential-shaped command output before it reaches task JSON,
      activity detail, logs, or the model. Keep ordinary non-secret Git URLs
      readable enough for diagnostics.

**Tests:** output truncation, redaction, task expiry, poll timeout, abort during
initial call, abort during poll, cancellation, owner isolation, Git remote URL
redaction, and activity/log confidentiality.

## TASK-012 — Align deadline layers

**Files:** `core/config.rs`, execution process/task code, MCP client reliability,
AI/subagent timeout adapters

- [ ] Document and test command deadline versus MCP round-trip, task TTL, AI
      step, and subagent wall-time limits.
- [ ] Clamp or reject impossible timeout combinations with bounded errors;
      never silently convert an in-flight sync command into an untracked retry.
- [ ] Preserve `timeout_ms=0` semantics as operator-capped/unbounded according
      to existing configuration, while making the client-facing handoff safe.
- [ ] Add a deterministic policy for explicit synchronous terminal calls whose
      requested runtime can exceed the MCP round-trip deadline: either reject
      the impossible request before spawning or promote it to a durable task
      with an idempotent handoff. Never start an untracked synchronous process.
- [ ] Implement the handoff as “start once, wait only for the bounded sync
      window, then return the task envelope”; do not rely on guessing a remote
      client's timeout. The first-party client must use an idempotency key for
      retries after a lost response.

# PHASE-02 — User profile and runtime discovery

## TASK-020 — Add filtered user runtime profile resolution

**Files:** `packages/rust-tools/src/application/execution/toolchain.rs` and a
small platform/profile module under the existing execution folder; config/CLI;
Rust profile tests and operator docs

- [ ] Resolve the operator's shell/profile source using the OS-native shell
      (`$SHELL`/Bash/Fish on Unix, PowerShell or configured shell on Windows)
      without sourcing it inside the relay process.
- [ ] Use a bounded helper invocation or static profile-path discovery to
      obtain PATH/runtime directories. Apply a strict timeout and a bounded
      output limit; a failed probe falls back to the minimal safe PATH with a
      safe diagnostic rather than blocking all terminal work.
- [ ] If a helper invocation is used, run it as a read-only, no-network child
      under the same platform sandbox and protected-path mask as terminal work;
      clear its environment first and import only a validated PATH result. The
      relay process must never source `.bashrc`, `config.fish`, or a PowerShell
      profile directly.
- [ ] Accept only canonical executable directories that are user-owned,
      non-world-writable, and either beneath the configured execution root or a
      reviewed system/runtime location. Never import arbitrary environment
      variables or shell aliases/functions.
- [ ] Treat every discovered directory as untrusted between discovery and
      spawn: open/revalidate it with the platform's safest available
      descriptor/ACL checks immediately before execution, reject symlink,
      junction, mount-point, and reparse-point escapes, and never let a profile
      rewrite change the command's resolved executable after validation.
- [ ] Recognize common layouts for Conda/Anaconda, nvm/fnm/Volta/asdf, npm and
      pnpm global bins, Cargo/Rustup, Python virtual environments, Homebrew,
      and ordinary user-local bins on each supported OS. Keep operator
      `--toolchain-path` as an explicit override.
- [ ] Define how discovered runtime directories are mounted into each sandbox
      when they live outside the execution root (for example Homebrew under
      `/opt/homebrew` or a system-level Node installation). Mount only reviewed
      read-only executable/runtime paths; never mount an entire host home or
      package-manager credential store as a side effect of PATH discovery.
- [ ] Keep `HOME`, locale, temp, and selected non-secret toolchain variables
      deterministic. Credential-shaped variables and arbitrary inherited
      values remain absent.
- [ ] Define and test the non-secret variable allowlist explicitly (for
      example active runtime prefixes and package-manager cache locations),
      reject names and values matching token/password/key/credential patterns,
      and never import `GH_TOKEN`, cloud credentials, registry auth, proxy
      passwords, or helper configuration merely because a profile exports them.

**Tests:** Bash/Fish profile PATH discovery, Conda/node/npm/pnpm/cargo fixtures,
malicious profile timeout, world-writable directory rejection, symlink escape,
secret environment non-inheritance, and profile probe failure fallback.

The profile test matrix must include Unix `.profile`, `.bash_profile`,
`.bashrc`, and `config.fish`, plus Windows PowerShell profile locations and
`PATHEXT` executable resolution. Tests must prove that aliases/functions are
not silently treated as executable files and that profile output cannot inject
new command arguments.

## TASK-021 — Make shell execution explicit but ergonomic

**Files:** terminal catalog schema/descriptions, request validation, docs, client
tests

- [ ] Keep direct argv as the default. Permit `bash -lc`, `fish -lc`,
      `pwsh -NoProfile -Command`, or the platform equivalent only when the shell
      executable resolves from the filtered runtime PATH.
- [ ] Pass a nonce-based, machine-readable probe request and parse only the
      returned PATH/runtime schema; do not accept arbitrary stdout as shell
      arguments. Probe output, startup diagnostics, and exit status are bounded
      and sanitized before they reach task results or logs.
- [ ] Do not automatically run arbitrary startup scripts for every command;
      use the profile probe to discover runtime paths, and let an explicit shell
      command request normal shell semantics inside the sandbox.
- [ ] Ensure shell startup files cannot bypass privilege masking or credential
      isolation.

# PHASE-03 — Per-OS sandbox backends

## TASK-030 — Preserve and test the Linux Bubblewrap backend

**Files:** `packages/rust-tools/src/application/execution/sandbox.rs`,
`sandbox/masks.rs`, `sandbox/paths.rs`, Linux security tests

- [ ] Keep Bubblewrap mandatory on Linux; keep isolated namespaces, no-new-privs,
      capability drop, environment clearing, protected-path discovery, generic
      SSH masking, privilege-broker masking, and explicit socket opt-ins.
- [ ] Audit the generated Bubblewrap argv against the current upstream
      security guidance: use a new session when a TTY is possible, bind a
      parent-death policy, close inherited file descriptors, and apply seccomp
      or equivalent filters where the existing contract requires them. Add a
      regression test that a shell wrapper cannot inject commands through a
      controlling terminal or inherited descriptor.
- [ ] Add operator-bounded CPU, memory, process-count, file-descriptor, and
      output limits using cgroups/rlimits or the closest supported primitive;
      record each limit in the task's execution status and fail closed when a
      required limit cannot be installed.
- [ ] Permit `$HOME` as the sandbox root only after bounded recursive masking
      succeeds; fail closed on scan overflow, metadata errors, symlinked
      protected entries, or protected-state overlap.
- [ ] Prove ordinary files work across multiple home subdirectories and nested
      credential families remain inaccessible.

## TASK-031 — Add reviewed macOS execution containment

**Files:** cfg-gated sandbox backend, relay command startup, macOS tests/docs

- [ ] Evaluate a supported macOS entitlement/helper architecture and its
      kernel-enforced file, network, process, and credential boundaries against
      the exact contract. Treat the deprecated `sandbox-exec` command as a
      research comparison only, never as the default production backend.
- [ ] Generate a minimal deny-by-default profile for protected files/sockets and
      privilege brokers, with the configured root writable and system/runtime
      paths read-only as appropriate.
- [ ] Prove signing, entitlement distribution, helper lifetime, parent-death,
      child-process, and update behavior on each claimed macOS version. A
      command-line artifact without that proof remains unsupported.
- [ ] If the host lacks a reliable primitive or the proof cannot cover setuid,
      process, and path behavior, refuse relay execution rather than invoking a
      raw shell.

## TASK-032 — Add reviewed Windows execution containment

**Files:** cfg-gated sandbox backend, relay command startup, Windows tests/docs

- [ ] Evaluate AppContainer or an equivalent explicit resource broker first;
      use restricted tokens as defense in depth, not as the filesystem boundary.
      Pair the sandbox with a Job Object that prevents process breakaway and
      enforces process-count, memory, CPU, handle, and kill-on-close limits.
      Keep the relay unprivileged.
- [ ] Enforce execution-root containment, protected credential paths, process
      tree limits, cancellation, timeout, and no privileged broker exposure.
- [ ] Test reparse-point/junction escapes, inherited handles, child process
      breakaway, low-integrity/token privileges, and ACL changes while a task is
      running. A failure in any boundary makes that Windows combination
      unsupported.
- [ ] Refuse execution when the required restriction cannot be established;
      portable CLI availability must never imply unsandboxed relay authority.

## TASK-033 — Freeze the platform support matrix before implementation

**Files:** platform backend design note, `commands/relay.rs`, release/build
documentation, platform CI or host-run evidence

- [ ] Define the exact supported combinations of OS, architecture, shell,
      filesystem, and sandbox primitive. “Builds on macOS/Windows” is not proof
      that the relay is supported there.
- [ ] Make startup capability negotiation report the backend and fail-closed
      reason without exposing host paths or implementation errors.
- [ ] Do not mark Plan 067 complete until every claimed OS passes the same
      positive/negative security matrix. If a backend is not ready, record that
      OS as intentionally unsupported and split its implementation into a
      follow-up plan rather than silently weakening the contract.

## TASK-034 — Share task manager across OS backends

**Files:** execution process/job modules and cfg tests

- [ ] Keep one `JobManager`, output buffer, timeout, cancellation, retention,
      idempotency, and task JSON contract for every backend.
- [ ] Use OS-specific process-group/tree termination and reaping, with tests for
      child processes and shell wrappers.
- [ ] Centralize admission control and resource accounting in `JobManager`:
      per-owner concurrency, global concurrency, poll-rate limiting, task TTL,
      output-retention bytes, and descriptor/process quotas must be enforced
      before spawn and released on every terminal path, including panic,
      cancellation, timeout, and relay shutdown.

## TASK-035 — Preserve execution-root versus workspace authority semantics

**Files:** `application/execution/paths.rs`, `core/workspace_path.rs`, Git
worktree handling, workspace tools, terminal security tests, active docs

- [ ] Keep `execution_root` as the hard ceiling and `--dir`/explicit workspace
      roots as the active authorization set. Only the intentional
      `--dir "$HOME" --execution-root "$HOME"` profile exposes the whole
      non-protected home tree; a repository `--dir` must not gain sibling access
      merely because the ceiling is HOME.
- [ ] Ensure terminal `git worktree add` cannot implicitly register a new
      workspace or expand authority. A worktree outside the active allowlist
      remains unavailable until explicitly authorized; a HOME-wide profile
      remains bounded by the existing HOME ceiling and masks.
- [ ] Keep relative and absolute cwd resolution consistent across all backends,
      including Windows drive/UNC paths and symlink/junction behavior.
- [ ] Do not advertise host `systemctl --user` or `journalctl --user` as
      guaranteed terminal capabilities while the sandbox has no reviewed user
      bus/journal bridge. Sandbox-local service/process commands remain valid;
      any future host bridge requires a separate authority review.

# PHASE-04 — Broad-root credential and privilege hardening

## TASK-040 — Home-scale protected-path discovery

**Files:** `core/protected_paths.rs`, `sandbox/masks.rs`, platform mask modules,
Rust security tests

- [ ] Preserve the canonical protected-path source; do not create a second
      terminal-only list.
- [ ] Define platform-specific equivalents for credential stores (for example
      Windows `.ssh`/`.gnupg`/`.aws` locations, `%APPDATA%` and `%USERPROFILE%`
      variants) while keeping one canonical policy model and one test matrix.
- [ ] Make scan limits configurable only through reviewed operator bounds, keep
      deterministic skip rules, and fail closed when discovery is incomplete.
- [ ] Replace the current implicit home-tree scalability assumption with an
      explicit scan strategy and measured budget. The strategy must cover newly
      created nested `.env*` files and sockets without silently skipping
      dependency/cache trees; if the bounded scan cannot finish, return a safe
      refusal and recommend a narrower execution root. Record benchmark results
      for realistic 100k, 500k, and multi-million-entry home fixtures.
- [ ] Cover top-level and nested `.env*`, all listed credential families,
      Cargo credentials, sockets, symlink tricks, and `.env.example` exception.

## TASK-041 — Shell-wrapped privilege denial

**Files:** `terminal_policy.rs`, sandbox mask/profile modules, security tests

- [ ] Keep direct executable validation for `sudo`, `su`, `doas`, `pkexec`, and
      `runas`.
- [ ] Mask/deny each broker at every safe-PATH spelling in every backend so
      `sh -lc`, Bash/Fish, PowerShell, Python, Node, or equivalent wrappers
      cannot invoke it.
- [ ] Prove no-new-privs/restricted-token behavior and absence of privileged
      sockets, D-Bus brokers, setuid helpers, or equivalent escalation paths.

## TASK-042 — Keep SSH and optional sockets separate

**Files:** sandbox profiles, terminal policy, SSH tests, configuration docs

- [ ] Keep generic `ssh`/`scp`/`sftp` unavailable and dedicated
      `ssh_readonly_exec` functional.
- [ ] Keep Docker/Tailscale exposure opt-in only; do not expose host sockets,
      agent/keyring sockets, or credential helpers through profile discovery.
- [ ] Keep remote Git network delivery on the credential-isolated dedicated
      bridge. Terminal Git may inspect or edit local remote configuration, but
      output containing URL userinfo, PAT-like query fragments, or helper
      credentials must be redacted and terminal Git must not receive bridge
      tokens or credential-helper forwarding.

# PHASE-05 — Terminal-first catalog and MCP routing

## TASK-050 — Publish the reduced immutable catalog

**Files:** `packages/rust-tools/src/interfaces/mcp/catalog.rs`, profile filter,
`packages/rust-tools/src/application/execution.rs`, Git/LSP dispatch modules,
new `.agents/contracts/067-tool-catalog-v15.json`, catalog tests/docs

- [ ] Remove client-visible local Git wrappers and LSP/code-intelligence tools
      from the current catalog while retaining only code used by retained
      capabilities or validation.
- [ ] Keep terminal lifecycle, workspace authority, structured filesystem,
      remote Git bridge, forge/integration, SSH, HTTP/web, and Telegram surfaces
      according to the 52-tool target.
- [ ] Keep primary profile at the agreed 15 core tools; optional integrations
      remain explicitly configurable.
- [ ] Retain v14 unchanged and add a deterministic v15 snapshot; update only
      the current snapshot pointer/contract.
- [ ] Make tools removed from the current catalog fail as unknown before
      dispatch; do not leave a hidden callable path merely because an internal
      dispatcher still recognizes the historical name.

## TASK-051 — Migrate stored tool selections without widening authority

**Files:** `server/infrastructure/mcp/capabilities.ts`, conversation/tool
selection APIs and UI, shared tool identity helpers, migration tests

- [ ] Treat removed local Git/LSP IDs in existing conversations as unavailable
      and surface a bounded migration state; do not silently substitute a
      broader terminal authority or mutate approval records.
- [ ] Ensure newly composed tool maps contain only tools present in the current
      catalog and active server inventory. Unknown historical IDs must not be
      sent to the model, used for approval lookup, or granted to subagents.
- [ ] Preserve retained remote Git/forge/SSH/HTTP/Telegram IDs and their
      approval semantics across the catalog snapshot transition.

## TASK-052 — Preserve active-tool-derived MCP-first guidance

**Files:** `server/application/chat/tool-selection-policy.ts`,
`execute-chat-turn.ts`, subagent prompt/runtime, routing tests

- [ ] Keep one shared policy composer derived from the final model-facing tool
      map for primary and delegated agents.
- [ ] Prefer active structured filesystem, Git, network, SSH, forge, and
      messaging tools when they fully cover the request.
- [ ] Describe terminal as the fallback for builds, tests, package managers,
      interpreters, scripts, process inspection, sandbox-local services,
      pipelines, and uncovered operations. Do not ban legitimate terminal Git
      use globally or imply a host user-bus bridge that is not configured.
- [ ] Ensure the policy never names a disabled tool, grants authority, changes
      approval/effect intersection, or invents a discovery call.

# PHASE-06 — Agent/subagent context and fallback ergonomics

## TASK-060 — Persist terminal handoff in the model-visible conversation

**Files:** `server/infrastructure/mcp/modern-http-client.ts`, AI SDK stream/
persistence adapters, task-context tests

- [ ] Ensure accepted task IDs and resumable status survive initial stream abort,
      model-step timeout, context compaction, and assistant persistence.
- [ ] Keep terminal task output bounded/redacted before it enters model/UI
      context; classify large output and use existing references/continuations
      where appropriate.
- [ ] Ensure a child agent receives only its inherited terminal authority and a
      bounded task reference, and can poll/cancel only parent-owned work.
- [ ] Test the case where the initial MCP response is lost after process
      acceptance but before assistant persistence; the next model turn must be
      able to recover the task through the idempotency identity or an explicit
      persisted handoff, without rerunning the command.
- [ ] Test relay shutdown/restart semantics explicitly: running jobs are
      cancelled and reaped on graceful shutdown; after a restart, an old task
      ID is reported as unavailable and the model is told not to infer success
      or rerun automatically.

## TASK-061 — Validate terminal fallback without routing regressions

**Files:** primary/subagent routing tests, capability/effect tests, docs

- [ ] Prove dedicated tools remain selected for covered operations.
- [ ] Prove terminal remains selectable for build/test/package-manager,
      interpreter, project-script, service, pipeline, and unsupported operations.
- [ ] Prove read-only agents do not receive terminal mutation authority and
      writer agents do not gain tools/effects through catalog pruning.

# PHASE-07 — Documentation and focused validation

## TASK-070 — Update active documentation

**Files:** `README.md`, `docs/architecture.md`, `docs/configuration.md`,
`docs/security.md`, `docs/getting-started.md`, `docs/troubleshooting.md`,
`packages/rust-tools/README.md`, `packages/relay-agent/SKILL.md`,
`.agents/knowledge/tooling.md`, relevant `.agents/agents/*.md`

- [ ] Document execution-root semantics, `$HOME` profile, user runtime/profile
      discovery, credential and privilege boundaries, SSH/socket boundaries,
      per-OS support/fail-closed behavior, task timeout/cancel/resume semantics,
      and the dedicated-tool → terminal fallback hierarchy.
- [ ] Remove stale claims that 102 tools, local Git wrappers, or LSP tools are
      current client-visible capabilities.
- [ ] Do not rewrite historical Plan 065; link to it as the predecessor and
      record only current behavior in active guidance.

## TASK-071 — Focused behavior gates

**Rust:** `pnpm guardrail:rust` plus behavior-named tests covering home-root
access, profile discovery, nested credential masking, `.env.example`, direct and
wrapped privilege denial, SSH blocking, optional sockets, timeout/cancel,
idempotency, MCP legacy/extension task-shape compatibility, task ownership,
resource quotas (concurrency, polling, TTL, output, CPU/memory/process/FD),
platform backend refusal/selection, and cross-platform cfg behavior.

**Nuxt/server:** `pnpm guardrail:nuxt` plus routing, MCP task reliability,
assistant persistence, context compaction, subagent authority, and catalog
composition tests.

**Catalog/docs:** immutable v14 preservation, deterministic v15 snapshot,
current 52-tool surface, no stale names in active docs, and no secret/path
leakage in errors/activity/logs.

At final closure run `pnpm guardrail:full` and the repository's required Rust and
web test commands. Do not add Plan-067-named verification scripts.

# PHASE-08 — Orphan sweep and closure

## TASK-080 — Remove only proven dead client surface

- [ ] Search for every removed local Git/LSP tool name in catalog, dispatch,
      schemas, tests, UI categories, docs, and agent profiles.
- [ ] Remove dead production modules only when no retained path uses them;
      preserve shared security, workspace, Git bridge, and validation code.
- [ ] Confirm no generated files, unrelated `agents/` content, secrets, or
      historical snapshots changed.

## TASK-081 — Final security/diff review and truthful closure

- [ ] Inspect the full diff for raw-host-shell regressions, credential exposure,
      privilege/socket expansion, authority intersection changes, and accidental
      catalog snapshot mutation.
- [ ] Record focused test and guard results, exact final branch/HEAD and commits,
      and any OS backend that remains intentionally fail-closed or unsupported.
- [ ] Mark this plan **CLOSED / VERIFIED** only when all success criteria and
      cross-stack guards have evidence. If an OS cannot meet the sandbox contract,
      leave the plan blocked with the exact missing primitive and operator action;
      do not claim multi-OS completion.

## Final acceptance matrix

The implementation is closure-ready only when every claimed platform has
positive and negative evidence for all rows below. A skipped row is an
unverified requirement, not a pass.

| Area | Positive evidence | Negative evidence |
| --- | --- | --- |
| User runtime | Conda/Anaconda, Node/npm/pnpm, Cargo/Rustup, Python env, and one OS-native package-manager path resolve from the filtered profile | Missing/malicious profile, world-writable path, symlink escape, and secret env value are rejected or omitted |
| Filesystem | Ordinary files in multiple `$HOME` subdirectories work when `dir=$HOME` | Outside execution root, unauthorized sibling under a narrow `dir`, protected path, junction/symlink escape fail closed |
| Credentials | `.env.example` remains readable as specified | All listed credential files, nested `.env.*`, sockets, Git URL userinfo, and helper forwarding are masked/redacted |
| Privilege | Normal user process/build/test works | Direct and shell-wrapped `sudo`, `su`, `doas`, `pkexec`, `runas`, setuid/helper and broker paths cannot elevate |
| SSH/sockets | Dedicated `ssh_readonly_exec` still works under its own policy | Generic SSH clients, Docker/Tailscale/D-Bus/keyring/agent sockets stay unavailable by default |
| Lifecycle | Short sync result, long async task, polling, cancellation, timeout, shutdown, and retry all return stable bounded state | Client timeout/disconnect never causes an untracked process or automatic duplicate execution |
| Resource controls | Per-owner/global concurrency, task TTL, poll rate, output bytes, process/FD, CPU, and memory limits are enforced and observable | Flooding starts/polls, fork bombs, descriptor exhaustion, memory/CPU exhaustion, and oversized output are bounded or fail closed |
| Context | Task ID/output/status survive stream abort and compaction through the existing persistence/handoff path | Cross-owner/session task get/cancel and stale post-restart IDs are denied without data disclosure |
| Routing/catalog | Active dedicated tools are preferred; terminal remains usable for native CLI workflows; Full/Primary sets equal the declared v15 sets | Disabled/removed tools are never recommended, callable, or silently substituted; v14 bytes remain unchanged |
| Platforms | Each claimed OS reports the reviewed sandbox backend and passes the matrix | Unsupported/missing primitive fails closed with a bounded reason; compile-only evidence is insufficient |

## Risks and rollback gates

### R-01 — Native OS sandbox cannot match Linux guarantees

Do not ship a raw-shell fallback. Keep the affected OS fail-closed and split a
follow-up backend plan if its credential, privilege, or process-tree proof is
incomplete.

### R-02 — Profile discovery becomes a hidden code-execution path

Run probes only inside the restricted read-only profile, import filtered PATH
and non-secret settings, bound time/output, and keep direct argv as the default.
Disable automatic profile probing if it can execute outside that profile.

### R-03 — HOME-scale masking is too slow or incomplete

Benchmark realistic trees and keep an explicit bounded scan. If the scan cannot
finish safely, refuse that root and recommend a narrower one; never skip unknown
trees merely to improve latency.

### R-04 — Sync timeout creates duplicate or unknown work

Start once, register idempotency before spawn, return a task envelope after the
bounded sync window, and retry at most once with the same key. Any ambiguous
outcome is surfaced as unknown and is never rerun automatically.

### R-05 — Catalog pruning breaks stored conversations or delivery workflows

Use an immutable v15 snapshot, filter historical IDs as unavailable, preserve
retained credential-isolated bridges, and keep local Git terminal fallback
available. Roll back only the catalog removal if migration tests fail.

### R-06 — Terminal Git leaks repository-stored credentials

Redact URL userinfo and helper output at task/activity/log boundaries, keep bridge
tokens out of terminal environments, and add a fixture containing an embedded
PAT. Do not claim credential safety based solely on `.git-credentials` masking.

### R-07 — Task ownership metadata is missing at a transport boundary

Require authenticated subject/session context for every get/cancel path and use
the same bounded not-found response for cross-owner probes. Do not expose task
existence through status, timing, or error differences.

### R-08 — OS primitive or protocol version is weaker than assumed

Pin each implementation to an explicitly negotiated MCP task wire version and
an observed OS primitive/version. If macOS entitlement signing, Windows
AppContainer/resource brokering, Linux namespace setup, or MCP extension
headers cannot be proven in the target build, mark that combination
unsupported and fail closed; do not silently fall back to a deprecated or
partial sandbox.

### R-09 — Resource exhaustion bypasses otherwise-correct isolation

Enforce admission, process/descriptor, CPU/memory, output, polling, and TTL
budgets before spawn and release them on every exit path. Add fork-bomb,
descriptor-flood, oversized-output, and concurrent-owner tests. A platform that
cannot enforce the required budget is not a supported broad-root profile.

### R-10 — Profile discovery races with executable replacement

Revalidate ownership, permissions, canonical path, and mount/reparse state at
the final spawn boundary, and resolve/execute by a stable descriptor where the
OS permits. If the platform cannot close the race, use static operator paths or
fail the profile probe closed.

## Explicit non-goals

- No raw host shell, removed Bubblewrap, root execution, or privilege escalation.
- No inherited arbitrary environment, credentials, agent/keyring sockets, Docker
  or Tailscale authority by default.
- No generic SSH replacement, universal `git` executable ban, or redundant tool
  discovery MCP call.
- No new sandbox framework or dependency unless the platform spike proves an
  existing OS primitive is insufficient and the dependency passes the normal
  lockfile, licensing, audit, and guard review.
- No production service restart/reload, deployment, release publication,
  force-push, or merge without a separate explicit request.
- No Plan-067-specific verification scripts and no unrelated cleanup.
