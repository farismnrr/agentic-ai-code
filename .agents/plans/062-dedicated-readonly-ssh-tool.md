# Plan 062 — Dedicated Read-Only SSH MCP Tool

**Status:** PLANNED / NOT IMPLEMENTED
**Goal:** Promote the Plan 061 read-only SSH backend into a first-class, client-portable MCP tool named `ssh_readonly_exec`, while removing the hidden SSH special-case from `terminal_exec` so every MCP client receives one explicit, self-describing SSH capability with one authoritative Rust security path.
**Success Criteria:** Any conforming MCP client (Nuxt, OpenWebUI, ChatGPT, or another client) can discover `ssh_readonly_exec` through ordinary `tools/list` without client-specific SSH logic; calls use a bounded schema such as `{ alias, command, args, timeout_ms, execution_mode }`; the relay retains the Plan 061 sanitized SSH-config parser, exact credential mounts, pinned system OpenSSH, strict host-key and non-interactive key-only authentication, semantic remote read-only policy, database/Docker hardening, output bounds, redaction, activity, and job lifecycle; `terminal_exec` no longer contains SSH-specific routing/effects/idempotency/activity behavior and cannot be used as an alternate SSH execution path; focused Rust contract/security tests and `pnpm guardrail` pass; implementation stops at local commit unless push/PR/merge/deployment is separately authorized; no systemd relay action occurs during implementation.

## Why this follow-up exists

Plan 061 correctly implemented the difficult security boundary, but exposed it as an input-sensitive special case of generic `terminal_exec`. That leaks semantic knowledge into clients: a frontend that pre-classifies generic terminal effects can remove `terminal_exec` before the model ever has a chance to issue a safe SSH call. The result is client-specific integration pressure and inconsistent discovery across Nuxt/OpenWebUI/other MCP consumers.

Plan 062 changes the public shape, not the security authority. The Rust relay remains the only component that resolves SSH aliases, reads reviewed SSH material, decides whether a remote operation is safe, and constructs OpenSSH argv.

## Architectural decision

Use one dedicated MCP tool:

```text
ssh_readonly_exec
```

Representative input:

```json
{
  "alias": "smart-meeting",
  "command": "docker",
  "args": ["ps"],
  "timeout_ms": 30000,
  "execution_mode": "auto"
}
```

The model/client receives no raw SSH-option surface. It cannot submit `-F`, `-o`, `-i`, `-p`, `-L`, `-R`, `-J`, `ProxyCommand`, agent options, known-host overrides, config paths, identity paths, or arbitrary OpenSSH flags. Connectivity comes only from the relay-owned normalized alias specification.

### Layer ownership

| Layer | Responsibility |
| --- | --- |
| MCP relay catalog | Advertise one explicit read-only SSH capability with truthful static annotations |
| Rust application/core | Resolve alias, validate remote command, enforce auth/config/credential/network policy, spawn/cancel/bound output |
| MCP clients | Discover and call the tool; no SSH parsing, effect inference, `.ssh` access, or client-specific policy knowledge required |
| OpenSSH | Transport only, using relay-owned argv after alias/config normalization |

### Canonical-path rule

`ssh_readonly_exec` becomes the **only** model-facing SSH path.

- Remove Plan 061 SSH detection/routing from `terminal_exec`.
- Restore ordinary `terminal_exec` effect/idempotency/activity semantics.
- Explicitly reject direct `ssh`/alternate SSH client execution through generic terminal policy with a stable message directing callers to `ssh_readonly_exec`. This prevents two policy surfaces from drifting and avoids a client accidentally bypassing the dedicated contract when ordinary terminal networking is enabled.
- Do not add any Nuxt/OpenWebUI-specific exception for SSH.

## Scope

### In scope

- New Rust MCP tool `ssh_readonly_exec` in the canonical tool catalog.
- Static, portable MCP annotations/effects for the dedicated capability.
- Primary and Full profile exposure unless a fresh catalog review finds a concrete reason to restrict one profile.
- Optional MCP Tasks support reusing the existing bounded job manager.
- Reuse/refactor of Plan 061 SSH config, remote policy, credential sandbox, authentication normalization, redaction, and runtime code.
- Removal of all SSH-specific `terminal_exec` detection, dispatch, effect classification, idempotency exceptions, and activity presentation.
- Generic-terminal rejection of direct SSH executables so the dedicated tool is canonical.
- Focused core/application/infrastructure/interface tests.
- Operator/client documentation and Plan 061/062 memory reconciliation where existing wording becomes stale.

