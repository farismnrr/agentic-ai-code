# Plan 065 — Broad User-Space Terminal and MCP-First Tool Routing

**Status:** CLOSED / LOCALLY VERIFIED — host user-service bridge intentionally excluded
**Goal:** Make the relay terminal behave like a practical operator-user shell across the configured execution root while preserving hard credential and privilege boundaries, and make coding agents prefer dedicated MCP tools before falling back to terminal execution.

**Success Criteria:**
- A relay configured with `--execution-root "$HOME"` can run normal user-space workflows from any non-protected path beneath the operator home instead of being restricted to one repository sandbox root.
- `terminal_exec` remains bounded by the configured execution root / authorized workspace roots; this plan does not create unrestricted root-filesystem write authority.
- Standard user-space workflows work through terminal execution where no dedicated MCP capability is appropriate: builds, tests, package managers, interpreters, repository scripts, process inspection, sandbox-local service commands, and ordinary network commands when terminal networking is already enabled by operator policy.
- Privilege escalation remains impossible through direct execution and through shell wrapping; at minimum `sudo`, `su`, `doas`, `pkexec`, and `runas` remain denied, and the sandbox must not expose an equivalent root-capable bridge by accident.
- Credential-bearing files/directories remain unreadable and unwritable from terminal execution, including current protected-path families such as `.ssh`, `.gnupg`, `.aws`, `.config/gcloud`, `.config/gh`, `.docker`, `.kube`, `.npmrc`, `.netrc`, `.pypirc`, `.git-credentials`, Cargo credentials, and `.env*` except the existing non-secret `.env.example` exception.
- Relay terminal children continue to receive a scrubbed environment rather than inheriting operator secrets, and credential-agent/keyring sockets remain unavailable unless a separately reviewed dedicated capability intentionally exposes narrowly scoped material.
- Generic `ssh`/`scp`/`sftp` remain unavailable through `terminal_exec`; remote diagnostics continue to use `ssh_readonly_exec`.
- Docker/Tailscale or other privileged sockets are not enabled as an implicit consequence of broad terminal access. Existing explicit operator opt-ins remain separate authority expansions.
- The primary coding agent receives a request-scoped tool-selection policy derived from the actual active tool set and is instructed to use a dedicated MCP tool when it fully covers the operation.
- Agent guidance explicitly prefers structured Git tools (`git_*`) over terminal Git commands, structured file/search/code tools over shell equivalents, dedicated HTTP/SSH/Telegram/forge tools over CLI equivalents, and uses terminal only when no dedicated tool is suitable or the workflow is inherently terminal-native.
- `terminal_exec` and `terminal_job_start` descriptions no longer advertise Git as a preferred generic terminal use case; they clearly present terminal execution as a fallback/general CLI capability.
- Tool-routing behavior is deterministic in prompts/catalog metadata and covered by unit/contract tests; no brittle attempt is made to ban all legitimate Git or shell use solely by command-name heuristics.
- Existing permission modes, effect checks, approval boundaries, protected-path checks, Git credential isolation, catalog immutability rules, activity redaction, and deployment separation remain intact.
- `pnpm guardrail:rust` passes for Rust-only stages, `pnpm guardrail:nuxt` passes for server/agent-routing stages, and `pnpm guardrail` passes at final closure because this initiative changes both stacks plus repository guidance.
- No systemd restart/reload, production deployment, tag, release publish, force push, merge, or destructive external action is performed as part of plan execution without a separate explicit user request/approval.

## Scope

### In scope
- Broaden terminal filesystem usability within the already configured execution-root authority, including support for `$HOME` as the sandbox root when `$HOME` is intentionally configured as the execution root.
- Preserve/strengthen protected credential masking when the sandbox root is large (for example the whole operator home).
- Harden privilege-denial so shell wrapping cannot bypass the top-level executable policy.
- Preserve existing generic SSH denial and dedicated read-only SSH routing.
- Preserve explicit privileged-socket opt-ins and prevent broad terminal mode from implicitly exposing them.
- Add a canonical MCP-first tool-selection policy for the primary coding agent using the actual active tool set for each turn.
- Apply equivalent tool-selection guidance to delegated/subagent execution at the shared runtime/prompt composition boundary rather than duplicating inconsistent prose across profiles where avoidable.
- Update relay tool descriptions so models see terminal as fallback/general CLI rather than an equivalent replacement for first-class MCP tools.
- Add focused Rust and TypeScript tests for home-root sandboxing, credential masking, privilege-denial, and tool-routing prompt construction.
- Update agent/operator documentation for the new terminal behavior and tool-selection hierarchy.
- Publish the next immutable MCP catalog snapshot if current repository catalog-contract policy requires a new snapshot for changed client-visible descriptions.

### Out of scope
- Allowing `sudo`, root shells, `su`, `doas`, `pkexec`, or equivalent privilege escalation.
- Exposing real credentials to the model, including SSH private keys, GitHub/Git credentials, cloud credentials, package-registry credentials, `.env` secrets, keyrings, browser sessions/cookies, or arbitrary inherited environment secrets.
- Replacing the dedicated `ssh_readonly_exec` path with generic terminal SSH.
- Automatically enabling Docker, Tailscale, system D-Bus, host keyrings, SSH agent forwarding, GPG agent forwarding, or other privileged/credential-bearing sockets.
- Removing Bubblewrap or running model-selected commands directly in the host namespace.
- Making `/` a writable execution root or mounting the complete host root filesystem writable.
- Weakening current user approval/permission modes because terminal becomes more capable.
- Creating a new “tool discovery” MCP method solely for the model to list tools; the model already receives the active tool schemas for a turn.
- Hard-blocking every `git` invocation in the terminal. Dedicated Git tools are the preferred route, but terminal Git remains a fallback for a genuinely uncovered Git operation or a shell-native workflow.
- Redesigning native file/Git/code/forge tool contracts beyond what is needed to advertise/consume the routing policy.
- Restarting the operator relay service or changing live systemd configuration during implementation without separate approval.

## Current State

Verified repository/context facts at planning time:

- Repository identity is verified by `ai-self/project.yaml` as `masih-awam-ai-code`, with expected origin `https://github.com/farismnrr/agentic-ai-code.git`; the active checkout is `/home/farismnrr/Projects/MasihAwam/ai-code` on `main`.
- The worktree has no staged/unstaged tracked changes at planning time; an unrelated untracked `agents/` directory already exists and must not be touched or staged by this initiative.
- Current MCP catalog contract is on the v13 snapshot line (`catalog_v13_snapshot_matches_current_static_surface`).
- `terminal_exec` currently advertises support for “scripts, builds, package managers, Git, and interpreters,” which gives models no reason to prefer the much safer/more structured native Git surface.
- The primary chat system prompt currently only establishes workspace identity plus plan/read-only mode text. It does not state any dedicated-tool-first selection policy.
- Primary chat already has the actual model-facing tool map before the final AI SDK agent call, so a request-scoped tool-selection prompt can be generated from the tools actually available on that turn without inventing a separate discovery API.
- The relay already exposes dedicated families for structured workspace reads/writes, Git, code intelligence, HTTP/web access, forge/change-request operations, SSH diagnostics, Telegram messaging, and terminal execution.
- Current `build_terminal_invocation` resolves `cwd` through authorized roots, clears the child environment, derives terminal networking from `allow_terminal_network`, exposes optional sockets only through explicit config, and marks terminal invocation as standard sandbox execution.
- `sandbox::spawn` currently discovers a repository/workspace sandbox root and explicitly rejects `sandbox_root == host_home`, which prevents `$HOME` from being used as the effective sandbox root even when the configured execution root is intentionally `$HOME`.
- Bubblewrap currently mounts standard system runtime directories read-only, creates isolated `/dev`, `/proc`, and `/tmp`, mounts the selected workspace root read/write for terminal execution, and masks protected credential paths discovered within the mounted workspace.
- Protected-path detection already covers the important owner credential stores and nested `.env*` files, with `.env.example` intentionally non-secret.
- The sandbox recursively discovers protected paths under the mounted root with a bounded scan (`MAX_PROTECTED_SCAN_ENTRIES`) and skips heavy build/cache directories. Moving the mounted root from one repository to the entire home directory therefore requires explicit scalability/fail-closed validation rather than simply removing the `$HOME` guard.
- Top-level `terminal_policy::validate_executable` currently rejects `sudo`, `su`, `doas`, `pkexec`, and `runas`, plus generic SSH clients; however a shell executable such as `sh -lc "sudo ..."` passes top-level executable validation. Generic SSH is additionally protected inside Bubblewrap by masking the actual SSH client binaries; privilege-escalation binaries do not currently have an equivalent sandbox-level mask.
- The sandbox uses `env_clear()` and reconstructs a minimal environment (`HOME`, safe `PATH`, locale, temp path, selected toolchain homes), which is a strong existing credential boundary and must remain.
- Docker and Tailscale sockets are already explicit configuration-controlled authority expansions. Docker in particular is root-equivalent on common deployments and must not become part of the broad terminal default.
- The documented single-owner relay profile already recommends `--dir "$HOME"` and `--execution-root "$HOME"`; implementation currently does not fully honor that intended broad terminal shape because of the home-root sandbox rejection.

## Constraints & Decisions

### D-01 — Reuse the execution-root authority; do not invent a second terminal-scope configuration

The repository already has a canonical operator-controlled filesystem authority: the execution root plus authorized workspace roots. Broad terminal behavior should reuse it.

For the requested single-owner profile:

```text
execution_root = $HOME
configured dir = $HOME
```

means the terminal may operate throughout the non-protected operator home. A deployment configured to a narrower execution root remains narrow automatically.

Do not add a redundant `terminal_scope=workspace|home` setting unless implementation proves the existing execution-root contract cannot safely express the required behavior.

### D-02 — “Broad terminal” means broad user-space authority, not unsandboxed host authority

Keep Bubblewrap. The target is:

```text
operator-selected execution root: writable according to existing permission mode
standard system runtime (/usr, /bin, /lib, /etc): read-only as currently modeled
/dev, /proc, /tmp: isolated sandbox instances
credentials: masked/denied
privilege brokers: denied/masked
credential/privileged sockets: absent unless separately authorized
```

Do not replace this with a direct host shell.

### D-03 — `$HOME` becomes a valid sandbox root only after credential masking is proven safe at home scale

Remove the unconditional `sandbox_root == host_home` rejection only together with:

- deterministic protected-path masking before command execution;
- bounded/fail-closed traversal behavior suitable for a large home tree;
- tests covering top-level and nested credentials;
- tests proving unrelated normal files under multiple home subdirectories remain usable;
- no host credential-agent socket exposure.

If protected-path discovery cannot complete safely within configured bounds, command execution must fail closed rather than running with incomplete masking.

### D-04 — Privilege denial requires defense in depth, not only top-level command validation

Keep top-level executable rejection, but add sandbox-level denial/masking so shell wrappers cannot reintroduce prohibited binaries.

Minimum protected privilege brokers:

```text
sudo
su
doas
pkexec
runas (where relevant)
```

Implementation must also verify the effective Bubblewrap child privilege model (capabilities / no-new-privileges / namespace behavior) and add regression coverage proving a model-selected shell cannot obtain greater host authority than the relay operator user.

Do not attempt to maintain an enormous denylist of every administrative command. The security boundary should come from sandbox isolation, absent privileged sockets, and lack of privilege escalation—not from guessing every command name.

### D-05 — Credential policy remains a hard security boundary

The existing canonical `protected_paths` module remains the source of truth. Do not create a second terminal-only secret list.

Preserve all current defenses:

- protected-path rejection in native workspace operations;
- Bubblewrap masking for terminal-visible trees;
- environment clearing;
- credential-shaped output redaction;
- generic SSH client masking;
- Git remote/forge credential isolation through dedicated native paths.

Add explicit tests for credential channels that are not ordinary files: SSH/GPG/keyring environment/socket surfaces must not become reachable merely because `$HOME` is mounted.

### D-06 — Dedicated MCP tools are preferred by policy; terminal is a fallback/general CLI tool

Use this selection order whenever the active tool set contains a suitable dedicated capability:

1. **Structured repository/workspace tools** for directory listing, search, reads, writes, edits, patches.
2. **Structured Git tools (`git_*`)** for Git operations they cover.
3. **Code-intelligence tools (`code_*`)** for symbols/definitions/references/hover/diagnostics/rename preview.
4. **Dedicated network/integration tools** such as `web_search`, `http_fetch`, forge/change-request tools, `ssh_readonly_exec`, and `telegram_send_message`.
5. **Terminal** for build/test/package-manager/interpreter/script/process/sandbox-local-service/composite-shell workflows, or when no active dedicated tool fully covers the required operation.

A dedicated tool should win when it provides equivalent semantics because it is more bounded, structured, observable, and policy-aware.

### D-07 — Tool selection uses the actual active tool set, not a stale hard-coded inventory

At each primary agent turn, build the selection guidance after tool composition from the same model-facing tool map passed to the AI SDK. This prevents instructions from recommending tools that are disabled or unavailable.

Normalize model-facing/scoped MCP names only for categorization. Preserve exact tool names in the generated guidance where useful so the model can select the tool it actually sees.

For subagents, derive equivalent guidance from their intersected authority / effective tool set so child instructions cannot claim authority the child does not have.

### D-08 — Do not add a separate “list tools first” model call

The model already receives tool schemas. “Check tools first” should mean:

- inspect the currently provided tool names/descriptions in model context;
- select a dedicated capability if it covers the task;
- use terminal only after determining no dedicated tool is sufficient.

Adding another MCP call just to echo the tool catalog would waste latency/tokens and could become stale relative to the actual per-turn filtered tool set.

### D-09 — Avoid a brittle terminal-level Git ban

Do not reject every `git` executable invocation in Rust policy. Reasons:

- clients may expose terminal without the entire Git tool family;
- a Git subcommand may exist that the dedicated surface intentionally does not cover;
- shell-native build/release scripts may legitimately invoke Git internally;
- parsing arbitrary shell text into exact MCP intent is brittle and easy to evade.

