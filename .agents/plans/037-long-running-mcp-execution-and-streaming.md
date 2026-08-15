# Plan 037 — Long-Running MCP Execution, Streaming, and Task Lifecycle

**Status:** CLOSED / VERIFIED — server/runtime Phases 9–11 pass, and a fresh ChatGPT action snapshot now exposes the six-tool catalog with no client-side `terminal_exec.timeout_ms` maximum; live fallback polling/cancellation also passes.

## Goal

Make the Rust `ai-tools relay` suitable for real coding-agent workloads where commands may run for minutes or hours, while preserving the existing sandbox, OAuth, privacy, admission-control, and layered-architecture boundaries.

The finished system must support:

1. short commands with the current low-latency request/result experience;
2. long-running commands such as Rust/C/C++ builds, package installation, Docker/image work, integration checks, and large test/lint pipelines without an arbitrary five-minute server ceiling;
3. incremental stdout/stderr consumption instead of buffering all output until process exit;
4. explicit cancellation that kills the full process tree safely;
5. bounded memory even when tools produce very large output;
6. standards-based MCP task/progress behavior where the connected client supports it;
7. a clean fallback lifecycle for clients that do not support the current MCP Tasks extension;
8. a first-party Nuxt UX that can present live command progress like a coding tool/terminal without inventing fake percentage progress;
9. truthful behavior for ChatGPT and other third-party clients whose rendering/UI is controlled by the client rather than this repository;
10. a **home-scoped coding workspace** for the single-user laptop relay: the effective execution root should be the canonical non-root user home (for example `/home/user`), not one repository/project directory, so one connected MCP server can work across projects under that home without reconfiguration.

This plan is about **execution lifecycle and protocol ergonomics**, not about weakening command authorization, adding a remote shell bypass, or broadening the sandbox.

---

## Why this plan exists

The current relay works for short commands, but its execution contract is still synchronous and hard-bounded for workloads that are normal in coding environments.

### Confirmed current gaps

#### 1. Hard five-minute timeout ceiling

`packages/rust-tools/application/src/execution.rs` currently defines:

```text
MAX_TIMEOUT_MS = 300_000
```

and rejects `terminal_exec` / `http_fetch` calls above that value.

The public MCP tool schema in `packages/rust-tools/interfaces/src/mcp.rs` also publishes:

```json
"timeout_ms": {
  "minimum": 0,
  "maximum": 300000,
  "default": 30000
}
```

Therefore the five-minute limit is part of both server enforcement and the discovered client contract.

A coding MCP server must not assume every legitimate build finishes within five minutes.

#### 2. `timeout_ms = 0` has inconsistent semantics

The HTTP CLI documents `0 = no timeout`, while the terminal CLI feeds `Duration::from_millis(0)` into `tokio::time::timeout`, effectively making zero an immediate timeout.

Timeout semantics must become explicit, shared, and consistent within each tool class.

#### 3. Tool execution holds one MCP request open until the command exits

`handle_tools_call()` awaits `dispatch_tool_call()` directly before creating the JSON-RPC response.

A two-hour build therefore implies a two-hour HTTP request even if the internal command timeout is removed. That is fragile across clients, proxies, tunnels, browser/runtime request limits, and ordinary network reconnects.

#### 4. No incremental terminal output

The relay reads stdout/stderr using `read_to_end()`.

The inner `ai-tools terminal` CLI independently uses `wait_with_output()`.

The result is double buffering:

```text
child process
  -> terminal CLI buffers until exit
      -> relay buffers until exit
          -> MCP returns one final response
```

This is why long commands appear silent until they finish.

#### 5. Output limits are coupled to process termination

The relay currently caps captured stdout/stderr and may kill the process when the cap is exceeded.

For coding workloads, "the command produced too many logs" must not automatically mean "kill a valid compile/test job".

The safe policy is to keep draining the OS pipes while retaining only bounded output state for clients.

#### 6. Duplicate output policies exist

The terminal CLI truncates stdout/stderr around 20 KB, while the relay independently has a much larger output cap.

The process execution stack therefore has two unrelated truncation policies and two buffering layers.

#### 7. UTF-8 truncation is unsafe

The terminal helper slices strings by byte index (`&s[..limit]`). If the chosen byte index lands inside a multi-byte UTF-8 character, the CLI can panic.

#### 8. No durable execution/job lifecycle exists

The current application model is effectively:

```text
invoke command -> await process -> return string
```

There is no job ID, execution state, cancellation handle, event stream, retained tail buffer, or status query API.

#### 9. MCP Tasks capability is not advertised

The current server discovery capability only advertises tools. There is no task lifecycle capability and no `tasks/get`, `tasks/update`, or `tasks/cancel` path.

#### 10. Global execution concurrency is tied to request lifetime

The MCP transport currently acquires a global execution semaphore permit before dispatch and holds it until the synchronous command returns.

A task-based architecture must preserve execution concurrency limits, but permits must belong to **running jobs**, not to the HTTP request that happened to create the job.

#### 11. Release verification exposed the practical problem

The current release-preparation session successfully passed the mandatory repository gate, while longer Nuxt production/build steps repeatedly exceeded the practical single-call execution lifecycle. This is representative of real coding work and is not an exotic edge case.

#### 12. Current execution-root/operator guidance is too project-scoped

The relay containment code can validate a root as shallow as `/home/user`, but current operator guidance still presents `EXECUTION_ROOT=/home/<user>/<project>` and a deployed relay can therefore become effectively pinned to one project subtree. A separate chat/session then cannot move to sibling repositories such as `~/Projects/OtherProject` even though they belong to the same operator.

