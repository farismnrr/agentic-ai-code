# Plan 061 — Read-Only SSH Execution

**Status:** PLANNED / NOT IMPLEMENTED
**Goal:** Add industrial-grade SSH capability to the Rust relay so an AI can use operator-owned OpenSSH host aliases and key-based authentication for bounded remote inspection while being technically prevented from password guessing, interactive authentication, interactive shells, forwarding, and remote mutation.
**Success Criteria:** `ssh <configured-alias> <approved-read-only-command...>` works through the relay when the host can authenticate non-interactively with an existing key; any password/passphrase/keyboard-interactive requirement fails immediately without retries; interactive SSH sessions are rejected; raw operator SSH config is parsed into a relay-owned sanitized connection specification rather than executed as an unrestricted OpenSSH config surface; every remote operation must satisfy integrity, confidentiality, and availability policy; Docker and nested `docker exec` operations are semantically validated; database inspection requires a dedicated least-privilege read-only database identity plus engine-level read-only enforcement and query policy; SSH credentials are exposed only to the SSH-specific sandbox profile; dangerous OpenSSH directives/capabilities fail closed; focused adversarial tests and the Rust guardrail pass; no systemd relay restart/reload and no Git push occur during implementation unless separately authorized later.

## Scope

### In scope

- Rust relay SSH capability using the operator's existing OpenSSH configuration and file-based SSH keys.
- Host aliases such as `smart-meeting` resolved from the operator-owned SSH configuration.
- Non-interactive, key-only authentication.
- A dedicated SSH security boundary integrated with terminal execution so the user-facing invocation can remain familiar (`command="ssh"`, `args=[...]`).
- Server-enforced read-only remote command policy.
- Read-only access to the minimum SSH configuration/key material required by the SSH child process only.
- Bounded output, timeout, cancellation, activity classification/redaction, and existing job lifecycle behavior.
- Documentation, focused Rust tests, and repository plan/memory synchronization.

### Out of scope

- Password guessing, password prompts, passphrase prompts, keyboard-interactive authentication, challenge-response authentication, or automatic credential discovery.
- Reading, printing, returning, copying, or otherwise exposing private key contents to the model/client.
- Interactive remote shells (`ssh host` with no remote command), PTY allocation, SCP/SFTP, rsync-over-SSH, SSHFS, or arbitrary shell access.
- Any remote mutation, including file writes/deletes, package/service changes, process signaling, Git mutation, container/orchestrator mutation, database mutation, or privilege escalation.
- Agent forwarding, X11 forwarding, local/remote/dynamic port forwarding, Unix-socket forwarding, `LocalCommand`, arbitrary `ProxyCommand`, PKCS#11/security-key providers, or host-side arbitrary command execution sourced from SSH config.
- Weakening the repository-wide `.ssh` protected-path policy for ordinary workspace/file/terminal tools.
- Any `systemctl`/systemd relay restart, reload, stop, start, service mutation, or deployed relay binary replacement.
- Git push, pull request creation, merge, release, or deployment.

## Current State

Verified before plan creation on 2026-08-30:

- Base `main` is clean and synchronized at `addf925`; task branch is `feat/061-read-only-ssh`.
- Ordinary `terminal_exec` resolves an executable from the relay safe PATH, runs inside Bubblewrap, clears the environment, and only receives network access when `RELAY_ALLOW_TERMINAL_NETWORK=true`.
- Ordinary terminal execution currently receives writable workspace authority and never receives the operator's whole HOME.
- `.ssh` is deliberately a globally protected credential path and is masked/rejected by normal workspace/sandbox operations.
- The current terminal executable policy blocks privilege-escalation binaries and conditionally blocks Docker, but has no SSH-specific argument/config/remote-command policy.
- There is no SSH-specific MCP tool, sandbox profile, config validator, or test suite.
- Current job execution already provides bounded retained output, timeout, cancellation, process-group cleanup, redaction, and activity plumbing that SSH should reuse rather than duplicate.

## Constraints & Decisions

