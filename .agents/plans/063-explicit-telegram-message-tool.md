# Plan 063 — Explicit Telegram Message MCP Tool

**Status:** CLOSED — IMPLEMENTED / LOCALLY VERIFIED (2026-08-31)
**Goal:** Replace the orchestration-coupled Telegram task-completion notification pipeline with one explicit MCP tool, require an authorized absolute `working_directory` on every message, and remove the obsolete completion-notification plumbing without weakening the relay-owned Telegram credential/destination boundary.
**Success Criteria:** `tools/list` exposes `telegram_send_message` with required `working_directory` and `message`; valid calls route through the existing encrypted fixed-destination Telegram backend; invalid/missing/unauthorized working directories fail closed; caller arguments cannot override token/chat/thread/endpoint; orchestration completion no longer emits Telegram messages automatically; the legacy `task_completed` tool, private `server/task_completed` extension, Nuxt outbox/database/plugin wiring, and completion-specific dead code are removed; focused Rust/web tests and `pnpm guardrail` pass. Runtime activation is a separately authorized exact-merged-`main` operation and is not implied by source closure.

## Scope

### In scope

- Add a first-class MCP tool named `telegram_send_message`.
- Require `working_directory` and `message` on every call.
- Validate `working_directory` against the relay's currently authorized workspace roots before delivery.
- Include the normalized working directory in every Telegram message server-side.
- Reuse relay-owned Telegram bootstrap credentials, encrypted storage, fixed destination/topic, bounded sendMessage client, durable retry/backoff, and redaction.
- Remove the `task_completed` MCP tool and private `server/task_completed` transport extension.
- Remove Nuxt orchestration completion auto-notification, task-notification outbox/database/plugin/client capability code, and related tests.
- Reconcile docs, configuration comments, canonical memory, and historical Plan 056/057/058 wording where it would otherwise describe the removed live architecture.
- Perform a repository-wide orphan-symbol/dead-path sweep.

### Out of scope

- Caller-controlled Telegram token, chat ID, thread ID, API endpoint, or arbitrary Telegram methods.
- Inbound Telegram messaging/bot-command handling.
- Changing the existing owner-controlled Telegram bootstrap credential mechanism unless required for the new generic message payload.
- Runtime activation without a separate operator authorization. A separately authorized exact-merged-`main` deployment/restart is recorded as an operational handoff, not inferred from source closure.
- Broad workspace-authorization redesign unrelated to validating the new tool input.

## Starting State (before this plan)

- The relay owns Telegram credentials and fixed-destination `sendMessage` delivery under `packages/rust-tools/infrastructure/src/notifications*`.
- The current public tool `task_completed` accepts completion metadata and feeds the same relay notification ledger.
- Nuxt separately detects and calls the private `server/task_completed` extension through `server/infrastructure/mcp/modern-http-client.ts`.
- Orchestration completion automatically enqueues a Nuxt task-notification outbox entry from `server/infrastructure/ai/subagent-tool.ts`.
- Current documentation explicitly states that local filesystem paths are never sent to Telegram; this plan deliberately replaces that guarantee with a required owner-visible working-directory field.

## Constraints & Decisions

- Telegram becomes an explicit model-facing capability rather than an orchestration lifecycle side effect.
- The canonical tool is `telegram_send_message`; do not retain `task_completed` as an alias.
- Input schema is intentionally small: `working_directory` and `message` only.
- `working_directory` must be an absolute, canonical, currently authorized workspace directory. The relay, not the caller, formats the final `Working directory: ...` prefix.
- Telegram destination credentials remain relay-owned and absent from MCP arguments.
- Effects remain `network_write + external_mutation`; the tool is non-destructive but not read-only/idempotent.
- Reuse one relay delivery owner; do not create a second generic HTTP/Telegram client path.
- Delete obsolete Nuxt persistence/plugin/client plumbing instead of deprecating it indefinitely.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
| --- | --- | --- | --- |
| PHASE-01 | Freeze explicit tool and payload contract | none | Catalog schema/effects and workspace-validation owner are clear |
| PHASE-02 | Implement relay-side explicit message delivery | PHASE-01 | Valid tool calls queue/send bounded messages with required cwd |
| PHASE-03 | Remove legacy relay completion interfaces | PHASE-02 | `task_completed` and `server/task_completed` are absent |
| PHASE-04 | Remove Nuxt auto-notification pipeline | PHASE-03 | No task-notification outbox/plugin/orchestration completion wiring remains |
| PHASE-05 | Reconcile tests, docs, memory, and contracts | PHASE-04 | Source and durable documentation describe one explicit Telegram tool |
| PHASE-06 | Orphan sweep and validation | PHASE-05 | Stale symbols are gone and applicable local gates pass |
| PHASE-07 | Local closure | PHASE-06 | Plan is truthful, task-owned diff is committed, no runtime deployment performed |