Instead, enforce dedicated-tool preference through request-scoped agent policy + client-visible tool descriptions, and keep hard Rust enforcement for real security boundaries (privilege, credentials, containment, dedicated SSH separation).

### D-10 — Do not broaden network or privileged bridges as a side effect

`allow_terminal_network`, Docker socket exposure, Tailscale socket exposure, and other privileged bridges remain separate operator decisions. Broad filesystem scope must not flip these settings.

### D-11 — Preserve approval semantics

`terminal_exec` / terminal jobs remain open-world/destructive-capable tools under the current effect/permission architecture. Broadening what the sandbox can see must not bypass manual approval modes or effect checks.

### D-12 — Catalog compatibility remains immutable

If any serialized tool description changes, follow the repository’s existing snapshot convention:

- retain historic v13 unchanged;
- publish the next catalog snapshot/version expected by current contract tests;
- prove the new current catalog is stable;
- do not silently rewrite a historic snapshot.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
| --- | --- | --- | --- |
| PHASE-00 | Freeze baseline and exact authority/tool contracts | none | Current Rust/Nuxt focused gates and catalog/tool-routing baseline are known; unrelated `agents/` remains untouched |
| PHASE-01 | Introduce canonical MCP-first tool-selection policy | 00 | Primary agent prompt is generated from actual active tools and deterministically prefers dedicated tools |
| PHASE-02 | Apply routing policy to delegated agents and MCP descriptions | 01 | Child agents receive equivalent authority-aware guidance; terminal catalog text clearly describes fallback role |
| PHASE-03 | Allow execution-root `$HOME` as a broad terminal sandbox | 00 | Normal commands work across multiple non-protected home paths while containment remains execution-root bounded |
| PHASE-04 | Harden privilege-escalation isolation | 03 | Direct and shell-wrapped privilege brokers are denied; no implicit root-equivalent bridge exists |
| PHASE-05 | Harden credential isolation for broad home mounts | 03 | Top-level/nested credentials and credential-agent channels remain unavailable with bounded fail-closed masking |
| PHASE-06 | Integrate terminal fallback ergonomics and operator docs | 01–05 | Dedicated-tool-first behavior and broad terminal operating contract are documented consistently |
| PHASE-07 | Cross-stack security/behavior validation | 01–06 | Focused Rust + Nuxt tests, catalog snapshot, and repository guardrails pass |
| PHASE-08 | Orphan sweep and closure | 07 | No stale contradictory guidance/code remains; plan can be truthfully marked closed |

# PHASE-00 — Baseline and contract freeze

**Goal:** Establish known-good behavior and exact current contracts before mutation.
**Dependencies:** none

## TASK-001 — Revalidate repository identity and isolate task-owned changes

**Outcome:** Implementation cannot contaminate another project or unrelated work.

**Files:**
- Read only: `ai-self/project.yaml`
- Read only: `.agents/plans/065-broad-user-terminal-and-mcp-first-tool-routing.md`

**Steps:**
- [ ] Re-run workspace verification from the current checkout before the first implementation mutation.
- [ ] Confirm origin still matches `ai-self/project.yaml`.
- [ ] Record branch/worktree status.
- [ ] Preserve the unrelated untracked `agents/` directory exactly as found; do not stage, delete, clean, or modify it.
- [ ] Use a dedicated task branch/worktree for implementation when repository delivery policy requires it.

**Validation:**
- `python ai-self/tools/workspace-verify` → `verified: true` and expected origin/root.
- Dedicated `git_status` tool → no unexpected tracked changes before implementation.

**Commit boundary:** none; baseline only.

## TASK-002 — Capture current terminal, catalog, and agent-routing behavior

**Outcome:** Later regressions can be distinguished from intended changes.

**Files:**
- Read: `packages/rust-tools/src/interfaces/mcp/catalog.rs`
- Read: `packages/rust-tools/src/core/terminal_policy.rs`
- Read: `packages/rust-tools/src/application/execution/requests.rs`
- Read: `packages/rust-tools/src/application/execution/sandbox.rs`
- Read: `packages/rust-tools/src/application/execution/sandbox/paths.rs`
- Read: `server/application/chat/execute-chat-turn.ts`
- Read: `server/application/chat/workspace-context.ts`
- Read: `server/application/subagents/runtime.ts`

**Steps:**
- [ ] Record current terminal tool description and current catalog snapshot version.
- [ ] Confirm current primary system prompt has no MCP-first routing guidance.
- [ ] Confirm current home-root sandbox rejection.
- [ ] Confirm current direct privilege-broker rejection and generic SSH sandbox masking.
- [ ] Run focused current Rust terminal/security tests and relevant server unit tests before mutation.

**Validation:**
- `cargo test -p ai-tools --test terminal_policy --locked`
- `cargo test -p ai-tools --test protected_paths --locked`
- `cargo test -p ai-tools --test security --locked`
- `pnpm test -- --runInBand` only if supported by the repository’s current test runner; otherwise use the repository-native focused unit-test command discovered from `package.json` for affected `test/unit` files.

**Commit boundary:** none; baseline only.

**Phase exit criteria:**
- [ ] Known-good baseline recorded.
- [ ] Current security/routing limitations reproduced.
- [ ] No implementation file modified.

# PHASE-01 — Canonical MCP-first tool-selection policy

**Goal:** Make primary coding-agent tool choice explicitly prefer dedicated active MCP capabilities before terminal fallback.
**Dependencies:** PHASE-00

## TASK-003 — Add a request-scoped tool-selection policy builder

**Outcome:** One application-layer helper converts the actual active model-facing tool map into concise deterministic selection guidance.

**Files:**
- Create: `server/application/chat/tool-selection-policy.ts`
- Test: `test/unit/chat-tool-selection-policy.test.ts`

**Steps:**
- [ ] Accept the actual active tool names for the turn; do not read a static global catalog.
- [ ] Normalize scoped/model-facing MCP names only enough to categorize known capability families while preserving exact names for output.
- [ ] Categorize at least:
  - workspace/file/search/edit/patch;
  - `git_*`;
  - `code_*`;
  - web/HTTP;
  - forge/change-request/issue/action/security tools;
  - `ssh_readonly_exec`;
  - `telegram_send_message`;
  - terminal and terminal-job tools.
- [ ] Generate guidance only for categories actually present in the active tool set.
- [ ] State the terminal fallback cases positively and narrowly: build, test, package manager, interpreter, script, process/sandbox-local-service inspection, composite shell workflow, or no equivalent dedicated tool.
- [ ] State that a dedicated tool should be used when it fully covers the requested operation even if terminal could also do it.
- [ ] State specifically that Git operations covered by active `git_*` tools should not be executed via terminal.
- [ ] Keep the generated prompt bounded and stable; do not dump full JSON schemas into the system prompt.