- **Fail closed on authentication:** invoke OpenSSH with `BatchMode=yes`, password and keyboard-interactive authentication disabled, zero password prompts, and no interactive stdin. A host that cannot authenticate immediately using an already-usable key must stop with a bounded `interactive_auth_required`/authentication failure; the relay must never try candidate passwords, inspect unrelated credential stores, or retry alternate interactive methods.
- **No interactive shell:** bare `ssh alias` is rejected before spawn. Every SSH execution must contain one server-validated remote command.
- **Read-only is a technical policy, not an instruction to the model:** arbitrary remote shell text is unsafe because OpenSSH passes remote commands through the remote user's shell. The relay must not rely on natural-language instructions, a denylist of obvious mutators, or shell-metacharacter filtering alone.
- **Docker-first semantic policy:** optimize the reviewed command registry for container-host diagnostics. Direct host commands remain a narrow observational subset, while Docker commands receive subcommand/argument validation and `docker exec` recursively validates the nested container command. Database and observability access are treated as protocol-specific read-only capabilities, not generic shell permission. Unknown command families fail closed.
- **Validated composition, not arbitrary shell:** allow useful diagnostic composition such as pipelines and conditional chaining only through a real parser/AST where every node is independently classified read-only. Read-only transforms such as `grep`, `head`, `tail`, and bounded `awk`/`sed` subsets may participate. File redirection, write-mode `tee`, background jobs, command substitution that escapes classification, `sh -c`/`bash -c`, unrestricted interpreters, mutating `find`, editor/pager escapes, and any unknown AST node are denied. The relay renders only the already-validated AST into the remote command.
- **SSH config is untrusted capability input, not an execution contract:** the relay may read the operator's existing config to resolve aliases, but must parse only a reviewed safe subset into a normalized connection specification. Do not hand the raw config to OpenSSH as an unrestricted capability surface. Reject executable/dynamic directives including `ProxyCommand`, `LocalCommand`, `KnownHostsCommand`, `Match exec`, `RemoteCommand`, agent/X11/port/socket forwarding, PKCS#11/security-key provider execution, environment-driven executable paths, unsupported `Include` targets, and any directive that widens local execution/network/credential authority. `ProxyJump` is unsupported initially unless a later reviewed design proves equivalent policy on every hop.
- **Server-owned OpenSSH overrides:** force non-interactive key-only behavior and disable connection reuse/agent side channels with reviewed options including `BatchMode=yes`, `PasswordAuthentication=no`, `KbdInteractiveAuthentication=no`, `PreferredAuthentications=publickey`, `NumberOfPasswordPrompts=0`, `IdentitiesOnly=yes`, `IdentityAgent=none`, `ClearAllForwardings=yes`, `ForwardAgent=no`, `ForwardX11=no`, `PermitLocalCommand=no`, `ControlMaster=no`, `ControlPersist=no`, `RequestTTY=no`, `StdinNull=yes`, `EscapeChar=none`, strict host-key verification, and a single bounded connection attempt. User input may not override these controls.
- **Minimum credential mount:** ordinary terminal/workspace tools continue to see `.ssh` as protected. Only the SSH profile may receive read-only access to the exact reviewed identity/known-host material referenced by the normalized connection specification. Do not bind the entire HOME or the raw SSH directory by default. Symlink targets must be canonicalized and constrained to the approved SSH credential root; unexpected external includes/identity paths fail closed.
- **Host-key verification stays strict:** do not use `StrictHostKeyChecking=no`, `UserKnownHostsFile=/dev/null`, or automatic trust-on-first-use. Existing trusted known-host state may be used read-only; unknown/changed host keys fail.
- **No privilege escalation remotely:** reject `sudo`, `su`, `doas`, `pkexec`, shells/interpreters capable of arbitrary execution, and command families that can transition into a mutating mode.
- **Network authority remains explicit:** SSH requires network. Prefer an SSH-specific network-enabled invocation path rather than turning all terminal subprocesses network-enabled. Do not make `RELAY_ALLOW_TERMINAL_NETWORK=true` a prerequisite if a narrower SSH capability flag/profile can provide least privilege.
- **Three-dimensional diagnostic safety:** a command is eligible only when it is (1) integrity-safe/non-mutating, (2) confidentiality-safe for client-visible output, and (3) availability-bounded. A read-only command that can expose credentials or create unbounded/expensive work is still denied or rewritten to a safe bounded form.
- **Database defense in depth:** database inspection must not use owner/superuser/application-migration credentials merely because the SQL appears read-only. Require a dedicated diagnostic/read-only database principal where practical, combine that privilege boundary with engine-enforced read-only session/transaction mode, a relay-owned client invocation, statement/query validation, timeout/output/cardinality limits, and engine-specific escape prevention. If a reviewed read-only identity is unavailable, database inspection fails closed instead of trying broader credentials.
- **Docker authority is security-sensitive:** access to the Docker daemon is effectively host-level authority. Docker CLI parsing is therefore a security boundary, not a convenience validator. Only exact reviewed subcommands/flags are allowed; unknown or ambiguous forms fail closed.
- **Remote-side defense in depth:** the client-side relay policy remains mandatory, but production operators should be able to pair it with a dedicated SSH diagnostic principal and/or server-side forced-command/restricted-key policy that cannot open an interactive shell, forwarding, or unrelated host authority. This plan must document the option but must not require or mutate remote host SSH configuration during local implementation.
- **User-requested remote changes:** the AI/tool must reject execution and return a clear policy error. The conversational layer can provide the exact command for the user to run manually, but the relay must not execute it.
- **No systemd/no push:** implementation and validation stop at local code/tests/commit unless the user later gives separate authorization. Nuxt may be rebuilt/recreated if a cross-stack UI/contract change becomes necessary, but this plan is expected to remain Rust/docs-only.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
|---|---|---|---|
| PHASE-01 | Define SSH threat model and read-only policy contract | none | Authentication, config, command, credential, and network boundaries are executable specifications |
| PHASE-02 | Add SSH-specific configuration and sandbox profile | PHASE-01 | Only reviewed SSH material/network authority reaches the SSH subprocess |
| PHASE-03 | Implement SSH invocation and read-only command validation | PHASE-01–02 | Safe aliases/commands execute; interactive/auth/mutation paths fail before or during bounded spawn |
| PHASE-04 | Integrate catalog/effects/activity/errors | PHASE-03 | Client-visible behavior is correctly classified, redacted, bounded, and understandable |
| PHASE-05 | Adversarial and compatibility validation | PHASE-02–04 | Focused tests prove positive key auth and all high-risk negative cases |
| PHASE-06 | Documentation, durable lesson, final local closure | PHASE-01–05 | Docs/memory/plan are truthful; guardrail is green; local commit only; no push/systemd action |