## Local Closure Evidence (2026-08-31)

- Frozen historical catalog v12 remains byte-for-byte unchanged (`e48833e743960a134906927eca4543f3c6b9cad8c45b2fb16dd6aca57bdc3b54`). The current v13 snapshot is `.agents/contracts/063-tool-catalog-v13.json` with SHA-256 `606f16cab046283c77b7c5bf773c2dbfa51cf62d6488b63855705392e25a479e`.
- The transport-level loopback test exercises `tools/list` and `tools/call` through the actual Axum MCP router into `telegram_send_message`, while Telegram delivery is disabled so no external API call is made. It verifies the valid route, missing/relative/unauthorized `working_directory` rejection, and `task_completed`/`server/task_completed` absence.
- Focused Rust Telegram, catalog, transport, state-migration/topic-routing, and activity-privacy tests pass. The final body fails closed when the full canonical directory plus sanitized/redacted message would exceed Telegram's 4096-byte limit; it never truncates either field.
- The focused web capability-policy test and the full cross-stack `pnpm guardrail` pass (web lint/typecheck/unit tests; Rust formatting/clippy/typecheck/workspace tests).
- A repository-wide orphan sweep found only intentionally historical contracts/plans/migrations, removal assertions, and the retained relay-owned legacy state/table names needed to keep old completion rows inert. No active Nuxt or relay completion route/pipeline remains.
- This source closure did not itself restart a relay or claim live Telegram delivery. A separately authorized post-merge exact-`main` deployment must verify its own binary hash chain, service state, live catalog, and at most one explicit smoke call.

## PHASE-01 — Public contract and authorization ownership

### TASK-001: Replace the completion tool contract

**Outcome:** MCP discovery advertises one explicit Telegram send capability.

**Files:**
- Modify: `packages/rust-tools/interfaces/src/mcp/catalog.rs`
- Create/modify the existing package-local Telegram catalog module as appropriate
- Test: existing Rust catalog/notification integration tests

**Steps:**
- [x] Replace `task_completed` with `telegram_send_message`.
- [x] Require `working_directory` and `message`; reject additional properties.
- [x] Bound both values with explicit schema limits.
- [x] Keep Telegram credentials/destination absent from the schema.
- [x] Advertise truthful annotations and the existing coding security scheme.
- [x] Update tool profiles/catalog snapshots or contract tests owned by the current catalog version.

**Validation:**
- Rust catalog tests prove `telegram_send_message` exists and `task_completed` does not.
- Schema requires both fields and exposes no destination/credential fields.

### TASK-002: Reuse authoritative workspace authorization

**Outcome:** The relay accepts only a canonical absolute directory that is currently authorized by the same workspace authority used by filesystem/tool execution.

**Files:**
- Modify only the existing workspace authorization owner and/or notification adapter boundary required to query it.
- Test: Rust notification/tool integration tests.

**Steps:**
- [x] Resolve/canonicalize the supplied directory without following unsafe escapes.
- [x] Require it to be a directory and inside the active authorized workspace set, not merely under the execution ceiling.
- [x] Reject missing, relative, unauthorized, protected, or invalid paths before queue insertion/network delivery.
- [x] Keep authorization logic in one existing owner rather than reimplementing path policy inside the Telegram HTTP client.

## PHASE-02 — Relay-side generic Telegram message delivery

### TASK-003: Replace completion payload formatting with generic message payload

**Outcome:** The relay stores and sends one bounded redacted message containing the server-formatted working-directory prefix.

**Files:**
- Modify: `packages/rust-tools/infrastructure/src/notifications.rs`
- Modify: `packages/rust-tools/infrastructure/src/notifications/ledger.rs`
- Modify: `packages/rust-tools/infrastructure/src/notifications/telegram.rs` only if the generic payload requires it
- Test: `packages/rust-tools/infrastructure/tests/task_notifications.rs` renamed/reworked to a feature-appropriate Telegram-message test file if needed

**Steps:**
- [x] Replace completion-specific payload/source/title/summary/result URL semantics with `working_directory + message`.
- [x] Format every outgoing body as `Working directory: <canonical path>\n\n<message>`.
- [x] Preserve control-character stripping, credential-shaped redaction, UTF-8 byte bounds, and Telegram's maximum message bound.
- [x] Preserve encrypted credentials, fixed recipient/topic, retry/backoff, rate-limit handling, and bounded persistence.
- [x] Choose a safe deduplication/message identity model appropriate to explicit sends; do not accidentally suppress two distinct legitimate messages merely because their text matches.

### TASK-004: Wire `telegram_send_message` through ordinary `tools/call`