For this single-owner laptop coding relay, **the canonical user home is the intended coding workspace boundary**. The MCP connection must not need to be recreated or the relay restarted merely to switch between repositories under that home.

This change is not permission to escape into `/`, `/root`, another user's home, or arbitrary system paths. Home scope is the reviewed upper filesystem boundary for writable coding work.

#### 13. Home-installed developer toolchains are currently easy to make unavailable

The relay clears the environment and installs a hardcoded system-only `PATH`. On machines where Rust, Node, pnpm, or other developer tools live under user-managed locations such as `~/.cargo/bin` or a version manager under the home directory, valid coding commands can fail even though the binaries are inside the allowed home scope.

Plan 037 must preserve the safe-PATH principle while providing an **explicit, allowlisted user-toolchain path policy**. Do not solve this by blindly inheriting the relay process `PATH`.

---

## Non-goals

Plan 037 does **not** authorize:

- removing Bubblewrap containment;
- broadening writable coding scope **beyond the canonical non-root owner home**; moving from one-project scope to owner-home scope is an explicit goal of this plan;
- running the relay as root;
- exposing the Docker socket or privileged namespaces;
- bypassing destructive-action approval policy;
- weakening OAuth issuer/audience/owner/scope enforcement;
- logging command arguments, output, secrets, paths, or bearer tokens into telemetry;
- changing the repository's no-CI / no-unit-test policy;
- inventing a second proprietary remote-terminal transport when MCP can express the lifecycle;
- persisting full build logs indefinitely in Postgres or another database;
- claiming a third-party MCP host will visually render raw terminal chunks like a first-party coding terminal unless that behavior is proven against the current client.

---

## Architecture principles

### 1. Process execution and MCP request lifetime must be decoupled

Target model:

```text
MCP tools/call
    |
    +-- short/sync command --------------------------> final result
    |
    +-- long/task command
            |
            v
      Execution Job Manager
        |- sandboxed process
        |- stdout reader
        |- stderr reader
        |- timeout/deadline
        |- cancellation token
        |- process-group handle
        |- bounded output buffers
        |- state/result
            |
            +--> MCP Tasks adapter (supported clients)
            +--> fallback job tools (unsupported clients)
            +--> first-party Nuxt live UI
```

An HTTP request creates/observes a job. It does not need to stay open for the entire lifetime of the process.

### 2. One authoritative process runner

Do not create separate execution implementations for:

- sync terminal;
- async terminal;
- MCP Tasks;
- fallback job tools;
- Nuxt UI.

They must all use one underlying sandboxed process-execution lifecycle.

The current double execution/buffering path between relay dispatch and `ai-tools terminal` must be reviewed and simplified so timeout, cancellation, process-group handling, output draining, and truncation have one source of truth.

### 3. Preserve layered architecture

Before implementation, re-evaluate ownership across the Rust workspace rather than placing the entire job system in `transport.rs`.

Preferred responsibility split:

```text
core
  job IDs / states / bounded domain values where appropriate

application
  execution lifecycle policy / use cases / contracts

infrastructure
  Bubblewrap + tokio process runner
  concurrent stdout/stderr readers
  in-memory running-job registry
  process-group cancellation

interfaces
  MCP schemas / task DTOs / fallback tool contracts

cli
  composition / standalone command UX
```

Exact file placement may differ if current crate dependency direction requires it, but dependency inversion must remain valid and circular dependencies are forbidden.

### 4. Long-running does not mean unbounded resource use

A command may run for hours while still having bounded:

- concurrent process count;
- retained output bytes;
- pending event queue;
- request/body size;
- task count;
- task/job TTL after completion;
- telemetry cardinality.

### 5. Owner home is the coding workspace boundary

For the single-user laptop relay, distinguish these concepts explicitly:

```text
filesystem execution root = canonical owner home, e.g. /home/farismnrr
current working directory = any contained directory selected per command/job
project/repository         = discovered dynamically under the home; not a security boundary
```

Requirements:

- Default/recommended `EXECUTION_ROOT` for the operator deployment is the canonical non-root owner home.
- `cwd` may move between sibling repositories/directories anywhere under that home without relay restart or MCP reconnection.
- Relative `cwd` remains relative to the home execution root; absolute `cwd` is accepted only when canonicalized beneath it.
- Symlink/canonicalization checks must continue preventing traversal outside the home boundary.
- `/`, `/root`, other users' homes, and system-level roots remain forbidden as writable execution roots.
- The bwrap boundary must not accidentally expose additional host-home directories merely because the selected root is broader.
- Home scope does **not** imply that credential material should be casually exposed. Phase 0 must inventory sensitive owner-home paths and Phase 8 must define/re-prove a practical deny/mask policy for credentials that are not required for the requested command. Do not make a brittle blanket deny of useful developer state without auditing actual toolchain needs.
- Home-installed toolchains must be supported through reviewed allowlisted binary directories/mounts (for example a configured Rust/Cargo or Node version-manager bin path) while retaining environment clearing and avoiding arbitrary inherited `PATH`.

Do not create a concept of one permanently paired "project base" in the MCP protocol. Project detection is convenience/context; **home containment is the filesystem security boundary**.

The command duration and memory/output limits are separate concerns.

---

## Timeout contract

Plan 037 must deliberately replace the current hardcoded five-minute policy.

### Required semantics

For `terminal_exec`:

- `timeout_ms` remains optional;
- the default remains a normal short-command default unless a better reviewed value is chosen;
- `timeout_ms = 0` means **no command deadline**;
- positive values mean an explicit command deadline in milliseconds;
- the relay operator may configure a server-side maximum independently of the tool schema;
- an operator-configured maximum may itself support `0 = no maximum`;
- the schema must not falsely advertise a universal 300000 ms maximum when the runtime policy is configurable;
- timeout enforcement belongs to the job/process lifecycle and survives the transition from the initiating request to a task/job.

Suggested runtime settings to evaluate:

```text
--default-terminal-timeout-ms
--max-terminal-timeout-ms
--completed-job-ttl-ms
--max-retained-output-bytes
--max-running-jobs
```

Do not blindly add all flags if existing configuration conventions suggest a cleaner shape. Keep configuration cohesive.

### HTTP/search timeout policy is separate

Do **not** remove network-request safeguards merely because terminal builds need to run longer.

`http_fetch` and `web_search` should have their own reviewed timeout policies. A terminal command that may compile for hours is not equivalent to an HTTP request that may hang on a remote peer indefinitely.

---

## Output and streaming contract

### Continuous pipe draining

stdout and stderr must be consumed while the process is running.

Do not use `read_to_end()` or `wait_with_output()` as the long-running execution primitive.

Use asynchronous readers that continuously drain both streams.

### Bounded retention, not kill-on-log-volume

When retained output reaches its configured limit:

- keep the command running;
- keep draining stdout/stderr;
- discard/roll older retained data according to a bounded strategy;
- record how much data was omitted;
- expose a clear bounded marker to the client.

Example final/tail rendering:

```text
... 6.2 MiB earlier output omitted ...
Compiling relay-infrastructure v0.0.8
Finished release profile
```

Do not retain an unbounded `Vec<u8>` for an hours-long compile.

### Preserve stream identity

Internally retain whether a chunk came from stdout or stderr.

If exact interleaving cannot be guaranteed, use monotonically increasing event sequence numbers/timestamps and document the ordering contract instead of pretending two independently-read pipes have perfect ordering.

### UTF-8 safety

Chunking/truncation must never byte-slice a Rust UTF-8 `str` at an invalid character boundary.

Prefer byte-oriented buffering with safe lossy decoding at presentation boundaries, or another implementation that correctly handles multi-byte characters split across read chunks.

### Progress is not fake percentage

For commands such as `cargo build`, progress may be textual rather than numeric.

Valid UI status:

```text
Running · 12m 43s
Compiling tokio...
Compiling reqwest...
```

Do not invent `73%` when the underlying command does not provide meaningful percentage progress.

---

## Job lifecycle model

Implement one in-memory execution job abstraction with at least:

```text
job_id
state
created_at
started_at
finished_at
command deadline / timeout policy
process / process-group cancellation handle
exit code or terminal outcome
stdout/stderr retained tail/events
omitted/dropped byte count
final bounded result
```

Required states should map cleanly to the current MCP Tasks specification after the Phase 0 standards re-audit. At minimum the internal model must distinguish:

```text
queued / accepted
running
completed
failed
timed_out
cancelled
```

If the MCP extension uses a smaller/different public state vocabulary, adapt rather than leaking repository-specific state names into the standard contract.

### Job storage

Initial implementation should be in-memory unless Phase 0 proves persistence is required.

Requirements:

- relay restart may cancel/lose active jobs and must document that behavior;
- completed jobs expire after a bounded TTL;
- expired jobs are removed;
- task/job count is bounded;
- shutdown cancels/reaps active process groups safely;
- no full command output is persisted to database/log telemetry by default.

---

## MCP Tasks integration

### Phase 0 must re-audit the current official protocol

MCP evolves quickly. Before implementing Tasks, re-read the current official MCP specification/extensions for:

- task capability negotiation;
- task-returning `tools/call` results;
- task state/result semantics;
- polling/status fields;
- cancellation;
- progress/message mechanisms;
- task TTL/expiry;
- client capability advertisement;
- any current restrictions around stateless Streamable HTTP.

Also re-check current OpenAI/ChatGPT MCP documentation.

Do not copy protocol details from this plan if the current official specification has changed.

### Capability negotiation is mandatory

Do not send a task result to a client that did not advertise the required task capability.

The relay must advertise Tasks only when it actually implements the required lifecycle.

### Long-running tool behavior

Evaluate an explicit execution mode such as:

```text
execution_mode = auto | sync | task
```

or the current official MCP equivalent if the protocol already provides a better mechanism.

Desired behavior:

- `sync`: preserve simple request/result behavior for intentionally short commands;
- `task`: create a job immediately and return the standard task handle;
- `auto`: use a policy that does not make clients guess whether a potentially long command must finish inside one HTTP request.

Do not add an ambiguous `auto` mode without a deterministic policy.

---

## Fallback for clients without MCP Tasks

Because task-extension support may vary by MCP host/client, the same underlying job manager must be reachable through a small fallback tool surface when needed.

Candidate fallback tools:

```text
terminal_job_start
terminal_job_get
terminal_job_cancel
```

Optional output cursor/tail retrieval may be folded into `terminal_job_get` if that keeps the contract simpler.

Do not create six tiny tools when three cohesive operations are enough.

Fallback requirements:

- same sandbox as `terminal_exec`;
- same OAuth/owner scope;
- same execution concurrency policy;
- same timeout semantics;
- same cancellation semantics;
- same output retention;
- same telemetry privacy;
- no second process runner.