### Out of scope

- Any client-specific Nuxt/OpenWebUI SSH implementation or prompt hack.
- A second raw SSH tool accepting OpenSSH flags or command strings.
- Interactive SSH, SCP/SFTP/rsync/SSHFS, tunneling, forwarding, agent use, ProxyJump, ProxyCommand, local commands, PTY, password guessing, or passphrase prompting.
- Expanding the Plan 061 remote command allowlist merely to make the new tool look broader.
- Reading/printing private keys or raw operator SSH config to clients/models.
- Restarting/reloading/stopping/starting the systemd relay, replacing the deployed binary, or making real production SSH connections during implementation.
- Git push/PR/merge unless separately authorized after local implementation closure.

## Public contract

### Tool schema

`ssh_readonly_exec` should expose only reviewed application-level fields:

- `alias: string` — required, bounded SSH alias resolved by the relay-owned safe config parser.
- `command: string` — required, one remote executable/family name such as `docker`, `git`, `curl`, `uptime`.
- `args: string[]` — optional bounded argv tokens for the remote command. Composition operators are permitted only if the existing Plan 061 parser supports them through the reviewed semantic path; do not expose a raw shell blob as a second schema field.
- `timeout_ms: integer` — optional, bounded by the SSH-specific maximum.
- `execution_mode: sync|async|auto` — optional, reusing standard MCP Tasks semantics.

No `cwd` is required for remote authority. If the current execution abstraction needs a local cwd internally, derive it server-side from the configured execution root rather than making local workspace selection part of the SSH public API.

### Static annotations/effects

The dedicated tool can advertise truthful static semantics because mutation-capable remote commands are rejected inside the tool:

- `readOnlyHint: true`
- `destructiveHint: false`
- `idempotentHint: true`
- `openWorldHint: true`

Canonical effects:

```text
process_exec + network_read + privileged_bridge
```

It must not advertise `workspace_write`, `workspace_delete`, `network_write`, or `external_mutation`.

The `privileged_bridge` effect remains appropriate because the relay deliberately crosses into an operator-authorized remote host/credential boundary even though the allowed operation is read-only.

## Phase overview

| Phase | Goal | Exit criteria |
| --- | --- | --- |
| PHASE-01 | Freeze dedicated tool contract and terminal rollback boundary | One explicit schema/effect/profile contract; exact Plan 061 terminal special-cases identified for removal |
| PHASE-02 | Refactor execution owner around dedicated SSH request | Dedicated tool builds the existing hardened SSH invocation without generic terminal routing |
| PHASE-03 | Remove terminal SSH semantics and enforce one canonical path | `terminal_exec` is generic again and direct SSH through it fails closed |
| PHASE-04 | Integrate catalog/tasks/activity/effects | All client-visible metadata is static, truthful, portable, and credential-safe |
| PHASE-05 | Adversarial, compatibility, and regression validation | Dedicated positive cases pass; old terminal path and all bypasses fail |
| PHASE-06 | Docs, memory, guardrail, local closure | Code/docs/plan agree; applicable gates pass; local commit and clean branch only |

## PHASE-01 — Contract and ownership

### TASK-001: Add first-class MCP catalog contract

**Outcome:** `tools/list` exposes a self-contained SSH capability that requires no client knowledge of `terminal_exec` internals.

**Primary files:**
- `packages/rust-tools/interfaces/src/mcp/catalog.rs`
- package-local catalog/profile tests or existing acceptance coverage

**Steps:**
- [ ] Add `ssh_readonly_exec` with the bounded schema defined above.
- [ ] Give it static read-only/open-world annotations and optional task support.
- [ ] Include it in both Full and Primary tool profiles unless evidence says otherwise.
- [ ] Keep raw SSH options/config/key fields absent from the schema.
- [ ] Update catalog/profile expected counts/contracts through existing feature/contract mechanisms; do not add plan-numbered verifier scripts.
- [ ] Ensure descriptions explicitly say that alias/config/key resolution is server-owned and mutation is rejected.