**Validation:**
- Unit test: active `git_status` + `terminal_exec` produces explicit Git-tool preference.
- Unit test: terminal-only tool set does not claim a nonexistent dedicated Git/file tool.
- Unit test: `file_read`/`text_search`/`code_*` categories appear only when present.
- Unit test: scoped MCP model-facing names are categorized without changing the exact name the model must call.
- Unit test: prompt length stays below a small deterministic bound chosen in implementation.

**Commit boundary:** `feat(agent): add active-tool selection policy`

## TASK-004 — Inject MCP-first policy into primary agent turns

**Outcome:** Every tool-capable primary coding-agent turn receives the selection policy matching its actual tools.

**Files:**
- Modify: `server/application/chat/execute-chat-turn.ts`
- Modify if composition ownership requires: `server/application/chat/workspace-context.ts`
- Test: `test/unit/chat-tool-selection-policy.test.ts`
- Extend relevant execution test if present: `test/unit/chat-entry-authority.test.ts`

**Steps:**
- [ ] Build tool-selection guidance after MCP/internal/subagent tool composition, when the final active tool keys are known.
- [ ] Append the guidance to the system prompt passed to `streamAiSdkAgent`.
- [ ] Do not inject terminal guidance for plain no-tool LangGraph chat.
- [ ] Preserve plan-mode and read-only-mode authority text; the routing policy must never imply mutation authority in plan/read-only mode.
- [ ] Keep tool authority and tool-choice policy separate: the prompt may recommend only tools actually present.
- [ ] Avoid changing tool approval maps, effects, or schemas in this task.

**Validation:**
- Primary agent with `git_status` + `terminal_exec` receives Git-first guidance.
- Primary agent with only read tools receives no mutation/terminal recommendation.
- No-tool chat path remains unchanged except for refactoring required to share prompt assembly.
- `pnpm guardrail:nuxt` passes after this phase.

**Commit boundary:** `feat(chat): prefer dedicated mcp tools before terminal`

**Phase exit criteria:**
- [ ] Primary tool turns use actual-tool-aware routing guidance.
- [ ] Plain chat and permission-mode semantics remain unchanged.
- [ ] Focused unit tests and Nuxt guardrail pass.

# PHASE-02 — Delegated-agent routing and client-visible tool guidance

**Goal:** Make the same dedicated-tool-first rule visible to child agents and external MCP clients without duplicating inconsistent policy.
**Dependencies:** PHASE-01

## TASK-005 — Reuse canonical routing policy for subagents

**Outcome:** Delegated agents receive tool-choice guidance derived from their effective intersected authority.

**Files:**
- Modify: `server/application/subagents/runtime.ts`
- Modify the subagent execution adapter that composes child system instructions, discovered from the current runtime wiring before mutation.
- Modify only if necessary: `.agents/agents/general-purpose.md`, `.agents/agents/explore.md`, `.agents/agents/plan.md`, `.agents/agents/review.md`, `.agents/agents/verify.md`
- Test: relevant existing subagent unit tests under `test/unit/`; add a focused feature-named test if no current file owns prompt/context policy.

**Steps:**
- [ ] Derive child routing guidance from `intersectSubagentAuthority(...)` / final effective child tool set, not parent authority.
- [ ] Reuse the same conceptual capability ordering as primary chat.
- [ ] Do not recommend terminal to profiles that do not actually receive terminal.
- [ ] Prefer a shared application helper over copying long prose into every agent profile.
- [ ] Keep profile-specific task instructions intact.
- [ ] Preserve max context/token bounds; account for the added bounded routing text.

**Validation:**
- Child with Git tools + terminal receives Git-first guidance.
- Child without terminal receives no terminal fallback language.
- Child cannot gain a tool merely because the routing policy mentions a capability category.
- Existing subagent authority/intersection tests remain green.

**Commit boundary:** `feat(subagents): inherit mcp-first tool routing`

## TASK-006 — Reframe terminal tool descriptions as fallback/general CLI

**Outcome:** Any MCP client that consumes the relay catalog sees a terminal description that encourages dedicated first-class tools rather than competing with them.

**Files:**
- Modify: `packages/rust-tools/src/interfaces/mcp/catalog.rs`
- Test: `packages/rust-tools/tests/ssh_catalog.rs` or the repository’s current catalog snapshot owner discovered during implementation.

**Steps:**
- [ ] Change `terminal_exec` description to say it is for shell/CLI workflows when no dedicated MCP tool fully covers the operation.
- [ ] Remove the current wording that advertises Git as a normal generic terminal use case.
- [ ] Add explicit examples that remain terminal-native: builds, tests, package managers, interpreters, scripts, process/sandbox-local-service commands.
- [ ] Apply equivalent fallback wording to `terminal_job_start` so long-running terminal jobs follow the same policy.
- [ ] Keep schema and annotations unchanged unless a separately justified implementation need arises.
- [ ] If serialized catalog output changed, create the next immutable snapshot according to existing test convention; keep v13 byte-for-byte historical.

**Validation:**
- Catalog test proves the current terminal descriptions contain fallback/dedicated-tool-first language.
- Historic v13 snapshot remains unchanged.
- New/current catalog snapshot test passes.

**Commit boundary:** `docs(mcp): mark terminal as dedicated-tool fallback`

**Phase exit criteria:**
- [ ] Primary and delegated agents share one routing rule.
- [ ] External MCP clients receive matching terminal guidance from the catalog.
- [ ] No dedicated capability is hidden or removed.

# PHASE-03 — Broad execution-root terminal sandbox

**Goal:** Allow terminal commands to operate across the configured execution root, including `$HOME`, without losing sandbox isolation.
**Dependencies:** PHASE-00

## TASK-007 — Support host home as an intentional sandbox root

**Outcome:** `$HOME` may be mounted as the writable terminal root when it is also the authorized execution root/configured root.

**Files:**
- Modify: `packages/rust-tools/src/application/execution/sandbox.rs`
- Modify if mount ordering needs it: `packages/rust-tools/src/application/execution/sandbox/paths.rs`
- Modify if cwd authority requires a narrow correction: `packages/rust-tools/src/application/execution/paths.rs`
- Test: `packages/rust-tools/tests/security.rs`
- Test: `packages/rust-tools/tests/relay_config.rs`

**Steps:**
- [ ] Replace the unconditional `sandbox_root == host_home` failure with an explicit safe home-root path.
- [ ] Require the selected home root to still be inside the configured execution-root/authorized-root authority; do not treat `$HOME` as implicit authority when the operator configured a narrower root.
- [ ] Preserve read-only standard system mounts, isolated `/dev`, `/proc`, `/tmp`, PID namespace, and process lifecycle controls.
- [ ] Ensure default `cwd` and explicit `cwd` both work when they resolve to allowed non-protected paths under home.
- [ ] Preserve authorized sibling behavior; do not make sibling repository access broader than current config authorizes.
- [ ] Preserve native protected-target rejection when `cwd` itself is a protected credential path.
- [ ] Do not expose `/run/user`, host `/tmp`, host `/proc`, or arbitrary host root mounts as part of the home-root change.