If current MCP client compatibility proves a different fallback design is materially better, document the evidence before changing the surface.

---

## Nuxt live execution UX

The first-party web application is the place where this repository can guarantee a coding-tool-quality display.

Design a reusable command/task presentation rather than dumping an ever-growing `<pre>`.

Desired UX:

```text
[ Running ]  cargo build --release                 12m 43s

Compiling tokio v...
Compiling reqwest v...
Compiling relay-infrastructure v0.0.8
...

6.2 MiB earlier output omitted

[ Cancel ]
```

Requirements:

- live appended output/status while a job runs;
- stdout/stderr visually distinguishable when useful without noisy decoration;
- elapsed duration;
- clear running/completed/failed/timed-out/cancelled state;
- cancel action for a running job;
- bounded DOM/rendered history so the browser does not freeze on huge logs;
- auto-scroll only when the user is already near the bottom; do not fight manual scrolling;
- preserve existing approval semantics for destructive actions;
- reconnect/poll should recover current job state while the job is still retained;
- accessibility and keyboard usability remain acceptable.

Do not require the Nuxt page to keep the original `tools/call` HTTP request open for hours.

---

## ChatGPT / third-party client behavior

The relay can control protocol semantics, task state, progress data, and tool results.

It cannot control whether ChatGPT's native tool card chooses to render every raw stdout chunk like a terminal.

Therefore acceptance must distinguish:

### Server-side proof

- task capability negotiation works;
- long command can continue after the initiating request returns;
- task state can be queried;
- cancellation works;
- progress/status data is emitted according to the current MCP contract;
- final bounded output is retrievable.

### ChatGPT UI proof

Record only what the current ChatGPT client actually displays.

Do not mark "Codex-like streaming UI" complete merely because the MCP server internally emits progress.

---

## Execution concurrency and admission

The existing global execution semaphore is a security/resource boundary and must not disappear.

Refactor ownership so that:

- the permit is associated with the **running job/process**;
- the initiating HTTP request may return while the job retains its permit;
- the permit is released exactly once when the job completes/fails/times out/cancels;
- queued/accepted jobs cannot bypass the configured maximum running-process count;
- cancellation and relay shutdown cannot leak permits;
- request admission remains separate from running-job concurrency.

Consider making the current hardcoded execution concurrency (`16`) configurable if that can be done cleanly without expanding scope unnecessarily.

---

## Cancellation and process cleanup

Cancellation is a first-class behavior, not an error-path afterthought.

Required proof:

- cancelling a job kills the full Unix process group;
- descendants do not survive;
- process leader is reaped;
- repeated cancel requests are idempotent or safely rejected according to the public contract;
- cancelling an already-completed task does not kill an unrelated reused PID;
- relay shutdown cleans active jobs;
- timeout and manual cancellation use the same authoritative cleanup primitive;
- Bubblewrap `--die-with-parent` remains an additional safety layer rather than the only cleanup mechanism.

---

## Error/result semantics

Keep public diagnostics bounded and generic where confidentiality requires it.

Clients may receive useful lifecycle state such as:

```text
failed
timed out after configured deadline
cancelled
output omitted due to retention limit
```

but must not receive raw internal errors containing:

- filesystem paths that are supposed to remain private;
- OAuth/JWKS/provider internals;
- secret environment values;
- bearer tokens;
- arbitrary raw process diagnostics when the existing security contract requires generic output.

Command stdout/stderr intentionally returned as the tool result is distinct from telemetry. Preserve the existing rule that command output must not leak into Loki/Jaeger/span attributes.

---

## Phased execution

### Phase 0 — Re-audit standards and current implementation

- [x] Re-read `AGENTS.md`, relevant `.agents/knowledge/*`, canonical memory, Plan 036, and this Plan 037.
- [x] Re-audit current MCP Tasks/progress/cancellation specification from official sources.
- [x] Re-audit current OpenAI/ChatGPT MCP capability behavior from official OpenAI sources.
- [x] Map current execution flow from MCP schema -> transport -> application dispatch -> Bubblewrap -> `ai-tools terminal` -> child process.
- [x] Confirm all current timeout/output/concurrency limits and document which are security boundaries vs accidental implementation limits.
- [x] Freeze the backward-compatible tool contract before edits.
- [x] Re-audit current execution-root/operator behavior and record that the target single-owner boundary is the canonical non-root owner home rather than one project directory.
- [x] Inventory home-resident developer toolchains and sensitive credential/state paths before changing bwrap mounts/PATH policy.

Acceptance:

- a short source/contract note records the exact current standard targeted and any difference from assumptions in this plan;
- no implementation begins from stale MCP task semantics.

### Phase 1 — Define timeout and job contracts

- [x] Define explicit terminal timeout semantics, including `0`.
- [x] Separate terminal duration policy from HTTP/search timeout policy.
- [x] Define job states and lifecycle.
- [x] Define output retention policy.
- [x] Define cancellation semantics.
- [x] Define execution concurrency ownership.
- [x] Decide configurable server limits and defaults without arbitrary five-minute assumptions.
- [x] Define one authoritative home-scoped workspace contract: canonical owner home as execution root, per-job `cwd`, sibling-project switching, and no project-base pinning.
- [x] Define explicit allowlisted user-toolchain directories without inheriting arbitrary parent `PATH`.

Acceptance:

- one coherent contract exists before process-runner refactor;
- JSON schema, runtime config, and CLI semantics can all implement the same rules.

### Phase 2 — Refactor to one streaming-safe process runner