## PHASE-01: Threat Model and Policy Contract

**Goal:** Turn “SSH is read-only and passwordless only” into deterministic server-owned rules.
**Dependencies:** none

### TASK-001: Define effective SSH authentication/config policy

**Outcome:** One validated contract for allowed host aliases, key material, host-key verification, and forbidden OpenSSH capabilities.

**Files:**
- Create/Modify: SSH policy module under `packages/rust-tools/core/src/` or the existing core policy owner selected during implementation review
- Test: `packages/rust-tools/core/tests/ssh_policy.rs`

**Steps:**
- [ ] Define bounded host-alias syntax and explicitly reject raw option injection/host strings beginning with `-`.
- [ ] Define required command-line OpenSSH overrides for BatchMode, password/keyboard-interactive denial, prompt count, stdin/TTY, forwarding, and local command behavior.
- [ ] Parse the operator SSH config with a relay-owned parser into a normalized safe connection specification; do not rely on executing `ssh -G` or equivalent through an unrestricted config surface unless a later review proves it cannot trigger local execution hooks.
- [ ] Allow only the reviewed alias fields needed for connectivity, such as bounded `HostName`, `User`, `Port`, reviewed `IdentityFile`, and known-host sources.
- [ ] Reject dangerous config directives and external/symlinked credential/include paths outside the approved SSH root, including executable/dynamic hooks, connection sharing, agent/provider injection, forwarding, and unsupported multi-hop behavior.
- [ ] Inject server-owned non-interactive/key-only/no-forwarding/no-multiplexing overrides that user input cannot weaken.
- [ ] Preserve strict known-host verification.
- [ ] Add parser fixtures for wildcards, multiple `Host` blocks, duplicate precedence, comments/whitespace, `%`/`~` expansion rules, `Include` rejection, and malformed directives so alias resolution is deterministic and cannot accidentally broaden authority.

**Validation:**
- Unit tests prove dangerous aliases/options/config directives fail closed and safe key-based aliases normalize deterministically.

**Commit boundary:** `feat(ssh): define fail-closed ssh policy`

### TASK-002: Define remote read-only command registry

**Outcome:** A Docker-first semantic command policy with exact argument validation, recursive `docker exec` validation, database-specific read-only enforcement, and parsed diagnostic composition without arbitrary remote shell authority.

**Files:**
- Create/Modify: SSH remote-command policy module under the Rust core/application owner selected during implementation review
- Test: `packages/rust-tools/core/tests/ssh_policy.rs` and/or package-local application integration tests

**Steps:**
- [ ] Enumerate Docker-first diagnostic command families from real operator use cases: container listing/logs/inspect/stats/top/config, nested read-only `docker exec`, database inspection, observability inspection, host/network discovery, file/config reading, and Git inspection.
- [ ] Parse requested remote command syntax into a bounded AST rather than validating an opaque shell string.
- [ ] Validate executable, every option/operand, and every pipeline/conditional node by command family; unknown nodes fail closed.
- [ ] Recursively validate `docker exec` nested commands and reject interactive/TTY shell entrypoints, package managers, editors, service-control/migration commands, and unknown nested executables.
- [ ] Add database adapters that require an explicitly configured/reviewed read-only database principal where practical, combine semantic statement validation with engine-enforced read-only session/connection settings, and generate database-client argv from relay-owned templates rather than forwarding arbitrary CLI options.
- [ ] Harden PostgreSQL/MySQL/MariaDB/SQLite/Redis client semantics: ignore user startup/config hooks where possible, deny client shell/meta-command escapes and credential overrides, block mutating stored procedures/functions or ambiguous SQL, use a Redis-compatible read-command allowlist plus ACL-compatible least privilege, and fail closed for unknown engines.
- [ ] Reject write redirection, write-mode `tee`, background execution, unsafe command substitution, `find -delete/-exec`, `sed -i`, Git mutators, process signals, and other mutation-capable forms.
- [ ] Add confidentiality rules for read-only commands that can expose secrets. Raw `docker inspect`, `docker compose config`, environment dumps, database credential/config output, and equivalent high-risk results must be denied or reduced to server-owned safe projections.
- [ ] Add availability rules per command family: no streaming/follow modes, no detached/background work, bounded log tails/results, bounded SQL statement time/output/cardinality, bounded Redis scans, and denial of expensive/dangerous read commands such as unrestricted `KEYS`/`MONITOR`-class operations.
- [ ] Add deterministic rendering/quoting only after the complete AST passes semantic validation.
- [ ] Return a stable policy error containing a user-actionable explanation, never an invitation for the model to “try another way”.

**Validation:**
- Table-driven tests include safe positives and adversarial bypass attempts for every supported family.

**Commit boundary:** `feat(ssh): enforce remote read-only commands`

**Phase exit criteria:**
- [ ] No requirement depends on model obedience.
- [ ] Password/interactive auth and remote mutation are represented as hard server-side denials.
- [ ] Config-driven local execution/forwarding surfaces are accounted for.

## PHASE-02: SSH-Specific Sandbox and Credential Boundary