**Validation:**
- Terminal command succeeds from two or more ordinary directories under the configured home execution root.
- Terminal command can read/write a normal task-owned file under a non-protected home subdirectory when permission mode permits mutation.
- A deployment configured to a project execution root still rejects cwd traversal to another home directory/repository outside authority.
- Cwd directly inside `.ssh`, `.config/gh`, `.aws`, `.env` family, or another canonical protected path is rejected.
- `pnpm guardrail:rust` passes after the phase.

**Commit boundary:** `feat(relay): allow protected home-scoped terminal sandbox`

## TASK-008 — Preserve terminal-native workflow ergonomics

**Outcome:** Broad terminal supports practical user workflows without weakening safe executable resolution unnecessarily.

**Files:**
- Review/modify only if required: `packages/rust-tools/src/core/terminal_policy.rs`
- Review/modify only if required: `packages/rust-tools/src/application/execution/toolchain.rs`
- Modify docs later in PHASE-06 rather than duplicating documentation here.

**Steps:**
- [ ] Verify system tools in the relay safe PATH remain available from home-root execution.
- [ ] Verify configured toolchain paths still work when the sandbox root is home and when toolchain paths themselves are symlink-based (for example fnm-managed Node).
- [ ] Keep explicit reviewed toolchain-path behavior rather than inheriting the full login-shell PATH.
- [ ] Keep repository scripts executable through approved interpreters (`bash`, `python`, `node`, etc.) even if direct executable paths remain intentionally restricted.
- [ ] Do not loosen executable path rules merely to mimic an interactive shell unless a concrete workflow remains impossible and the alternative is reviewed against protected paths/setuid binaries.

**Validation:**
- Representative Rust command works from a Rust project under home.
- Representative Node/package-manager command works from a Node project under home using configured toolchain paths.
- Interpreter-invoked repository script works.
- Safe PATH remains deterministic and does not include credential/config directories.

**Commit boundary:** combine with TASK-007 unless an independently reviewable executable-resolution change is actually needed.

**Phase exit criteria:**
- [ ] `$HOME` execution root is usable for ordinary user-space terminal work.
- [ ] Narrow deployments remain narrow.
- [ ] Runtime/toolchain behavior remains deterministic.

# PHASE-04 — Privilege-escalation hardening

**Goal:** Guarantee that broad terminal authority cannot turn into elevated host authority, including through shell wrapping.
**Dependencies:** PHASE-03

## TASK-009 — Mask privilege-broker executables inside Bubblewrap

**Outcome:** Shell-wrapped execution cannot bypass direct `validate_executable` checks.

**Files:**
- Modify: `packages/rust-tools/src/application/execution/sandbox.rs`
- Modify: `packages/rust-tools/src/core/terminal_policy.rs` only if canonical denied-name ownership needs refactoring to avoid duplicate lists.
- Test: `packages/rust-tools/tests/terminal_policy.rs`
- Test: `packages/rust-tools/tests/security.rs`

**Steps:**
- [ ] Create one canonical privilege-broker name/path policy or a small shared helper so direct validation and sandbox masking cannot drift.
- [ ] Mask resolved system binaries for `sudo`, `su`, `doas`, `pkexec`, and platform-relevant `runas` equivalents visible in the sandbox.
- [ ] Handle common `/usr/bin` vs `/bin` aliases/canonicalization without double-binding conflicting paths.
- [ ] Fail closed on unsafe symlink/path metadata for protected broker binaries using the same defensive style as SSH masking.
- [ ] Keep generic SSH masking separate semantically even if implementation shares a generic “mask forbidden executable” helper.

**Validation:**
- Direct `terminal_exec(command="sudo", ...)` is rejected.
- `sh -lc "sudo ..."`, `bash -lc "su ..."`, and equivalent wrappers cannot execute the broker binary.
- Allowed ordinary shell commands still work.
- Generic `ssh`/`scp`/`sftp` behavior remains unchanged and still directs callers to `ssh_readonly_exec`.

**Commit boundary:** `fix(relay): enforce privilege denial inside sandbox`

## TASK-010 — Verify sandbox privilege model and privileged-bridge isolation

**Outcome:** Security does not depend only on command-name masking.

**Files:**
- Modify if needed: `packages/rust-tools/src/application/execution/sandbox.rs`
- Modify if needed: `packages/rust-tools/src/application/execution/sandbox/paths.rs`
- Test: `packages/rust-tools/tests/security.rs`
- Test: `packages/rust-tools/tests/relay_config.rs`

**Steps:**
- [ ] Verify the Bubblewrap child cannot acquire additional host capabilities/privileges through setuid or namespace behavior; if current invocation does not make that guarantee explicit enough, add the smallest supported sandbox flag/configuration that does.
- [ ] Confirm host system D-Bus, keyring, container, and other privilege-bearing runtime sockets are not mounted by default.
- [ ] Confirm `allow_docker` remains an explicit high-risk operator opt-in and is not implied by broad terminal scope.
- [ ] Confirm `allow_tailscale` remains independent.
- [ ] Add regression coverage around socket visibility/config toggles rather than relying solely on docs.

**Validation:**
- Default broad-home terminal has no Docker/Tailscale socket unless explicitly configured.
- Explicit existing socket opt-in behavior still works according to current tests/policy.
- Child reports only operator-user-equivalent effective authority; no test demonstrates host-root elevation.

**Commit boundary:** `fix(relay): preserve least privilege in broad terminal sandbox`

**Phase exit criteria:**
- [ ] Direct and nested-shell privilege escalation routes are closed.
- [ ] Root-equivalent bridges are not implicitly exposed.
- [ ] Security regression tests pass.

# PHASE-05 — Credential isolation at home scale

**Goal:** Preserve the original “credentials stay NO” rule when the terminal can see the broader operator home.
**Dependencies:** PHASE-03

## TASK-011 — Make protected-path masking safe and scalable for a home-root mount

**Outcome:** Broad-home execution never runs with incomplete credential masking and does not become impractically fragile on normal development homes.

**Files:**
- Modify: `packages/rust-tools/src/application/execution/sandbox.rs`
- Modify only if canonical policy expansion is justified: `packages/rust-tools/src/core/protected_paths.rs`
- Test: `packages/rust-tools/tests/protected_paths.rs`
- Test: `packages/rust-tools/tests/security.rs`

**Steps:**
- [ ] Reuse `core::protected_paths` as the only path-name policy.
- [ ] Review `discover_protected_paths` for `$HOME` scale, including its 500k-entry bound and unscanned build/cache directories.
- [ ] Keep traversal bounded and fail closed if masking cannot be completed.
- [ ] Ensure all top-level credential stores are masked even when recursive traversal skips heavy directories.
- [ ] Ensure nested `.env`, `.env.local`, `.env.production`, etc. remain masked in project trees while `.env.example` remains visible according to current policy.
- [ ] Ensure protected symbolic-link edge cases fail closed rather than following links outside the expected tree.
- [ ] Avoid adding wildcard/broad filesystem exceptions that could make credential detection silently incomplete.
- [ ] If home-scale performance requires optimization, prefer deterministic pruning/indexing inside existing code over a new daemon/database/cache dependency.