- [x] Remove duplicate relay/terminal timeout and retention policy.
- [x] Replace relay `read_to_end()` / terminal `wait_with_output()` execution semantics with continuously drained async stdout/stderr.
- [x] Centralize process-group spawn/kill/reap behavior.
- [x] Implement UTF-8-safe output presentation.
- [x] Ensure output overflow drops retained data rather than killing the command.
- [x] Preserve Bubblewrap containment, env clearing, safe PATH, execution-root checks, and command validation.
- [x] Make home-installed toolchains usable through reviewed allowlisted paths/mounts while keeping the process environment fail-closed.

Acceptance:

- a noisy command can emit well beyond the retained-output limit and still finish successfully;
- retained memory stays bounded;
- multi-byte UTF-8 output cannot panic truncation;
- timeout/cancel kills descendants correctly.

### Phase 3 — Introduce Execution Job Manager

- [x] Add job IDs and lifecycle state.
- [x] Hold process/concurrency permit for running-job lifetime.
- [x] Maintain bounded output/event state.
- [x] Support query, cancel, completion, timeout, and expiry.
- [x] Bound running + retained completed job counts.
- [x] Add shutdown cancellation hook to the job manager; live deployment review remains deferred.

Acceptance:

- a job keeps running after its creating handler returns;
- another request can query its state;
- cancellation and expiry do not leak processes or semaphore permits.

### Phase 4 — Preserve and improve synchronous `terminal_exec`

- [x] Keep short-command synchronous behavior backward compatible.
- [x] Remove the fixed 300000 ms terminal schema/runtime ceiling.
- [x] Make `timeout_ms = 0` behavior explicit and correct.
- [x] Reuse the same job/process runner rather than old duplicate logic.
- [x] Keep final sync output bounded.

Acceptance:

- existing short ChatGPT `terminal_exec` calls continue working;
- a normal 30-second/default call behaves as before from the client's perspective;
- configured long timeout is accepted according to server policy.

### Phase 5 — MCP Tasks integration

- [x] Advertise current task capability correctly.
- [x] Implement the current standard task-returning tools/call behavior.
- [x] Implement required task status/result operations.
- [x] Implement standard cancellation.
- [x] Expose status/progress without command/output leakage into telemetry.
- [x] Reject/mask unsupported task behavior for clients that did not negotiate it.

Acceptance:

- a client that supports Tasks can start a long command without holding one HTTP request for the command lifetime;
- it can query status, cancel, and retrieve the final result;
- client capability negotiation is enforced.

### Phase 6 — Compatibility fallback for non-Task clients

- [x] Add the smallest reviewed fallback job tool surface if still necessary after Phase 0.
- [x] Reuse the exact same Job Manager.
- [x] Keep auth/sandbox/concurrency behavior identical to standard task mode.
- [x] Document which clients need the fallback.

Acceptance:

- clients without MCP Tasks can still safely run >5-minute work using multiple bounded MCP calls;
- no duplicated process runner exists.

### Phase 7 — Nuxt live coding-tool UX

- [x] Wire first-party UI to task/job state.
- [x] Render live/tail output with bounded browser memory.
- [x] Add running state and cancellation.
- [x] Handle reconnect/polling.
- [x] Preserve approval UX for destructive commands; the first-party relay page only adds lifecycle polling and does not bypass chat approval handling.
- [x] Avoid fake progress percentages; the UI shows queued/running/terminal state and raw retained output.

Acceptance:

- a multi-minute command visibly updates while running;
- user can scroll old retained output without auto-scroll fighting them;
- cancel updates UI and kills the underlying process tree;
- huge log volume does not freeze the browser.

### Phase 8 — Build, install, and restart the operator relay (implementation handoff)

This is the **executor stop point** for the first implementation pass. The implementation agent must build and redeploy the operator relay, but must not continue into the functional/security/long-duration acceptance phases below. Those tests are intentionally reserved for a separate reviewer session.

Required executor actions:

- [x] Run the mandatory repository commit gate required by repo policy: `pnpm verify:commit`. This is still mandatory even though the later acceptance/test matrix is deferred.
- [x] Run a production web build: `pnpm build`.
- [x] Build the native Rust tools in release mode: `pnpm build:tools`.
- [x] Confirm the produced `ai-tools` binary reports the intended current version (`0.0.8`); do not run the full MCP acceptance suite.
- [x] Install/update the reviewed release binary used by the operator relay, preserving the existing ownership and executable permissions.
- [x] Update the existing operator relay configuration so the effective `EXECUTION_ROOT` is the canonical non-root owner home (`$HOME`, `/home/farismnrr`), not the `ai-code` repository or another single project directory.
- [x] Keep the relay working directory independently configurable; it remains `/home/farismnrr/Projects/MasihAwam/ai-code` while the filesystem scope is `/home/farismnrr`.
- [x] Preserve existing OAuth issuer/audience/owner values and other protected runtime secrets; changing execution scope did not require regenerating or exposing credentials.
- [x] Preserve the existing loopback-only listener, Cloudflare/outbound-tunnel topology, non-root service account, and Bubblewrap requirement.
- [x] Reload/restart the existing persistent operator relay service (`ai-tools-relay.service`) so the newly built binary and home-scoped execution root are active.
- [x] Perform **deployment-status checks only**: the service is active/running with `NRestarts=0` and no MCP tools or acceptance scenarios were invoked in this phase.
- [x] Record the exact build/install/restart commands and resulting deployed binary/version/config shape without recording secrets.

Executor handoff state:

- implementation is built and running with owner-home scope;
- no functional/security/long-duration acceptance claim has been made yet;
- Plan 037 remains **OPEN / READY FOR REVIEW**, not CLOSED;
- Phases 9–11 remain unchecked for the separate review session.

Executor evidence (2026-08-16): `CI=true pnpm verify:commit`; `CI=true pnpm build`;
`CI=true pnpm build:tools`; `target/release/ai-tools --version` → `ai-tools
0.0.8`; `install -m 755 target/release/ai-tools
/home/farismnrr/.local/bin/ai-tools`; `systemctl --user daemon-reload`;
`systemctl --user restart ai-tools-relay.service`; status checks reported
`ActiveState=active`, `SubState=running`, `NRestarts=0`, and
`EXECUTION_ROOT=/home/farismnrr`. No functional, security, or long-duration
acceptance claim is made here.

## Reviewer round 1 findings (2026-08-16)

The first reviewer pass intentionally did not modify production code. It re-ran the mandatory gate/builds and exercised the deployed/home-scoped execution model plus a temporary local relay using the installed `ai-tools 0.0.8` binary. Plan 037 remains open because the following blockers are reproducible.

### Blocking findings

1. **Target command exit status is not propagated correctly.** `ai-tools terminal` prints `Exit: <target>` but exits zero unless its own output begins with `Error:`. A live `sh -c 'exit 7'` call therefore surfaced outer `Exit: 0` with inner `Stdout: Exit: 7`. The authoritative runner must receive the real target exit status so failed builds/tests cannot be reported as successful.
2. **The MCP Tasks wire contract is not conformant to the current `io.modelcontextprotocol/tasks` extension.** Current `tasks/get` lacks `resultType: "complete"`; task timestamps are epoch-millisecond strings instead of ISO-8601; `tasks/update` and `tasks/cancel` return full task objects instead of empty acknowledgements; and task/tool failure state mapping must distinguish a completed tool result with `isError: true` from a JSON-RPC-level task failure. Re-audit TTL semantics at the same time: the current completed-job retention duration is advertised as task lifetime from creation, which is unsuitable for genuinely long-running work.
3. **Polling does not expose output while a job is running.** Pipes are drained continuously into bounded buffers, but those buffers are copied into the public `JobSnapshot` only at `finish()`. Runtime proof with `printf first; sleep 1; printf second` returned `status=working` with empty stdout, then exposed all output only after completion. The first-party polling UI therefore is not yet a live coding-terminal UX.
4. **The deployed safe toolchain allowlist is incomplete.** Normal non-login commands receive only the fixed system PATH. `RELAY_TOOLCHAIN_PATH` is unset in the operator profile, so the owner-home relay cannot find the installed Cargo, Node, or pnpm toolchains through the reviewed allowlist. Do not use login-shell startup files or arbitrary inherited host PATH as the fix; configure explicit canonical owner-home toolchain directories.
5. **The current ChatGPT-side action catalog is stale relative to the rebuilt server.** The live server/local `tools/list` exposes the fallback job tools and the new terminal timeout schema, while the already-connected app snapshot in this chat still exposes the earlier three-tool catalog and five-minute client-side maximum. After the next rebuild/restart, refresh/rescan the app actions and validate from a fresh chat before making a ChatGPT compatibility claim.
6. **The explicit Job Manager shutdown hook is not wired.** `JobManager::shutdown()` exists, but the relay graceful-shutdown path does not call it. Runtime descendant cleanup still passed because the current process/Bubblewrap lifecycle killed descendants, but the source-level Plan 037 shutdown contract must have one explicit authoritative cleanup path rather than relying on `--die-with-parent` alone.

### Reviewer evidence that already passed

- owner-home scope: `cwd=/home/farismnrr/Projects/Sensio` works without relay reconfiguration; direct `/tmp` and a symlink escape from `$HOME` to `/tmp` are rejected;
- owner identity is non-root (`uid=1000`); tested `.ssh` and `.docker` credential directories are masked inside the sandbox;
- `timeout_ms=0` completes without an accidental immediate timeout; a raw local fallback job accepted `timeout_ms=600000`;
- manual cancellation, timeout, and relay shutdown prevented a delayed descendant marker from surviving;
- bounded noisy-output retention completed successfully and reported omitted bytes instead of killing the process;
- bounded running-job admission/queue behavior was exercised with a one-job limit;
- public unauthenticated `server/discover` still returns an OAuth Bearer challenge;
- `cargo audit` passed;
- reviewer re-ran `CI=true pnpm verify:commit`, `CI=true pnpm build:tools`, and `CI=true pnpm build`; all completed successfully.

### Remediation handoff

The implementation executor should fix every blocking finding above, re-run the mandatory gate/build/install/restart sequence from Phase 8, keep the owner-home scope, and stop again at the Phase 8 handoff. **Do not mark Phases 9–11 complete and do not close Plan 037.** A separate reviewer will re-run acceptance after the remediated binary is deployed.

## Reviewer round 2 pre-restart evidence (2026-08-16)

The reviewer fixed the round-1 blockers and validated the rebuilt `ai-tools 0.0.8` binary before replacing the running operator service. This evidence is against the staged/final binary and temporary owner-home relays; Plan 037 remains open until the persistent `ai-tools-relay.service` is restarted onto that binary and re-checked live.

Passed before the final restart:

- real target exit status propagation (`exit 7` remains a tool-level error with the real exit code);
- MCP Tasks `io.modelcontextprotocol/tasks` wire contract: `resultType`, RFC3339 timestamps, completed tool errors, update/cancel acknowledgements, and long-running TTL semantics;
- first-party fallback job polling exposes bounded live stdout while work is still running; standard MCP Tasks stay protocol-clean rather than carrying non-standard raw-stream fields;
- owner-home containment, sibling-project cwd, outside-home/symlink rejection, non-root execution, credential masking, safe Cargo/Node/pnpm allowlist, timeout=0, explicit long timeout, bounded noisy output, UTF-8 safety, cancel/timeout/shutdown process-tree cleanup;
- completed-job eviction under 70+ sequential jobs, preventing the round-1 registry saturation defect;
- explicit `JobManager::shutdown()` wiring;
- frozen six-tool catalog + Phase 7 contract;
- `scripts/phase4-black-box.sh`;
- `cargo audit`;
- `CI=true pnpm verify:commit`;
- `CI=true pnpm build`;
- `CI=true pnpm build:tools`.

Deployment staging completed without weakening the relay self-update boundary. Because the active relay read-only-binds its own executable directory, the replacement binary is staged at `/home/farismnrr/.local/share/ai-code/bin/ai-tools` and the operator env now selects it with `AI_TOOLS_BIN`; `RELAY_TOOLCHAIN_PATH` explicitly allowlists `/home/farismnrr/.cargo/bin` and the default fnm Node bin. `EXECUTION_ROOT` remains `/home/farismnrr`. No secret values were changed or recorded.

Remaining closure step: restart the persistent user service, then re-prove live deployed version/config, sibling-project toolchain visibility, non-zero exit propagation, long/fallback job lifecycle, and service health before checking Phases 9–11 complete.

## Reviewer round 2 post-restart evidence (2026-08-16)

The persistent relay was restarted onto the reviewed staged `ai-tools 0.0.8` binary/config and re-proved live through the authenticated ChatGPT connector. Live deployed results: sibling-project `cwd=/home/farismnrr/Projects/Sensio`; UID 1000; Cargo, Node, and pnpm resolve only from the fixed system PATH plus the explicit reviewed Cargo/fnm allowlist; `.ssh` and `.docker` are masked; `/tmp` is rejected outside the canonical home execution root; `timeout_ms=0` completes normally; and `sh -c 'exit 7'` is surfaced as an MCP tool error with the real exit code 7. The staged operator binary hash matches `target/release/ai-tools`.

Final-state deterministic evidence also passes: `CI=true pnpm verify:commit`, `scripts/phase4-black-box.sh`, `scripts/phase7-chatgpt-contract.sh`, `scripts/verify-rust-phase3-telemetry.sh`, and `cargo audit`. The telemetry harness was reconciled to the repository's pinned Node 24 native TypeScript support because the obsolete `tsx` executable is no longer a dependency; the acceptance result confirms first-party Rust spans export while dependency noise, filesystem paths, and the canary remain absent.

The only remaining closure blocker was external/client-side: the already-open reviewer ChatGPT connection still exposed the pre-refresh frozen three-tool catalog and a client-side `terminal_exec.timeout_ms` maximum of 300000, while the reviewed server/frozen repository catalog contained six tools and no terminal maximum. This was not a relay regression.

### Fresh ChatGPT client closeout evidence (2026-08-16)

A fresh ChatGPT conversation loaded the refreshed Masih Awam MCP action snapshot and resolved the final external compatibility blocker:

- all six expected actions are present and callable: `terminal_exec`, `http_fetch`, `web_search`, `terminal_job_start`, `terminal_job_get`, and `terminal_job_cancel`;
- the refreshed `terminal_exec.timeout_ms` client schema has `minimum: 0` and no fixed `maximum: 300000`; a live `terminal_exec` call with `timeout_ms=600000` completed successfully in the Masih Awam repository;
- fallback job execution with `timeout_ms=600000` completed successfully through `terminal_job_start` + `terminal_job_get`;
- live fallback polling exposed `stdout=live-before-sleep\n` while the job remained `status=working`;
- `terminal_job_cancel` followed by `terminal_job_get` reached `status=cancelled`, retained the already-produced output, and did not emit the delayed post-sleep marker.

This fresh-client proof reconciles the deployed server, frozen repository tool catalog, and ChatGPT-visible action contract. No Plan 037 closure blocker remains.

### Phase 9 — Security and observability review (deferred; do not execute in implementation pass)

The first implementation executor must leave this entire phase unchecked. A separate reviewer performs it after the rebuilt home-scoped relay is running.

- [x] Re-prove Bubblewrap and owner-home execution-root containment, including sibling-project `cwd` changes and rejection of paths outside the canonical home.
- [x] Re-prove sensitive home credential paths follow the reviewed deny/mask policy and are not broadened accidentally by the home-scoped bind.
- [x] Re-prove configured home toolchain paths work without arbitrary parent-PATH inheritance.
- [x] Re-prove non-root requirement.
- [x] Re-prove OAuth resource/owner/scope behavior.
- [x] Re-prove request admission and execution concurrency.
- [x] Re-prove no command arguments/output in logs/traces.
- [x] Re-prove bounded public errors.
- [x] Re-prove cancellation/timeout do not orphan processes.
- [x] Re-prove output overflow cannot cause unbounded memory or pipe deadlock.

Acceptance:

- zero intentional security-boundary weakening relative to Plans 031B/035/036.

### Phase 10 — Long-duration acceptance matrix (deferred; reviewer-owned)

Run explicit deterministic/manual acceptance without introducing a general unit-test suite.