**Goal:** Give OpenSSH exactly the resources it needs without weakening ordinary protected-path policy.
**Dependencies:** PHASE-01

### TASK-003: Add SSH-specific sandbox profile

**Outcome:** A dedicated Bubblewrap profile mounts only reviewed SSH material read-only, provides network access, and excludes writable workspaces/optional privileged sockets unless explicitly necessary.

**Files:**
- Modify: `packages/rust-tools/application/src/execution/sandbox.rs`
- Modify if cohesion requires: `packages/rust-tools/application/src/execution/sandbox/**`
- Test: package-local application/infrastructure SSH sandbox tests

**Steps:**
- [ ] Add a profile distinct from ordinary terminal/LSP/hook profiles.
- [ ] Bind the current authorized workspace read-only unless remote command execution has no local-workspace requirement; prefer no writable local bind.
- [ ] Bind only the exact validated known-host/identity material from the normalized SSH connection specification read-only; do not mount raw `.ssh` when narrower mounts suffice.
- [ ] Keep Docker/Tailscale, SSH-agent sockets, control sockets, and unrelated optional sockets unavailable locally.
- [ ] Clear environment and expose only reviewed SSH-required variables; do not inherit `SSH_AUTH_SOCK`, arbitrary askpass helpers, or user-controlled executable search paths.
- [ ] Keep process-group cleanup, bounded stdout/stderr, timeout, and cancellation through the existing job manager.

**Validation:**
- Sandbox tests prove ordinary terminal cannot read `.ssh`, SSH child can use only approved SSH files, unrelated HOME files stay absent, and local workspace mutation is impossible from the SSH process profile.

**Commit boundary:** `feat(ssh): isolate ssh credentials and network`

### TASK-004: Add explicit SSH operator configuration

**Outcome:** SSH is opt-in or explicitly configurable without widening all terminal network authority.

**Files:**
- Modify: `packages/rust-tools/core/src/config.rs`
- Modify: `packages/rust-tools/core/src/config/cli.rs`
- Modify: `.env.example`
- Test: `packages/rust-tools/core/tests/relay_config.rs`

**Steps:**
- [ ] Add the smallest operator-facing SSH enable/config surface needed for least privilege.
- [ ] Default SSH capability to disabled unless repository product requirements justify safe auto-detection; document the chosen default.
- [ ] Keep `RELAY_ALLOW_TERMINAL_NETWORK` semantics unchanged.
- [ ] Validate SSH config root/path ownership/canonicalization without logging secrets.

**Validation:**
- Config tests prove secure defaults, invalid roots fail, and SSH enablement does not alter ordinary terminal network behavior.

**Commit boundary:** `feat(ssh): add explicit relay ssh configuration`

**Phase exit criteria:**
- [ ] `.ssh` remains protected for every non-SSH tool.
- [ ] SSH has network access without globally enabling terminal network.
- [ ] No whole-HOME or agent socket exposure is introduced.

## PHASE-03: Invocation and Execution

**Goal:** Support familiar SSH invocation while routing it through the stricter SSH boundary.
**Dependencies:** PHASE-01–02

### TASK-005: Route `terminal_exec` SSH requests through SSH policy

**Outcome:** `command="ssh"` is recognized as a special security-sensitive execution class rather than an ordinary terminal executable.

**Files:**
- Modify: `packages/rust-tools/application/src/execution/requests.rs`
- Modify if needed: `packages/rust-tools/application/src/execution.rs`
- Modify: `packages/rust-tools/core/src/terminal_policy.rs`
- Test: package-local application execution tests

**Steps:**
- [ ] Detect only the resolved OpenSSH client executable; reject alternate shell wrappers or arbitrary SSH-compatible executables.
- [ ] Parse host alias and remote command tokens with a dedicated parser, not generic shell parsing.
- [ ] Resolve the alias through the relay-owned sanitized SSH config parser; pass OpenSSH an explicit normalized connection spec rather than an unrestricted raw config.
- [ ] Reject bare `ssh alias`, SSH option injection, PTY/forwarding/config overrides, file-transfer modes, SSH-agent use, connection sharing/control sockets, askpass hooks, and unsupported multi-hop behavior.
- [ ] Validate the remote command against the server-owned integrity/confidentiality/availability policy.
- [ ] Inject mandatory non-interactive/key-only/no-forwarding/no-multiplexing security overrides after rejecting conflicting user options.
- [ ] Route spawn to the SSH-specific sandbox/network/credential profile.
- [ ] Preserve existing timeout/job/cancellation semantics.

**Validation:**
- Positive fixture executes a safe remote command using a non-interactive test SSH server/key.
- Bare interactive shell and unsafe option attempts fail before remote command execution.

**Commit boundary:** `feat(ssh): route terminal ssh through safe execution`

### TASK-006: Normalize authentication failure behavior

**Outcome:** Password/passphrase/interactive requirements terminate without trial-and-error or secret discovery.

**Files:**
- Modify: SSH invocation/result adapter in application execution
- Test: SSH integration tests