**Validation:**
- Shell cannot read representative fixtures at `~/.ssh/*`, `~/.config/gh/*`, `~/.aws/*`, `~/.docker/*`, `~/.kube/*`, `~/.npmrc`, `~/.netrc`, Cargo credential files, or nested `.env*` secret fixtures.
- `.env.example` remains readable where current policy intends.
- A normal file adjacent to protected material remains readable/writable.
- Protected symlink fixture fails closed.
- Large synthetic home-tree fixture stays within deterministic test bounds and demonstrates either successful masking or explicit fail-closed behavior—never silent partial exposure.

**Commit boundary:** `fix(relay): preserve credential masks across home scope`

## TASK-012 — Preserve environment and credential-agent isolation

**Outcome:** Broad filesystem visibility does not reintroduce secrets through process environment or sockets.

**Files:**
- Review/modify if needed: `packages/rust-tools/src/application/execution/sandbox.rs`
- Test: `packages/rust-tools/tests/security.rs`
- Test: `packages/rust-tools/tests/redaction.rs` if output-redaction behavior is touched.

**Steps:**
- [ ] Preserve `env_clear()` as mandatory for terminal children.
- [ ] Keep only the minimal reconstructed environment required for toolchain/runtime behavior.
- [ ] Confirm common credential environment variables are absent from the child unless they are non-secret runtime paths deliberately set by the relay.
- [ ] Confirm host SSH/GPG/keyring agent sockets are not mounted/inherited.
- [ ] Preserve output credential redaction for stdout/stderr and asynchronous terminal job retention.
- [ ] Do not expose browser/session/token stores to make CLI authentication “convenient”; authenticated Git/forge/SSH continue through dedicated credential-isolated tools.

**Validation:**
- Test fixture seeds credential-shaped environment variables in the relay parent; terminal child cannot read them.
- `SSH_AUTH_SOCK`/GPG/keyring-style host sockets are absent unless an existing explicitly reviewed capability owns them.
- Terminal job output remains redacted using the same canonical redaction path.

**Commit boundary:** combine with TASK-011 unless code changes are independently reviewable.

**Phase exit criteria:**
- [ ] File, environment, and socket credential channels remain denied.
- [ ] Authenticated remote operations still rely on dedicated safe tools rather than terminal credential exposure.
- [ ] Credential regression matrix passes.

# PHASE-06 — Terminal fallback ergonomics and documentation

**Goal:** Make the operating model obvious to agents and operators: dedicated MCP first, broad terminal second, credentials/privilege never.
**Dependencies:** PHASE-01 through PHASE-05

## TASK-013 — Document the canonical tool-selection hierarchy

**Outcome:** Repository agent guidance matches runtime behavior.

**Files:**
- Modify: `.agents/knowledge/tooling.md`
- Modify if referenced by active agent guidance: `.agents/README.md`

**Steps:**
- [ ] Add a concise “MCP-first tool selection” rule.
- [ ] Give explicit examples:
  - `git_status` instead of `terminal_exec(git status)` when available;
  - `git_diff` instead of terminal Git diff;
  - `file_read`/`text_search` instead of `cat`/`rg` when the structured tool covers the need;
  - `code_definition`/`code_references` instead of grep-based semantic guessing;
  - `ssh_readonly_exec` instead of generic SSH;
  - terminal for `cargo test`, `pnpm`, interpreters, scripts, and other terminal-native workflows that do not require a host privileged/session bridge.
- [ ] Clarify that terminal fallback is allowed when the active dedicated surface does not cover the operation.
- [ ] Clarify that using a dedicated tool is a routing preference, while privilege/credential boundaries are hard enforcement.

**Validation:**
- `pnpm guardrail` agent-doc integrity checks pass.
- No active guidance still tells agents to prefer terminal Git when native Git tools exist.

**Commit boundary:** `docs(agent): define mcp-first terminal fallback policy`

## TASK-014 — Update operator/security documentation for broad home execution

**Outcome:** Operators understand how to enable broad user-space scope without accidentally weakening secret/privilege boundaries.

**Files:**
- Modify: `docs/configuration.md`
- Modify: `docs/security.md`
- Modify: `docs/troubleshooting.md`
- Modify if relevant after current review: `docs/development.md`
- Modify: `.agents/knowledge/tooling.md`

**Steps:**
- [ ] Explain that execution root remains the filesystem authority; `$HOME` is appropriate for a single-owner broad coding relay, while project roots preserve narrower deployments.
- [ ] State that Bubblewrap remains mandatory and credentials remain masked.
- [ ] State that `sudo`/privilege brokers remain unavailable even under `$HOME` scope.
- [ ] State that generic SSH remains unavailable and dedicated SSH diagnostics own credential use.
- [ ] State that Docker access is separate/root-equivalent authority and should stay disabled unless intentionally needed.
- [ ] Explain terminal network remains governed by its existing separate operator setting.
- [ ] Document safe examples and expected failures without including real credentials or machine-specific secret paths.
- [ ] Keep live systemd/restart instructions separate; do not perform them as part of implementation closure.

**Validation:**
- Documentation matches actual CLI/config names discovered in current source.
- No docs claim broad terminal inherits the login environment or credentials.
- No docs imply `$HOME` scope overrides protected paths.

**Commit boundary:** `docs(relay): explain broad user-space terminal security model`

**Phase exit criteria:**
- [ ] Agent and operator docs describe one consistent authority/routing model.
- [ ] Examples distinguish dedicated MCP from terminal fallback clearly.

# PHASE-07 — Cross-stack security and behavior validation

**Goal:** Prove the requested capability works without regressing hard boundaries.
**Dependencies:** PHASE-01 through PHASE-06

## TASK-015 — Run focused Rust acceptance matrix

**Outcome:** Terminal behavior is validated at the actual relay boundary.

**Files:**
- Test owners: `packages/rust-tools/tests/terminal_policy.rs`
- Test owners: `packages/rust-tools/tests/protected_paths.rs`
- Test owners: `packages/rust-tools/tests/security.rs`
- Test owners: `packages/rust-tools/tests/relay_config.rs`
- Catalog owner: `packages/rust-tools/tests/ssh_catalog.rs` or current catalog-contract replacement if repository ownership changed during implementation.

**Required behavior matrix:**

| Case | Expected |
| --- | --- |
| cwd in repo A under `$HOME` | allowed |
| cwd in ordinary sibling repo B under authorized `$HOME` execution root | allowed |
| cwd outside configured execution root | denied |
| read/write ordinary non-protected user file | allowed subject to permission mode |
| read `.ssh` / `.aws` / `.config/gh` / `.docker` / `.kube` | denied/masked |
| read nested `.env.local` | denied/masked |
| read `.env.example` | allowed per existing policy |
| parent environment contains secret token | child cannot observe it |
| direct `sudo`/`su`/`doas`/`pkexec` | denied |
| `sh -lc` wrapper around forbidden privilege broker | denied/unavailable |
| generic `ssh`/`scp`/`sftp` | denied; dedicated SSH remains available separately |
| Docker socket with default config | absent |
| terminal networking default/configured behavior | unchanged from explicit config |
| normal build/package/interpreter command | works |