**Outcome:** Explicit MCP calls invoke the generic Telegram service through the canonical tool-dispatch path.

**Files:**
- Modify: `packages/rust-tools/infrastructure/src/transport/tools.rs`
- Replace/rename: `packages/rust-tools/infrastructure/src/transport/task_completion.rs`
- Modify effect/activity policy owners as required.

**Steps:**
- [x] Parse and validate the new arguments.
- [x] Resolve authorized cwd before invoking notification persistence/delivery.
- [x] Return bounded status such as queued/sent-disabled semantics without leaking credential/network details.
- [x] Preserve `network_write + external_mutation` policy classification.
- [x] Ensure activity/telemetry remains credential-safe and does not record raw message bodies.

## PHASE-03 — Remove legacy relay completion interfaces

### TASK-005: Remove `server/task_completed`

**Outcome:** The private first-party completion extension no longer exists.

**Files:**
- Modify: `packages/rust-tools/application/src/dispatcher.rs`
- Modify: `packages/rust-tools/infrastructure/src/transport/mcp_http.rs`
- Modify: `packages/rust-tools/interfaces/src/mcp.rs`
- Modify relevant protocol/transport tests.

**Steps:**
- [x] Remove dispatcher variant/route for `server/task_completed`.
- [x] Remove extension advertisement `io.masihawam/task-completion-notifications`.
- [x] Remove Nuxt/external source distinctions that existed only for completion signaling.
- [x] Delete completion-only parser/types/functions with no remaining caller.

### TASK-006: Remove stale completion-specific policy/catalog symbols

**Outcome:** No hidden alias or policy branch still recognizes `task_completed`.

**Files:**
- Modify: `packages/rust-tools/application/src/hooks/policy.rs`
- Modify: `packages/rust-tools/interfaces/src/mcp/catalog.rs`
- Modify any current catalog frozen contract if repository policy requires a new version rather than editing historical snapshots.

**Steps:**
- [x] Replace effect classification with `telegram_send_message`.
- [x] Remove old allowlist/profile/catalog references.
- [x] Preserve historical immutable contract snapshots and create v13 as the current successor.

## PHASE-04 — Remove Nuxt automatic completion pipeline

### TASK-007: Remove MCP completion capability/client methods

**Outcome:** Nuxt no longer probes or calls the private completion extension.

**Files:**
- Modify: `server/infrastructure/mcp/modern-http-client.ts`
- Modify client interfaces/types/callers.
- Test: `test/unit/task-notifications.test.ts` replaced with focused surviving behavior tests or removed if the subsystem disappears entirely.

**Steps:**
- [x] Remove `supportsTaskCompletion()` and `taskCompleted()`.
- [x] Remove private extension discovery and request method typing.
- [x] Remove completion-specific result/input types with no callers.

### TASK-008: Remove orchestration auto-notification and Nuxt outbox

**Outcome:** Completing an orchestrator graph has no Telegram side effect.

**Files:**
- Modify: `server/infrastructure/ai/subagent-tool.ts`
- Modify: `server/infrastructure/ai/chat-turn-dependencies.ts`
- Modify: `server/application/chat/contracts.ts`
- Delete if orphaned: `server/application/task-notifications.ts`
- Delete if orphaned: `server/infrastructure/task-notifications/outbox-worker.ts`
- Delete if orphaned: `server/plugins/task-notifications.server.ts`
- Delete if orphaned: `server/infrastructure/database/task-notifications.ts`
- Remove related schema/migration artifacts only if they are genuinely feature-owned and safe to remove from current source; preserve already-applied historical migrations as immutable history.

**Steps:**
- [x] Remove `notifyIfCompleted()` and completion-transition helpers.
- [x] Remove task-notification dependency injection/ports.
- [x] Remove server plugin worker startup.
- [x] Remove active database schema/runtime code that exists solely for the Nuxt notification outbox.
- [x] Do not delete historical migration files merely because the runtime table becomes unused.

## PHASE-05 — Tests, documentation, and durable context

### TASK-009: Rewrite feature tests around explicit sends

**Outcome:** Tests prove the new security and behavior contract rather than legacy completion semantics.

**Steps:**
- [x] Positive valid authorized cwd + message case.
- [x] Missing cwd/message rejection.
- [x] Relative/nonexistent/unauthorized cwd rejection.
- [x] Unknown/destination/credential argument rejection at schema/handler boundaries.
- [x] Message formatting always includes canonical cwd.
- [x] Redaction/control-character/message-size behavior remains bounded and fails closed for an oversized final body.
- [x] Fixed topic/channel credentials still load from relay-owned encrypted state.
- [x] Legacy `task_completed`/`server/task_completed` absence is tested.
- [x] Orchestration completion no longer enqueues a notification.