**Validation:**
- [ ] Catalog discovery test proves the tool is present in intended profiles and validates schema shape.
- [ ] Schema rejects unknown/raw SSH option fields.

### TASK-002: Freeze one canonical SSH authority owner

**Outcome:** Plan 061 core policy modules remain authoritative and are not duplicated in interfaces or clients.

**Primary files:**
- `packages/rust-tools/core/src/ssh_policy/**`
- `packages/rust-tools/core/src/config/ssh.rs`
- `packages/rust-tools/application/src/execution/ssh.rs`

**Steps:**
- [ ] Preserve `ssh_policy::resolve_connection_spec`, `openssh_args`, and remote semantic validation as the single security implementation.
- [ ] Preserve SSH root ownership/canonicalization and exact credential-file containment.
- [ ] Preserve pinned `/usr/bin/ssh` or `/bin/ssh` selection, strict host trust, BatchMode/key-only/no-agent/no-forwarding/no-PTY controls.
- [ ] Refactor only the public request translation layer needed to consume `{ alias, command, args, ... }` directly.
- [ ] Do not introduce a second parser or client-visible OpenSSH argument path.

## PHASE-02 — Dedicated execution path

### TASK-003: Build dedicated invocation from structured SSH input

**Outcome:** Application execution constructs Plan 061’s hardened invocation directly from `ssh_readonly_exec` arguments.

**Primary files:**
- `packages/rust-tools/application/src/execution.rs`
- `packages/rust-tools/application/src/execution/requests.rs`
- `packages/rust-tools/application/src/execution/ssh.rs`
- `packages/rust-tools/application/src/execution/sandbox.rs`
- `packages/rust-tools/application/src/execution/sandbox/ssh_material.rs`

**Steps:**
- [ ] Add a dedicated request builder for `ssh_readonly_exec`; do not route through terminal command parsing.
- [ ] Convert `command + args` into the existing validated remote-command representation with deterministic token/rendering rules.
- [ ] Require `RELAY_ALLOW_SSH=true` and the existing reviewed SSH config/principal configuration.
- [ ] Retain the SSH-specific 60-second maximum unless a current review justifies changing it.
- [ ] Reuse `InvocationSecurity::Ssh` or rename it to a clearer dedicated type if that improves ownership without broad refactoring.
- [ ] Reuse read-only workspace/no-local-privileged-sockets/exact-credential mounts and null stdin.
- [ ] Keep auth/host-key/connect failure normalization and credential redaction.
- [ ] Preserve cancellation/process-group cleanup and bounded output.

**Validation:**
- [ ] Application integration test proves the dedicated call reaches the SSH invocation path with no generic terminal parsing.
- [ ] Disabled SSH capability fails before spawn.
- [ ] Oversized timeout/args/invalid alias fail before spawn.

### TASK-004: Support standard sync/async/auto lifecycle

**Outcome:** Dedicated SSH calls work uniformly across MCP clients that do or do not negotiate Tasks.

**Primary files:**
- `packages/rust-tools/application/src/execution.rs`
- `packages/rust-tools/application/src/execution/dispatch.rs` and/or current task-support owner
- `packages/rust-tools/infrastructure/src/transport/task_calls.rs`
- `packages/rust-tools/infrastructure/src/transport/tool_helpers.rs`
- `packages/rust-tools/infrastructure/src/transport/tools.rs`

**Steps:**
- [ ] Make `ssh_readonly_exec` task-capable using the same JobManager process job as other task-supported execution tools.
- [ ] Because the tool is technically read-only, do not require mutation idempotency keys for async execution.
- [ ] Keep fallback job APIs backward-compatible; do not create `ssh_job_start/get/cancel` tools.
- [ ] Ensure sync/async/auto admission has one implementation rather than SSH-specific transport duplication.

## PHASE-03 — Restore generic `terminal_exec`

### TASK-005: Remove Plan 061 hidden terminal routing

**Outcome:** `terminal_exec` once again represents ordinary local sandboxed execution only.