**Validation:**
- `cargo test -p ai-tools --test terminal_policy --locked`
- `cargo test -p ai-tools --test protected_paths --locked`
- `cargo test -p ai-tools --test security --locked`
- `cargo test -p ai-tools --test relay_config --locked`
- Current catalog snapshot test.
- `pnpm guardrail:rust`

**Commit boundary:** tests should normally land with their owning implementation commits; no test-only cleanup commit unless necessary.

## TASK-016 — Run focused agent-routing acceptance matrix

**Outcome:** Dedicated tools are deterministically preferred in generated instructions without lying about unavailable capabilities.

**Files:**
- Test: `test/unit/chat-tool-selection-policy.test.ts`
- Relevant existing chat/subagent tests discovered during implementation.

**Required behavior matrix:**

| Active tools | Expected routing guidance |
| --- | --- |
| `git_status`, `git_diff`, `terminal_exec` | prefer `git_*`; terminal fallback only |
| `file_read`, `text_search`, `terminal_exec` | prefer structured file/search |
| `code_definition`, `code_references`, `terminal_exec` | prefer code intelligence for semantic navigation |
| `ssh_readonly_exec`, `terminal_exec` | use dedicated SSH, never generic terminal SSH |
| only `terminal_exec` | terminal allowed; do not claim missing dedicated tool |
| no terminal | no terminal-fallback recommendation |
| read-only/plan authority | no mutation guidance regardless of available underlying catalog |
| child tool intersection removes Git/terminal | child prompt mentions neither unavailable capability |

**Validation:**
- Repository-native focused unit test command for the changed test files.
- `pnpm guardrail:nuxt`

**Commit boundary:** tests land with PHASE-01/02 implementation commits.

## TASK-017 — Run final repository gates and manual smoke checks

**Outcome:** Both stacks and cross-stack documentation/contracts are green.

**Steps:**
- [ ] Run `pnpm guardrail` because both Nuxt/server and Rust plus agent docs changed.
- [ ] Run Rust warnings-denied/checks if not already covered by the current guardrail implementation.
- [ ] Build the release `ai-tools` binary if security-sensitive sandbox code changed.
- [ ] Run a local non-production relay smoke profile only if the repository has an existing safe local fixture/harness; do not restart the operator’s systemd relay.
- [ ] Smoke a representative MCP-first Git flow from an agent test/harness if deterministic local model/tool-selection testing exists; otherwise rely on deterministic prompt/catalog contract tests and record that model stochastic behavior is advisory rather than a hard security property.
- [ ] Confirm activity/error output does not leak protected paths or credential values.

**Validation:**
- `pnpm guardrail`
- `cargo build --release --locked --bin ai-tools`
- `target/release/ai-tools --version`
- Current MCP catalog snapshot/size/hash acceptance according to repository convention.

**Phase exit criteria:**
- [ ] Rust security matrix passes.
- [ ] Agent-routing matrix passes.
- [ ] Final guardrail passes.
- [ ] No production/service mutation performed.

# PHASE-08 — Orphan sweep and closure

**Goal:** Remove stale contradictory code/guidance and leave one clear authority model.
**Dependencies:** PHASE-07

## TASK-018 — Sweep stale terminal/tool-routing assumptions

**Outcome:** Active source/docs no longer contradict Plan 065.

**Steps:**
- [ ] Search active source/docs for the old assumption that terminal is workspace-only because `$HOME` cannot be mounted.
- [ ] Search active agent guidance/catalog text for generic “Git through terminal” recommendations that conflict with MCP-first routing.
- [ ] Search for duplicated privilege-broker lists introduced during implementation and consolidate to one owner where practical.
- [ ] Search for duplicated credential-path lists; keep `core::protected_paths` canonical.
- [ ] Search for accidental exposure of host runtime/credential sockets.
- [ ] Confirm unrelated `agents/` remains untouched.

**Validation:**
- Repository-native `text_search` queries show no active contradictory guidance except historical plans/evidence intentionally preserved.
- `git_diff` contains only task-owned changes.
- `pnpm guardrail` remains green after cleanup.

**Commit boundary:** `refactor(relay): close broad terminal routing cleanup` only if real cleanup changes remain after logical feature commits.

## TASK-019 — Truthful plan closure and execution handoff

**Outcome:** Plan status reflects verified implementation rather than intent.

**Files:**
- Modify: `.agents/plans/065-broad-user-terminal-and-mcp-first-tool-routing.md`

**Steps:**
- [ ] Mark task/phase checkboxes complete only with evidence.
- [ ] Record focused validation commands and final guardrail result.
- [ ] Record any intentionally deferred operator action, such as updating/restarting the live systemd relay, as not performed.
- [ ] Record final catalog version if a new immutable snapshot was required.
- [ ] Follow repository delivery policy for commit/push only when implementation execution is explicitly requested; plan creation itself does not imply implementation or deployment.

**Validation:**
- Plan status accurately matches repository state.
- No unresolved acceptance item is marked complete.

**Commit boundary:** include plan closure with the final task-owned implementation commit or a small docs-only closure commit according to repository convention.

**Phase exit criteria:**
- [ ] No active contradiction/orphan remains.
- [ ] All acceptance criteria below are evidenced.
- [ ] Plan can be marked `CLOSED / VERIFIED` only after implementation and tests actually complete.

## Risks & Rollback

### R-01 — Home-root protected-path scan becomes too expensive

**Risk:** Recursively scanning the whole developer home may exceed the existing bounded entry budget or add unacceptable startup-per-command latency.

**Mitigation:**
- retain aggressive safe pruning for dependency/build/cache directories;
- mask canonical top-level credential stores deterministically;
- keep traversal bounded/fail-closed;
- optimize the existing in-process discovery algorithm before considering any new dependency or persistent cache;
- add a synthetic large-home regression test.

**Rollback:** Restore project-root sandbox selection while keeping privilege/tool-routing improvements; never “solve” performance by skipping credential masks.

### R-02 — Removing the home-root guard accidentally exposes credentials

**Risk:** A bind of `$HOME` happens before all protected paths are masked.

**Mitigation:** Treat complete mask construction as a precondition to spawn; any mask/discovery error aborts execution.

**Rollback:** Re-enable the home-root rejection immediately if any credential isolation test fails.

### R-03 — Shell wrapping bypasses command deny policy

**Risk:** `sh -lc` can resolve forbidden binaries inside `/usr/bin` even when top-level command validation denies their names.

**Mitigation:** Sandbox-level executable masking + privilege-model verification.

**Rollback:** Keep broad-home disabled until nested-shell privilege tests pass.

### R-04 — MCP-first prompt becomes stale or recommends unavailable tools

**Risk:** A hard-coded tool inventory diverges from per-user/per-mode tool composition.

**Mitigation:** Generate guidance only from the actual final active tool keys for that turn and from intersected child authority.

**Rollback:** Disable the dynamic policy injection rather than shipping misleading instructions; catalog description improvement can remain independently.

### R-05 — Over-aggressive Git routing blocks legitimate workflows