**Steps:**
- [ ] Ensure stdin cannot be used to answer prompts.
- [ ] Bound connect/auth timeout independently from long remote-command timeout where needed.
- [ ] Map expected OpenSSH auth failures to stable redacted categories.
- [ ] Never echo raw config paths, key paths, usernames beyond reviewed presentation policy, or OpenSSH diagnostics that can disclose private host details unnecessarily.
- [ ] Confirm there is exactly one logical connection attempt per tool call unless OpenSSH itself follows an explicitly permitted safe `ProxyJump` chain.

**Validation:**
- Password-only, encrypted-key-without-agent, unknown host key, changed host key, and unavailable host fixtures all fail quickly and without prompt/retry loops.

**Commit boundary:** `fix(ssh): fail closed on interactive authentication`

**Phase exit criteria:**
- [ ] Key-based non-interactive SSH succeeds.
- [ ] Any required human authentication stops immediately.
- [ ] Interactive shell access is impossible.

## PHASE-04: Catalog, Effects, Activity, and UX Contract

**Goal:** Make SSH behavior visible and correctly classified without exposing credentials.
**Dependencies:** PHASE-03

### TASK-007: Add SSH effect/risk classification without pretending it is ordinary local terminal

**Outcome:** Approval/activity/observability layers understand SSH as network read + process execution with a server-enforced read-only remote policy.

**Files:**
- Modify: `packages/rust-tools/application/src/hooks/policy.rs`
- Modify: `packages/rust-tools/application/src/activity.rs`
- Modify: `packages/rust-tools/application/src/activity/presentation.rs`
- Modify if required: MCP catalog/tool metadata owners under `packages/rust-tools/interfaces/src/mcp/`
- Test: existing effect/activity contract tests

**Steps:**
- [ ] Preserve `terminal_exec` compatibility while deriving SSH-specific effects from concrete validated input where architecture permits.
- [ ] Mark network access accurately; do not label SSH as `workspace_write` or `external_mutation` when the policy only permits read-only remote commands.
- [ ] Record a bounded human-readable action such as `ssh smart-meeting — systemctl status app` with credential/path redaction.
- [ ] Never journal private key/config contents or raw sensitive stderr.

**Validation:**
- Effect parity and activity presentation tests cover safe SSH and denied mutation attempts.

**Commit boundary:** `feat(ssh): classify and present ssh activity`

### TASK-008: Add clear policy error semantics

**Outcome:** Callers can distinguish unsupported mutation, interactive-auth requirement, host-key failure, and connection failure without raw-secret leakage.

**Files:**
- Modify: relevant Rust error/result adapter modules
- Test: security/confidentiality integration tests

**Steps:**
- [ ] Use bounded stable categories/messages.
- [ ] For mutation attempts, explicitly state that AI SSH execution is read-only and the requested command was not executed.
- [ ] Preserve enough command text for the conversational layer to offer a manual user command while redacting credential-shaped values.

**Validation:**
- Client-visible errors contain no raw private-key/config content or unrestricted OpenSSH stderr.

**Commit boundary:** `fix(ssh): bound ssh policy diagnostics`

**Phase exit criteria:**
- [ ] SSH activity/effects are truthful.
- [ ] Denials are actionable but confidentiality-safe.

## PHASE-05: Adversarial and Compatibility Validation

**Goal:** Prove the boundary against realistic bypass attempts.
**Dependencies:** PHASE-02–04

### TASK-009: Add deterministic local SSH fixtures

**Outcome:** Tests do not depend on production hosts, real user keys, or the running systemd relay.

**Files:**
- Add package-local Rust integration-test fixtures/helpers under the existing Rust test layout
- Do not add plan-numbered scripts under `scripts/`

**Steps:**
- [ ] Use disposable temporary keys/config/known_hosts and a local test SSH server fixture if available without adding disproportionate infrastructure; otherwise use a deterministic mock/process fixture at the OpenSSH boundary plus at least one opt-in real-client integration test.
- [ ] Cover key-only success and password-only failure.
- [ ] Cover encrypted/passphrase-required key failure without prompting.
- [ ] Cover unknown/changed host key failure.
- [ ] Cover host aliases and safe config fields.

**Validation:**
- Focused SSH integration suite is deterministic and secret-independent.

**Commit boundary:** `test(ssh): add deterministic ssh fixtures`

### TASK-010: Add mutation-bypass matrix

**Outcome:** High-risk command and config bypasses are regression-tested.

**Files:**
- Test: `packages/rust-tools/core/tests/ssh_policy.rs`
- Test: package-local application/infrastructure SSH integration tests