**Primary files identified from current source:**
- `packages/rust-tools/core/src/terminal_policy.rs`
- `packages/rust-tools/application/src/execution/requests.rs`
- `packages/rust-tools/application/src/hooks/policy.rs`
- `packages/rust-tools/application/src/activity/presentation.rs`
- `packages/rust-tools/infrastructure/src/transport/tool_helpers.rs`
- `packages/rust-tools/infrastructure/src/transport/tools.rs`
- Plan 061 tests that currently exercise SSH through `terminal_exec`

**Steps:**
- [ ] Remove `is_ssh_request` as a cross-layer capability classifier once no legitimate caller remains.
- [ ] Remove `command == ssh` dispatch from generic terminal request building.
- [ ] Remove SSH-specific dynamic effects from generic terminal hook/activity policy.
- [ ] Remove SSH-specific mutation-idempotency exception from `terminal_exec`.
- [ ] Remove SSH-specific activity rendering from generic terminal presentation.
- [ ] Restore generic terminal job/task behavior to its pre-Plan-061 semantics.
- [ ] Move any still-useful SSH normalization/redaction helpers to the dedicated tool owner rather than deleting security logic.

### TASK-006: Block alternate SSH execution through generic terminal

**Outcome:** There is one model-facing SSH security path, even if ordinary terminal networking is enabled.

**Primary files:**
- `packages/rust-tools/core/src/terminal_policy.rs`
- `packages/rust-tools/core/tests/terminal_policy.rs`

**Steps:**
- [ ] Reject exact `ssh` and reviewed alternate SSH client/file-transfer executables from generic terminal (`ssh`, `scp`, `sftp`, and equivalent names already relevant to this runtime).
- [ ] Return a bounded stable error: use `ssh_readonly_exec` for remote diagnostics.
- [ ] Ensure shell wrappers cannot trivially regain SSH by using a generic `sh -lc 'ssh ...'` path if terminal policy currently allows arbitrary shells. If shell execution remains intentionally supported for local coding, enforce the SSH executable boundary at the sandbox/executable/network layer rather than pretending string matching is sufficient. Choose the smallest technically sound boundary after implementation review.
- [ ] Do not weaken ordinary terminal behavior for unrelated local coding tools.

**Security note:** The purpose is canonicalization, not to claim that string-level shell denylisting can secure arbitrary shell execution. If `terminal_exec` with a network-enabled shell could invoke `/usr/bin/ssh`, the implementation must prevent that by namespace/executable exposure or another authoritative mechanism. Document the exact chosen boundary in the plan when implemented.

## PHASE-04 — Client-visible semantics

### TASK-007: Give dedicated tool truthful effects/activity

**Outcome:** Every MCP client can reason from ordinary catalog/tool metadata; Nuxt does not need an SSH special case.

**Primary files:**
- `packages/rust-tools/application/src/hooks/policy.rs`
- `packages/rust-tools/application/src/activity/presentation.rs`
- `packages/rust-tools/infrastructure/src/transport/tool_helpers.rs`

**Steps:**
- [ ] Add static dedicated effects `process_exec + network_read + privileged_bridge`.
- [ ] Never report workspace write/external mutation for admitted `ssh_readonly_exec` calls.
- [ ] Activity target/action must persist only bounded alias + diagnostic family (for example `SSH read-only · smart-meeting · docker logs`), never raw SQL literals, full command payloads, key/config paths, or credentials.
- [ ] Result detail/output keeps existing bounded credential redaction.
- [ ] Dedicated tool denials remain actionable and stable without exposing host credential details.

### TASK-008: Keep clients generic

**Outcome:** No first-party frontend is required to understand hidden SSH semantics.

**Review files only unless stale documentation/tests require edits:**
- `server/infrastructure/mcp/**`
- `server/application/chat/**`
- `shared/utils/capability-policy.ts`
- relevant web tests/docs

**Steps:**
- [ ] Do not add `ssh` command parsing to Nuxt/shared capability policy.
- [ ] Verify existing generic MCP tool composition can discover the new tool from stored relay inventory.
- [ ] If first-party Nuxt’s static effect policy needs a generic update for the new tool name, derive it from ordinary dedicated tool identity/annotations rather than parsing SSH input. Prefer no web change if existing external-tool annotations already work.
- [ ] Verify an external MCP server cannot gain first-party trust simply by naming a tool `ssh_readonly_exec`; existing provenance rules remain authoritative.

