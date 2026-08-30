# Plan 061 — Read-Only SSH Execution

**Status:** PLANNED / NOT IMPLEMENTED
**Goal:** Add industrial-grade SSH capability to the Rust relay so an AI can use operator-owned OpenSSH host aliases and key-based authentication for bounded remote inspection while being technically prevented from password guessing, interactive authentication, interactive shells, forwarding, and remote mutation.
**Success Criteria:** `ssh <configured-alias> <approved-read-only-command...>` works through the relay when the host can authenticate non-interactively with an existing key; any password/passphrase/keyboard-interactive requirement fails immediately without retries; interactive SSH sessions are rejected; remote commands are constrained by a server-owned read-only policy rather than model judgment; SSH credentials/config are exposed only to the SSH-specific sandbox profile; dangerous OpenSSH directives/capabilities fail closed; focused adversarial tests and the Rust guardrail pass; no systemd relay restart/reload and no Git push occur during implementation unless separately authorized later.

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
- **Structured allowlist:** start with a deliberately bounded server-owned registry of reviewed read-only command families and exact safe argument rules. Examples may include host/system inspection (`pwd`, `uname`, `hostname`, `id`, `whoami`, bounded `ls`/`stat`/`cat`/`head`/`tail`/`wc`/`df`/`du`, `ps`), read-only service/log inspection (`systemctl status/show/cat/list-*`, `journalctl` with non-following bounded options), and read-only Git inspection (`git status/log/show/diff/branch --list/rev-parse`) where argument semantics are explicitly validated. Commands/options with mutation-capable modes are denied unless a safe exact subset is proven.
- **No shell composition:** reject shell metacharacters, command substitution, redirections, pipelines, separators, environment-prefix execution, nested shells/interpreters, `sh -c`, `bash -c`, `env <cmd>`, `xargs`, `find -exec/-delete`, editor/pager escapes, and other composition surfaces. Build the remote command from validated tokens with a deterministic POSIX-safe quoting routine only after validation.
- **SSH config is untrusted capability input:** host aliases may come from the operator's existing config, but SSH-specific validation must reject dangerous effective configuration such as arbitrary `ProxyCommand`, `LocalCommand`, forwarding, agent/X11 forwarding, PKCS#11/security-key provider execution, or other directives that widen local execution/credential authority. `ProxyJump` may be supported only if it can be proven to preserve the same non-interactive/key-only/no-forwarding policy end to end; otherwise fail closed initially.
- **Minimum credential mount:** ordinary terminal/workspace tools continue to see `.ssh` as protected. Only the SSH profile may receive a read-only bind of the reviewed SSH config/known-host/key files needed by OpenSSH. Do not bind the entire HOME. Symlink targets must be canonicalized and constrained to the approved SSH credential root; unexpected external includes/identity paths fail closed.
- **Host-key verification stays strict:** do not use `StrictHostKeyChecking=no`, `UserKnownHostsFile=/dev/null`, or automatic trust-on-first-use. Existing trusted known-host state may be used read-only; unknown/changed host keys fail.
- **No privilege escalation remotely:** reject `sudo`, `su`, `doas`, `pkexec`, shells/interpreters capable of arbitrary execution, and command families that can transition into a mutating mode.
- **Network authority remains explicit:** SSH requires network. Prefer an SSH-specific network-enabled invocation path rather than turning all terminal subprocesses network-enabled. Do not make `RELAY_ALLOW_TERMINAL_NETWORK=true` a prerequisite if a narrower SSH capability flag/profile can provide least privilege.
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
- [ ] Resolve/review effective host configuration without exposing private key contents.
- [ ] Reject dangerous config directives and external/symlinked credential/include paths outside the approved SSH root.
- [ ] Preserve strict known-host verification.

**Validation:**
- Unit tests prove dangerous aliases/options/config directives fail closed and safe key-based aliases normalize deterministically.

**Commit boundary:** `feat(ssh): define fail-closed ssh policy`

### TASK-002: Define remote read-only command registry

**Outcome:** A narrow command-family registry with exact argument validation and no arbitrary remote shell text.

**Files:**
- Create/Modify: SSH remote-command policy module under the Rust core/application owner selected during implementation review
- Test: `packages/rust-tools/core/tests/ssh_policy.rs` and/or package-local application integration tests

**Steps:**
- [ ] Enumerate the initial reviewed read-only command families from real operator use cases.
- [ ] Validate executable and every option/operand by command family.
- [ ] Reject redirection/composition/metacharacters and nested command interpreters.
- [ ] Reject known dual-use/mutation flags (`find -delete/-exec`, `sed -i`, Git mutators, `systemctl` mutators, process signals, package managers, editors, etc.).
- [ ] Add deterministic token quoting only after semantic validation.
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
- [ ] Bind only validated SSH config/known-host/identity paths read-only.
- [ ] Keep Docker/Tailscale and unrelated optional sockets unavailable.
- [ ] Clear environment and expose only reviewed SSH-required variables.
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
- [ ] Reject bare `ssh alias`, SSH option injection, PTY/forwarding/config overrides, file-transfer modes, and unsupported multi-hop behavior.
- [ ] Validate the remote command against the server-owned read-only registry.
- [ ] Inject mandatory security overrides after rejecting conflicting user options.
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
- [ ] Reject shell composition: `;`, `&&`, `||`, `|`, redirects, `$()`, backticks, heredocs, newline injection, wildcard patterns where unsafe, and nested shell/interpreter execution.
- [ ] Reject dual-use bypasses such as `find -delete/-exec`, `sed -i`, `awk system()`, `perl/python/ruby/node -e`, `git -c core.pager=...`, pager/editor command escapes, `systemctl edit`, `journalctl -f` if unbounded, and arbitrary `ssh -o ...` overrides.
- [ ] Reject `ProxyCommand`, forwarding directives/options, and agent/X11 exposure.
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

## Risks & Rollback

- **False read-only classification could mutate a remote host** → use a positive semantic allowlist with exact argument rules; no arbitrary shell strings or “looks safe” heuristic. If a command family cannot be proven read-only, do not support it.
- **SSH config can execute local programs or widen credentials/network authority** → validate effective config and reject dangerous directives; override forwarding/local-command/auth behavior from server-owned arguments.
- **Mounting `.ssh` could expose private keys to unrelated tools** → create a dedicated SSH-only sandbox bind; keep global protected-path behavior unchanged and test ordinary terminal denial explicitly.
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
- [ ] Remote command execution is a positive semantic allowlist; arbitrary shell composition is impossible.
- [ ] Mutation-bypass test matrix leaves the remote fixture unchanged.
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