**Steps:**
- [ ] Reject `touch`, `rm`, `mv`, `cp`, `mkdir`, `chmod`, `chown`, package managers, editors, DB clients with mutation input, process signaling, reboot/shutdown, service start/stop/restart, Docker/Kubernetes mutation, and Git mutation.
- [ ] Reject unsafe shell composition while allowing only AST forms explicitly supported by the policy; pipelines/conditional chains are accepted only when every node is independently read-only, confidentiality-safe, and bounded.
- [ ] Reject dual-use bypasses such as `find -delete/-exec`, `sed -i`, `awk system()`, `perl/python/ruby/node -e`, `git -c core.pager=...`, pager/editor command escapes, nested `docker exec ... sh`, Docker lifecycle/mutation subcommands, writable SQL/DDL, mutating stored functions, Redis mutation/dangerous commands, and arbitrary `ssh -o ...` overrides.
- [ ] Reject dangerous Docker exec flags including interactive/TTY/detach/privileged/environment injection; keep `docker compose exec` unsupported in v1 unless a later dedicated policy proves equivalent safety.
- [ ] Prove confidentiality-safe projections: secret-bearing fields from `docker inspect`, full Compose environment resolution, raw credential/config dumps, and equivalent sensitive outputs are denied or redacted by construction.
- [ ] Prove availability bounds: `docker logs -f`, streaming `docker stats`, detached exec, unbounded tail/follow, expensive Redis scans/`KEYS`, sleep-like SQL/functions, and oversized query/log output fail or are rewritten to reviewed bounded forms.
- [ ] Prove database access cannot fall back from the configured read-only principal to a broader DB owner/superuser and that client startup/meta-command/shell escapes are unavailable.
- [ ] Prove allowed diagnostic pipelines/conditional chains remain safe node-by-node (for example `docker logs api --tail 100 | grep ERROR | tail -100`), and reject conditional composition when failure-path semantics could invoke a non-reviewed command.
- [ ] Reject `ProxyCommand`, `KnownHostsCommand`, `LocalCommand`, `Match exec`, forwarding directives/options, connection multiplexing, SSH-agent exposure, askpass helpers, and X11 exposure.
- [ ] Prove safe commands cannot mutate a fixture remote directory during the test matrix.

**Validation:**
- Every adversarial case is denied before mutation; remote fixture checksum/tree state remains unchanged.

**Commit boundary:** `test(ssh): prove read-only enforcement`

### TASK-011: Run Rust security/quality closure

**Outcome:** The implementation satisfies repository gates without unrelated stack work.

**Validation:**
- Focused SSH tests pass.
- `pnpm lint:rust` passes.
- `pnpm typecheck:rust` passes.
- `pnpm test:rust` passes.
- `cargo audit` runs when available/relevant for changed dependency state.
- `pnpm guardrail` passes before each local commit.
- No relay systemd unit/process is restarted/reloaded/stopped/started.
- No Git push occurs.

**Commit boundary:** none beyond the logical commits above; final cleanup commit only if needed.

**Phase exit criteria:**
- [ ] Positive and negative SSH behavior is deterministic.
- [ ] Remote mutation matrix remains unchanged.
- [ ] Repository Rust guardrail is green.

## PHASE-06: Documentation and Local Closure

**Goal:** Leave repository truth and operator expectations aligned with the new capability.
**Dependencies:** PHASE-01–05

### TASK-012: Update operator/client documentation

**Outcome:** Operators understand enablement, key/config prerequisites, read-only limits, and failure behavior.

**Files:**
- Modify: `docs/external-mcp.md`
- Modify: `docs/troubleshooting.md`
- Modify if architecture contract changes: `docs/architecture.md`
- Modify: `.env.example`

**Steps:**
- [ ] Document familiar `ssh <alias> <read-only-command>` examples.
- [ ] State that bare interactive SSH is intentionally unsupported.
- [ ] State that password/passphrase/keyboard-interactive auth stops immediately.
- [ ] State that AI cannot perform SSH mutations; provide manual-command guidance semantics.
- [ ] Document strict host-key/config restrictions and unsupported directives.
- [ ] Document Docker confidentiality/availability limits, unsupported `docker compose exec` v1 behavior, and safe projection semantics.
- [ ] Document database diagnostic prerequisites: explicit least-privilege read-only DB identity/ACL, no fallback to owner/superuser credentials, and engine-specific read-only/session/query bounds.
- [ ] Document optional production defense in depth using a dedicated SSH diagnostic principal and/or server-side forced-command/restricted-key policy without making remote SSH reconfiguration a prerequisite for local implementation.
- [ ] Do not instruct operators to weaken known-host checking or expose an SSH agent globally.

**Validation:**
- Docs exactly match implemented defaults and supported command families.

**Commit boundary:** `docs(ssh): document read-only remote access`

### TASK-013: Close plan and durable self-improvement review

**Outcome:** Plan status, canonical repository memory, and `ai-self` reusable lesson are updated only with validated durable facts.

**Files:**
- Modify: `.agents/plans/061-read-only-ssh-execution.md`
- Modify if durable: `.agents/memories/README.md`
- Modify if reusable lesson exists: `ai-self/lessons/lessons.md` or an existing relevant skill

**Steps:**
- [ ] Follow `.agents/knowledge/self-improvement.md`.
- [ ] Record the reusable principle that remote “read-only” requires semantic allowlisting at the remote execution boundary; shell-string denylisting is not sufficient.
- [ ] Record exact local acceptance and remaining deployment/runtime validation separately.
- [ ] Revalidate workspace identity and branch before Git commit.
- [ ] Commit locally only; do not push.

**Validation:**
- `git status --short` is clean after final local commit.
- Branch remains `feat/061-read-only-ssh`.
- No remote branch was created and no systemd relay action occurred.

**Commit boundary:** `docs(plan): close read-only ssh implementation`

**Phase exit criteria:**
- [ ] Code/tests/docs/plan/memory tell the same truth.
- [ ] Local branch is committed and clean.
- [ ] Deployment/restart remains a separate operator-authorized action.


## Docker-first Remote Diagnostic Model