Required scenarios:

| Scenario | Expected |
| --- | --- |
| short command | sync result remains fast/backward compatible |
| sibling projects under owner home | command can change `cwd` between them without relay restart/reconnect |
| path outside owner home | rejected after canonicalization |
| home-installed allowlisted toolchain | executable is available without inheriting arbitrary host `PATH` |
| protected credential path | follows reviewed deny/mask policy rather than becoming implicitly exposed |
| explicit 10+ minute timeout | accepted when server policy allows it |
| `timeout_ms = 0` | no command deadline, with documented lifecycle |
| configured maximum exceeded | bounded validation failure |
| long silent command | remains queryable/cancellable |
| long noisy command | remains alive; retained output bounded |
| stdout + stderr | both drained and represented safely |
| output > retention cap | earlier output omitted, process not killed |
| UTF-8 across chunk boundary | no panic/corruption bug |
| manual cancel | full process group terminated/reaped |
| timeout | full process group terminated/reaped |
| relay shutdown | active jobs cleaned |
| 17th concurrent job with current 16 limit | bounded queue/rejection according to chosen contract |
| Tasks-capable MCP client | standard task lifecycle works |
| non-Tasks client | sync/fallback path works |
| ChatGPT | record actual discovered/visible behavior truthfully |
| Nuxt | live output + cancel UX works |

For true "hours" behavior, it is acceptable to use a controlled long-running fixture (e.g. sleep/periodic output) rather than wasting hours compiling, provided process lifetime/cancellation/output semantics are genuinely exercised.

### Phase 11 — Final reviewer verification and closeout

This phase is also **deferred** to the separate review session. The implementation executor stops after Phase 8.

The reviewer must reconcile the build evidence already produced in Phase 8 and then run the security/acceptance verification needed for truthful closure, including `cargo audit`, relevant deterministic current security/protocol scripts, and `pnpm audit` if dependencies changed. Re-run `pnpm verify:commit`, `pnpm build`, or `pnpm build:tools` only when the reviewer needs fresh final-state evidence or the code changed after the executor handoff.

Do not introduce GitHub Actions or a unit-test suite to satisfy this plan.

Reviewer closeout checklist:

- [x] Fresh review for process leaks, races, semaphore leaks, memory growth, stale task handles, and cancellation races.
- [x] Fresh review for MCP protocol compatibility.
- [x] Fresh review for telemetry/output secrecy.
- [x] Update README/operator docs and package skill docs.
- [x] Update canonical memory with durable timeout/task/output decisions.
- [x] Update Plan 036 only where Plan 037 materially changes its remote-MCP operational contract.
- [x] Mark Plan 037 closed only after current implementation and evidence agree.

---

## Definition of Done

Plan 037 is complete only when all of the following are true:

1. `terminal_exec` no longer has an unconditional five-minute ceiling baked into both schema and runtime.
2. Terminal timeout semantics are explicit; `0` is not accidentally immediate timeout.
3. Long commands do not require one HTTP request to remain open for their entire lifetime when task/job mode is used.
4. stdout and stderr are drained incrementally while the process runs.
5. Output retention is bounded without killing valid noisy commands solely because logs exceeded the retained buffer.
6. UTF-8 output truncation/chunking is safe.
7. One authoritative job/process runner owns spawn, containment, timeout, cancellation, output, and cleanup behavior.
8. A Job Manager exposes bounded state/query/cancel semantics.
9. Running-job concurrency remains bounded and semaphore permits cannot leak.
10. Current MCP Tasks semantics are implemented for clients that negotiate them, or the plan documents why the current official standard cannot be used.
11. Non-Task clients retain a safe usable path without a duplicate runner.
12. Nuxt can show live command state/output and cancel long jobs without unbounded browser memory.
13. ChatGPT behavior is documented based on real current-client evidence, without promising UI rendering the server cannot control.
14. Bubblewrap, execution-root, non-root, OAuth, admission, process cleanup, and telemetry confidentiality remain intact.
15. The implementation executor successfully builds and restarts the real operator relay with the new binary and canonical owner-home execution root before handing the work to review.
16. Mandatory repository/build/security verification passes on the final reviewed implementation state.
17. The deployed single-owner relay uses the canonical non-root owner home as its coding execution root, can switch among sibling projects without reconfiguration, and still rejects traversal outside that home.
18. Home-installed developer toolchains work through an explicit reviewed allowlist rather than arbitrary inherited `PATH`, and the broader home scope has a reviewed credential-path exposure policy.
19. Plan 037 is not marked CLOSED by the implementation executor; closure occurs only after the separate reviewer completes the deferred security and acceptance phases.

---

## Git / execution workflow

This plan file is documentation only. Implementation must follow `.agents/knowledge/git.md`.

When execution starts:

1. do **not** implement Plan 037 on top of an unrelated dirty release-preparation working tree;
2. reconcile/finish the prior work first;
3. start from current `dev`;
4. create a short-lived implementation branch, recommended:

```text
feat/037-long-running-mcp-execution
```

5. implement Phases 0–8 in order;
6. pass the mandatory local gate before every normal implementation commit;
7. build, install, and restart the actual operator relay with `EXECUTION_ROOT=$HOME` as required by Phase 8;
8. stop after deployment-status checks and hand off with Phases 9–11 still unchecked; do **not** run the deferred functional/security/long-duration acceptance matrix and do **not** close Plan 037;
9. PR implementation to `dev` when appropriate under repo rules;
10. do not promote to `main` unless explicitly requested.