## PHASE-05 — Security and compatibility validation

### TASK-009: Migrate Plan 061 tests to the dedicated public path

**Outcome:** Test coverage reflects the actual supported interface rather than retaining dead terminal behavior.

**Primary tests:**
- `packages/rust-tools/core/tests/ssh_policy.rs`
- `packages/rust-tools/core/tests/relay_config.rs`
- `packages/rust-tools/core/tests/terminal_policy.rs`
- `packages/rust-tools/application/tests/ssh_diagnostics.rs`
- infrastructure/catalog/profile tests as appropriate

**Required matrix:**
- [ ] dedicated tool schema/discovery in Full + Primary;
- [ ] safe alias + `docker ps/logs/stats/top/inspect` normalization;
- [ ] nested read-only DB/Redis clients with least-privilege identities;
- [ ] read-only host/Git/HTTP diagnostic families;
- [ ] bare/interactive semantics impossible by schema;
- [ ] password/passphrase/keyboard-interactive behavior remains non-interactive/fail-closed;
- [ ] unknown/changed host keys remain fail-closed;
- [ ] raw SSH flags/config/key path injection impossible through schema;
- [ ] Docker mutation, shell/interpreter escape, process signaling, Git mutation, writable SQL/Redis, streaming/follow, metadata curl, sensitive path reads remain denied;
- [ ] activity does not retain query literals/credentials;
- [ ] generic `terminal_exec` no longer gets SSH-specific effects/idempotency/activity behavior;
- [ ] direct SSH via `terminal_exec` is rejected through the chosen canonical-path technical boundary;
- [ ] ordinary unrelated terminal execution retains existing behavior;
- [ ] ordinary non-SSH terminal network setting semantics remain unchanged.

### TASK-010: Portable MCP acceptance

**Outcome:** The feature is demonstrably usable without a Nuxt-specific adapter.

**Steps:**
- [ ] Use direct Rust/MCP integration tests to call `tools/list` and `tools/call` for `ssh_readonly_exec` through the real transport/application path.
- [ ] Keep an opt-in real OpenSSH fixture smoke test if useful, but do not touch production hosts during local closure.
- [ ] Prove the same tool contract is visible to any standard MCP consumer from `tools/list`; do not encode a client name into runtime behavior.
- [ ] If catalog snapshots/contracts are versioned, update them deliberately and record the exact new count/hash using existing contract mechanisms.

## PHASE-06 — Documentation and closure

### TASK-011: Update operator/client documentation

**Primary files:**
- `packages/rust-tools/README.md`
- `.env.example`
- relevant `docs/configuration.md`, `docs/external-mcp.md`, `docs/mcp-client.md`, `packages/relay-agent/SKILL.md` when their catalog/profile wording is affected

**Steps:**
- [ ] Replace `terminal_exec(command="ssh ...")` examples with `ssh_readonly_exec`.
- [ ] Explain that `-F /dev/null` is an internal OpenSSH hardening detail after alias normalization; clients must not provide it.
- [ ] Explain why `.ssh` being invisible to ordinary terminal/file tools is expected and not evidence that operator SSH config is absent.
- [ ] Document SSH enable/config variables without exposing secrets.
- [ ] State that direct SSH through generic terminal is unsupported; use the dedicated tool.
- [ ] Keep remote mutation/manual-command guidance and defense-in-depth recommendations from Plan 061.

### TASK-012: Reconcile Plan 061 and durable memory

**Primary files:**
- `.agents/plans/061-read-only-ssh-execution.md`
- `.agents/plans/062-dedicated-readonly-ssh-tool.md`
- `.agents/memories/README.md`

**Steps:**
- [ ] Keep Plan 061 as historical implementation truth, but append a supersession note that its `terminal_exec` exposure was replaced by Plan 062.
- [ ] Record the durable invariant: security-sensitive capabilities with materially different static effects should be first-class MCP tools when portability across generic clients matters; do not require clients to infer hidden input-sensitive semantics of a generic tool.
- [ ] Record that the SSH backend remains Rust-owned and client-independent.
- [ ] Follow `.agents/knowledge/self-improvement.md` before closure.