The target environment is container-centric: application services, databases, observability, and supporting infrastructure are expected to run in Docker rather than systemd-managed workloads. The SSH feature therefore optimizes for Docker-host diagnostics while preserving a strict remote read-only boundary.

### Allowed diagnostic classes

A diagnostic action is eligible only when it passes all three dimensions: **integrity-safe**, **confidentiality-safe**, and **availability-bounded**.

- Host observation required to reach and understand Docker: `uname`, `uptime`, bounded `df`/`free`/`ps`/`ss`/`ip`, `hostname`, `id`, `whoami`, `command -v`, and equivalent non-mutating discovery.
- Docker observation: exact reviewed forms of `docker ps`, `docker container ls`, `docker images`, safe projections of `docker inspect`, bounded `docker logs`, `docker stats --no-stream`, `docker top`, selected `docker network inspect`/`volume inspect` projections, `docker compose ps`, bounded `docker compose logs`, and only confidentiality-safe `docker compose config` query forms such as service/image/profile/name/hash validation metadata. Full environment-resolving Compose output is not generally safe.
- Bounded `docker exec` only when the Docker flags and the nested executable/arguments are independently validated. Interactive/TTY, detach/background, privileged execution, environment injection, shell entrypoints, and unrestricted interpreters are forbidden.
- Database inspection through containerized clients only with a reviewed read-only DB principal where practical, engine-level read-only enforcement, relay-owned client argv/config, semantic query validation, and timeout/output/cardinality bounds. No fallback to DB owner/superuser/application-migration identities.
- Container filesystem/config/log inspection through bounded read-only commands such as `cat`, `head`, `tail`, `grep`, constrained `awk`/`sed`, and `find` without mutation actions, provided the target/output is not a protected credential surface.
- HTTP diagnostics from the remote host or inside containers only for bounded GET/HEAD-style observation unless a protocol-specific read-only operation is reviewed separately.
- Trial-and-error capability discovery is allowed when every attempted operation remains observational, confidentiality-safe, and bounded, for example trying `docker`, then `podman`, then `nerdctl`, or locating a database client with `command -v`. Credential guessing or identity guessing is never part of trial-and-error.

### Explicitly denied classes

- Docker lifecycle or mutation: `run`, `start`, `stop`, `restart`, `kill`, `rm`, `rmi`, `build`, `pull`, `push`, `commit`, `update`, `cp`, `system prune`, `compose up/down/restart`, and equivalent state-changing operations.
- `docker exec` payloads that can mutate state, including shell/interpreter entrypoints, package managers, editors, file writes, service-control commands, migration tools, or unknown/unclassified executables.
- Database writes or DDL: `INSERT`, `UPDATE`, `DELETE`, `MERGE`, `COPY ... FROM`, mutating stored procedures/functions, schema/role/extension operations, writable transaction modes, and equivalent engine-specific mutations.
- Shell redirection or file creation: `>`, `>>`, write-mode `tee`, output-to-file constructs, mutating `sed -i`, `find -delete`, `xargs` into unclassified commands, background processes, or nested unrestricted shells.
- Privilege escalation, SSH forwarding, remote port forwarding, local port forwarding, agent forwarding, X11 forwarding, arbitrary `ProxyCommand`, arbitrary `LocalCommand`, and any SSH config directive that expands local execution/network authority beyond the reviewed connection path.

### Nested validation rule

`docker exec` is not inherently read-only. The relay must parse the Docker subcommand and then recursively validate the command executed inside the selected container. A request is allowed only when both layers are classified as read-only. Unknown commands fail closed.

Examples:

```text
ALLOW  ssh smart-meeting docker logs api --tail 200
ALLOW  ssh smart-meeting docker inspect api <relay-owned-safe-projection>
ALLOW  ssh smart-meeting docker exec postgres psql <relay-owned-readonly-options> -c "SELECT count(*) FROM users"
ALLOW  ssh smart-meeting docker exec api cat /app/config.json
DENY   ssh smart-meeting docker restart api
DENY   ssh smart-meeting docker exec api sh
DENY   ssh smart-meeting docker exec postgres psql -d app -c "DELETE FROM users"
```

### Database defense in depth

SQL text filtering alone is insufficient, and transaction-level read-only mode is not a complete privilege boundary. Database diagnostics should use a dedicated least-privilege read-only principal where practical and must not silently fall back to a DB owner, superuser, migration identity, or broader application credential. The relay then adds a second engine-level read-only session/transaction control and a third semantic query/client policy layer. PostgreSQL should use a relay-owned `psql` invocation that ignores user startup hooks, a read-only principal, read-only transaction/session settings, and statement timeout; MySQL/MariaDB should use a reviewed read-only principal/session plus safe client options; SQLite should open the database in read-only mode; Redis-compatible access should use both a restricted diagnostic ACL/identity and an explicit safe command allowlist. Unknown engines remain unavailable until a reviewed adapter exists. Expensive or side-effect-capable reads/functions are denied even if the SQL/command is syntactically read-only.

### SSH authentication and host trust