### TASK-010: Reconcile operator docs and memory

**Files:**
- Modify: `.env.example`
- Modify: `docs/configuration.md`
- Modify: `docs/remote-mcp.md`
- Modify: `.agents/memories/README.md`
- Modify Plan 056/057/058 status/invariant wording only where needed to make clear it is historical/superseded.

**Steps:**
- [x] Document the explicit `telegram_send_message` tool and required cwd/message fields.
- [x] Explicitly document that the canonical working directory is sent to the configured Telegram destination.
- [x] Retain fixed destination/bootstrap/credential security documentation.
- [x] Remove claims that Telegram is not an MCP tool or that local paths are never sent.
- [x] Record Plan 063 as the superseding live architecture in canonical memory.

## PHASE-06 — Orphan sweep and validation

### TASK-011: Repository-wide stale-symbol/dead-path sweep

**Steps:**
- [x] Search for `task_completed`.
- [x] Search for `server/task_completed`.
- [x] Search for `TaskCompletion`, `taskCompletion`, `taskNotifications`, `task-notifications`, `TASK_NOTIFICATION`.
- [x] Search for `completionTransitionWasNewlyReached`, `taskCompletionInputForGraph`, `supportsTaskCompletion`.
- [x] Delete or reconcile every remaining live-code occurrence; historical plan text remains only as historical/superseded evidence, removal assertions, or required inert-state compatibility.

### TASK-012: Run applicable repository gates

**Validation:**
- Focused Rust Telegram/catalog/transport tests.
- Focused web tests covering changed MCP/orchestration surfaces.
- `pnpm guardrail` → pass.
- Run additional Rust security/audit checks only if the implementation changes dependencies or security-sensitive code in a way that warrants them.

## PHASE-07 — Local closure

### TASK-013: Close plan and commit task-owned changes

**Steps:**
- [x] Re-read the final diff for unrelated changes/secrets.
- [x] Update this plan status/checklists truthfully.
- [x] Perform the mandatory `.agents/knowledge/self-improvement.md` review.
- [x] Revalidate repository identity/worktree before Git writes.
- [x] Stage only Plan 063 task-owned paths.
- [x] Review staged diff/check/stat.
- [x] Commit logical task-owned changes only after `pnpm guardrail` passes.
- [x] Keep source closure separate from runtime activation; an exact-merged-`main` restart occurs only under a separate explicit authorization.

## Risks & Rollback

- **Security contract change — absolute workspace paths now leave the machine via Telegram.** Mitigation: require explicit tool invocation, validate the path against active workspace authorization, send only to the fixed owner-configured destination, and document the behavior prominently. Rollback is reverting Plan 063 and restoring the completion-only formatter.
- **Duplicate/suppressed sends after removing task-id deduplication.** Mitigation: use a message identity suited to explicit calls and test repeated distinct sends. Do not derive dedup solely from message text.
- **Deleting active Nuxt persistence too aggressively.** Mitigation: perform reference search before deletion and preserve historical migrations.
- **Catalog compatibility break.** Intentional: `task_completed` is removed rather than aliased. Document the breaking tool-contract change and keep frozen historical catalog snapshots immutable.
- **Workspace authorization drift.** Mitigation: reuse the existing authoritative authorized-root registry rather than introducing notification-local path checks.

## Final Acceptance Criteria

- [x] `tools/list` exposes `telegram_send_message`.
- [x] `task_completed` is absent.
- [x] `server/task_completed` and its advertised extension are absent.
- [x] Every valid send requires an authorized absolute `working_directory` and non-empty bounded `message`.
- [x] Every outgoing Telegram body contains the canonical working directory server-side.
- [x] Token/chat/thread/endpoint cannot be overridden through MCP arguments.
- [x] Orchestration completion performs no automatic Telegram notification.
- [x] Nuxt task-notification outbox/plugin/database runtime code is removed when orphaned.
- [x] Existing Telegram encrypted credential storage, fixed destination/topic, bounded send, redaction, retry/backoff remain intact.
- [x] Stale live-code symbols are removed.
- [x] Docs and canonical memory describe the new architecture.
- [x] Focused tests and `pnpm guardrail` pass.
- [x] Source closure does not imply a live deployment; any user-authorized activation must be exact-merged-`main` and separately verified.

## Execution Handoff

Execute phases in order because the public tool contract determines relay implementation, which in turn makes the legacy relay and Nuxt pipelines safely deletable. Rust catalog/notification work and Nuxt orphan cleanup can be reviewed independently after the new relay path exists, but final validation is cross-stack because both sides intentionally remove the old shared completion contract. Runtime activation remains a separate operator action and must build from the exact merged `main` commit before replacing the installed relay binary.