### TASK-013: Run local closure gates and commit

**Required:**
- [ ] `cargo fmt --all -- --check`
- [ ] focused core SSH/config/terminal tests
- [ ] focused application/infrastructure/catalog tests
- [ ] `cargo test --workspace --lib --bins --tests --all-features --locked`
- [ ] `cargo clippy --workspace --lib --bins --tests --all-features --locked -- -D warnings`
- [ ] `RUSTFLAGS='-D warnings' cargo check --workspace --lib --bins --tests --all-features --locked`
- [ ] applicable Nuxt tests only if web/shared code actually changes
- [ ] `pnpm guardrail`
- [ ] `git diff --check`
- [ ] re-check maintainability/test-layout after structural moves
- [ ] local commit(s) on `feat/062-dedicated-readonly-ssh-tool`
- [ ] clean worktree

No push, PR, merge, service restart, deployed binary replacement, or real production SSH connection occurs without separate authorization.

## Implementation guidance

Prefer reuse over rewrite:

```text
existing Plan 061 core policy
  ├─ safe SSH config parser
  ├─ normalized connection spec
  ├─ OpenSSH hardened argv
  ├─ remote command semantic policy
  ├─ Docker/DB/Git/network/read adapters
  └─ exact credential validation
             │
             ▼
new ssh_readonly_exec request adapter
             │
             ▼
existing SSH-specific invocation/sandbox/job lifecycle
```

Delete only the coupling to generic terminal, not the proven backend.

## Risks

- **Two SSH paths survive:** explicitly test that generic terminal cannot become an alternate SSH route.
- **Client portability regresses through frontend policy:** do not parse SSH commands in Nuxt/OpenWebUI adapters; dedicated tool identity/annotations must be sufficient.
- **Catalog compatibility:** adding one tool changes profile counts/snapshots. Update frozen contracts deliberately and verify clients tolerate additive discovery.
- **Task lifecycle drift:** reuse JobManager and standard task execution; do not create SSH-specific polling APIs.
- **Activity confidentiality:** a dedicated tool makes it tempting to log structured arguments. Continue logging only alias/family metadata, not query literals or credentials.
- **Security regression during refactor:** preserve Plan 061 core adversarial tests before deleting terminal-specific tests; migrate coverage first, then remove dead routing.
- **Generic shell bypass:** rejecting `terminal_exec(command="ssh")` alone is not sufficient if a network-enabled shell can execute `/usr/bin/ssh`; implementation review must enforce one canonical route at the actual executable/sandbox authority boundary.

## Final acceptance criteria

- [ ] `tools/list` exposes `ssh_readonly_exec` as an explicit read-only/open-world MCP tool in intended profiles.
- [ ] A generic MCP client requires no Nuxt/OpenWebUI-specific SSH logic to discover or call it.
- [ ] The public schema has no raw SSH option/config/key path surface.
- [ ] Existing Plan 061 alias/config/key/auth/host-trust/sandbox/remote semantic policy remains authoritative.
- [ ] Dedicated safe remote diagnostics reach the existing SSH backend.
- [ ] Dedicated mutation/confidentiality/availability bypasses remain denied.
- [ ] `terminal_exec` contains no SSH-specific request routing/effect/idempotency/activity special-case.
- [ ] Generic terminal cannot be used as a second SSH execution path.
- [ ] Generic non-SSH terminal behavior is unchanged.
- [ ] Activity/errors remain bounded and credential-safe.
- [ ] Standard MCP Tasks/fallback execution lifecycle remains coherent without new SSH job tools.
- [ ] Profile/catalog contracts and docs reflect the dedicated tool.
- [ ] Full applicable Rust tests, warnings-denied checks, maintainability/test-layout gates, and `pnpm guardrail` pass.
- [ ] Plan 061 is marked superseded only at the exposure layer, not falsely rewritten as if its backend never existed.
- [ ] Plan 062 and canonical memory reflect exact implementation truth.
- [ ] Local branch is committed and clean.
- [ ] No systemd relay action occurred.
- [ ] No production SSH connection occurred.
- [ ] No push occurred without separate authorization.