- Read the operator's existing SSH alias/config only through a relay-owned safe parser and convert it into a normalized connection specification; do not execute raw config directives as part of resolution. `.ssh` remains protected from general terminal/file tools.
- Force non-interactive key-only authentication (`BatchMode=yes`, password/keyboard-interactive disabled, one connection attempt), `IdentitiesOnly=yes`, and `IdentityAgent=none`. If a key requires a passphrase that is not already usable non-interactively, stop immediately.
- Disable SSH connection multiplexing/control sockets, agent forwarding, X11, port/socket forwarding, askpass/local command hooks, and escape processing.
- Never guess usernames, passwords, alternate identities, private-key paths, or credentials.
- Preserve strict host-key verification and never auto-accept an unknown/changed host key.

## Risks & Rollback

- **False read-only classification could mutate a remote host** → use a positive semantic allowlist with exact argument rules; no arbitrary shell strings or “looks safe” heuristic. If a command family cannot be proven read-only, do not support it.
- **Read-only output can still leak secrets** → add confidentiality classification/projection per command family; do not expose raw Docker/Compose/env/credential-bearing output by default.
- **Read-only work can still exhaust production resources** → require command-family timeout/output/cardinality limits and deny follow/stream/detach/expensive operations.
- **SSH config can execute local programs or widen credentials/network authority** → parse only a reviewed safe subset into a normalized connection spec; reject executable/dynamic directives and force server-owned no-agent/no-forwarding/no-multiplexing options.
- **Mounting `.ssh` could expose private keys to unrelated tools** → create a dedicated SSH-only sandbox bind with exact credential files; keep global protected-path behavior unchanged and test ordinary terminal denial explicitly.
- **Docker daemon authority is effectively host-level** → treat Docker subcommand/flag/nested-command parsing as a security boundary and fail closed on unknown forms; recommend a restricted remote diagnostic principal as defense in depth.
- **Database read-only mode can be bypassed by excessive privilege or engine exceptions** → require least-privilege diagnostic identities plus engine read-only mode, client hardening, semantic validation, and bounded execution; never fall back to broader credentials.
- **Password/passphrase prompt could hang a job** → BatchMode + disabled interactive auth + no stdin + bounded connect/auth timeout; map failure without retries.
- **Host-key bypass could enable MITM** → strict known-host verification only; unknown/changed keys fail.
- **Read commands can be dual-use** → validate command-specific options and reject pager/editor hooks and escape-capable modes.
- **Compatibility with complex SSH configs** → start fail-closed and support only reviewed directives; expand via tests rather than permissive fallback.
- **Regression to terminal network policy** → SSH network authority must be separately owned and tested; ordinary terminal network default remains unchanged.
- **Rollback:** revert Plan 061 commits. Because no relay restart/deployment is part of this plan, rollback is repository-local until separately deployed.

## Final Acceptance Criteria

- [ ] Existing OpenSSH alias with immediately usable file-based key can execute a supported read-only remote command.
- [ ] Bare `ssh alias` interactive shell is rejected.
- [ ] Password-only, passphrase-required, and keyboard-interactive authentication stop without guessing or prompting.
- [ ] Unknown/changed host keys fail closed.
- [ ] Ordinary terminal/workspace tools still cannot read `.ssh`.
- [ ] SSH child sees only reviewed SSH material, not the whole HOME.
- [ ] Agent/X11/port/socket forwarding, `LocalCommand`, arbitrary `ProxyCommand`, and unsafe option overrides are denied.
- [ ] Raw SSH config execution surfaces are eliminated: only a relay-normalized safe connection specification reaches OpenSSH, and agent/forwarding/multiplexing/local-command hooks are disabled.
- [ ] Remote command execution is a positive semantic policy over a bounded parsed AST; useful read-only diagnostic composition works, while arbitrary shell authority is impossible.
- [ ] Docker diagnostic policy validates subcommands, flags, nested `docker exec`, confidentiality-safe projections, and bounded execution; `docker compose exec` remains unsupported in v1 unless separately reviewed.
- [ ] Database diagnostics require the configured/reviewed read-only identity and cannot fall back to DB owner/superuser/application-migration credentials; engine read-only mode, client hardening, semantic query policy, and resource bounds are proven.
- [ ] Mutation-bypass test matrix leaves the remote fixture unchanged.
- [ ] Confidentiality tests prove Docker/Compose/env/DB/SSH credential surfaces are not exposed through otherwise read-only commands.
- [ ] Availability tests prove follow/stream/detach/expensive/unbounded diagnostic operations are denied or safely bounded.
- [ ] SSH output/errors/activity are bounded and credential-redacted.
- [ ] Existing timeout/cancellation/process cleanup semantics remain intact.
- [ ] Focused tests and `pnpm guardrail` pass.
- [ ] No systemd relay restart/reload/stop/start occurred.
- [ ] No Git push occurred.
- [ ] Plan/docs/memory reflect exact implementation and validation truth.

## Execution Handoff

1. Implement PHASE-01 first; do not touch SSH credential mounts before the command/config threat model tests exist.
2. PHASE-02 and PHASE-03 are security-critical and should be reviewed together before exposing the capability through normal `terminal_exec` routing.
3. Do not add a permissive fallback for unsupported SSH configs or remote commands. Unsupported means denied until explicitly reviewed.
4. Keep implementation Rust/docs-only unless a real client contract/UI requirement is discovered; if Nuxt remains untouched, do not run Nuxt gates merely for completeness.
5. Stop after local commits. Do not push and do not restart/reload the systemd relay.