**Risk:** Hard terminal bans break uncovered Git subcommands or scripts.

**Mitigation:** Keep Git-first behavior as routing policy/description, not a blanket Rust terminal deny. Security-deny only privilege/credentials/SSH boundaries.

**Rollback:** Relax routing prose for the specific uncovered workflow while adding/expanding a dedicated Git tool only when there is a recurring justified capability gap.

### R-06 — Broad terminal unintentionally expands network/privileged socket authority

**Risk:** Home scope is conflated with full host authority.

**Mitigation:** Keep terminal networking and optional socket mounts on existing separate config paths; add regression tests proving defaults do not change.

**Rollback:** Revert the offending mount/config change while retaining home filesystem support.

### R-07 — Catalog-description change breaks immutable snapshot expectations

**Risk:** Current snapshot test treats descriptions as immutable catalog content.

**Mitigation:** Preserve v13 and publish the next snapshot per existing convention.

**Rollback:** Restore the old current description until the new snapshot is correctly created; never rewrite v13.

## Final Acceptance Criteria

- [x] Relay configured with `$HOME` execution root can execute ordinary commands from multiple non-protected home paths.
- [x] A narrower execution root remains strictly contained.
- [x] Broad terminal still runs inside Bubblewrap; no direct host-shell fallback exists.
- [x] Standard system runtime remains read-only and sandbox `/dev`, `/proc`, `/tmp` remain isolated.
- [x] `sudo`, `su`, `doas`, `pkexec`, and equivalent configured privilege brokers are denied directly.
- [x] Shell-wrapped attempts to run those privilege brokers are also denied/unavailable.
- [x] Docker/Tailscale/other privileged sockets are not implicitly enabled.
- [x] Generic SSH clients remain unavailable through terminal; `ssh_readonly_exec` remains the dedicated path.
- [x] Existing protected credential paths remain unreadable/unwritable from terminal.
- [x] Nested `.env*` secrets remain masked while `.env.example` keeps its existing non-secret exception.
- [x] Parent-process secret environment variables are not inherited by terminal children.
- [x] Host SSH/GPG/keyring credential sockets are not exposed by broad home mounting.
- [x] Terminal output/job retention continues credential redaction.
- [x] Primary agent tool-selection guidance is built from actual active tools.
- [x] Dedicated `git_*` tools are explicitly preferred over terminal Git for covered operations.
- [x] Structured file/search/edit/code tools are preferred over shell equivalents when they cover the task.
- [x] Dedicated HTTP/forge/SSH/Telegram capabilities are preferred when present and suitable.
- [x] Host `systemctl --user` / `journalctl --user` operations are intentionally excluded from generic terminal execution by operator decision; no host user D-Bus or journal bridge is mounted.
- [x] Terminal remains available for builds, tests, package managers, interpreters, scripts, composite shell workflows, and genuinely uncovered operations.
- [x] Delegated agents receive the same principle derived from effective child authority.
- [x] No prompt recommends a tool unavailable to that turn/child.
- [x] Terminal catalog descriptions communicate fallback/general CLI semantics.
- [x] Historic catalog v13 remains immutable; v14 was added.
- [x] Existing plan/read-only/manual approval/effect semantics remain intact.
- [x] Focused Rust security tests pass.
- [x] Focused agent-routing unit tests pass.
- [x] `pnpm guardrail:rust` and `pnpm guardrail:nuxt` pass in their affected stages.
- [x] Final `pnpm guardrail` passes.
- [x] Release binary builds after sandbox/security changes (`cargo build --release --locked --bin ai-tools`; `ai-tools 0.0.14`).
- [x] No unrelated `agents/` change is staged or modified.
- [x] No live systemd restart/deployment/release action occurs without a separate explicit user request/approval.

## Execution evidence — 2026-09-06

- Baseline: `main` / `4d2afa96177918782813d9022c98d2c5ca3b40c5`, expected origin and `ai-self/project.yaml` verified; the pre-existing untracked main-checkout `agents/` directory was neither touched nor staged. Implementation is in isolated `feat/065-terminal-mcp-routing` worktree.
- Routing: `server/application/chat/tool-selection-policy.ts` derives bounded recommendations from final model-facing tool keys. Primary injection occurs after tool composition; child injection occurs after authority/profile filtering and counts against child context. `test/unit/chat-tool-selection-policy.test.ts` covers exact/scoped keys, absent capabilities, read-only turns, child intersection, bounded hostile inventories, and approval preservation.
- Terminal security: the HOME fixture covers multiple ordinary directories, narrow-root rejection, protected credentials and nested `.env.*`, `.env.example`, browser/keyring/relay-state masking, scrubbed environment, sockets, privilege capabilities, direct and wrapped broker/SSH denial, optional Docker/Tailscale grants, protected-symlink failure, full cache/tree discovery and a 10,000-entry synthetic build tree. It also proves Cargo, Node through a symlinked reviewed path, npm, an interpreter, and terminal Git fallback; the immutable catalog contract continues to cover dedicated `git_status` availability.
- Catalog: v13 SHA-256 remains `606f16cab046283c77b7c5bf773c2dbfa51cf62d6488b63855705392e25a479e`; current v14 snapshot is `.agents/contracts/065-tool-catalog-v14.json` and changes only terminal descriptions.
- Validation passed: focused routing tests; focused Rust security/protected-path/catalog/policy tests; `pnpm guardrail:nuxt`; `pnpm guardrail:rust`; final `pnpm guardrail`.
- Runtime intentionally not performed: no systemd restart/reload, binary install, deployment, release/tag, or live relay verification. Release build passed: `cargo build --release --locked --bin ai-tools`; `target/release/ai-tools --version` reported `ai-tools 0.0.14`.

## Execution Handoff

Recommended execution order:

1. PHASE-00 baseline.
2. PHASE-01 and PHASE-03 may proceed in parallel after baseline because tool-routing and Rust sandbox work are independent.
3. PHASE-02 follows PHASE-01.
4. PHASE-04 and PHASE-05 follow PHASE-03 and may proceed in parallel if changes to `sandbox.rs` are coordinated to avoid conflicting edits.
5. PHASE-06 follows both routing and sandbox/security behavior so docs describe final truth.
6. PHASE-07 runs focused stack validation first, then final cross-stack guardrail.
7. PHASE-08 performs orphan sweep and truthful closure.

Implementation guardrails:

- Prefer dedicated MCP tools during execution of this plan itself: use `git_status`/`git_diff`/`git_*`, `file_read`/`file_edit`/`apply_patch`, `text_search`, and `code_*` before `terminal_exec` when they fully cover the operation.
- Use terminal for actual build/test/package-manager/script commands or when no dedicated MCP tool exists.
- Every terminal call must use the explicit task working directory.
- Do not access or expose real credentials while testing; use fixtures/canaries only.
- Do not use `sudo` or seek an alternate privilege path if the relay boundary blocks an operation.
- Do not restart the live relay service during implementation unless separately requested after code closure.
- Keep task-owned changes isolated from the pre-existing untracked `agents/` directory.
