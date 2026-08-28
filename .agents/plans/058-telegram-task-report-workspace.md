# Plan 058 — Telegram Task Report Workspace Context

**Status:** IMPLEMENTATION COMPLETE; DEPLOYMENT PENDING
**Goal:** Make every task-completion Telegram message an actionable report that identifies the workspace and reflects the completed task details, without introducing per-tool notifications or a generic Telegram capability.
**Success Criteria:** A workspace is mandatory in every completion contract, survives the Nuxt outbox handoff, appears in the relay-formatted message, orchestration reports include the task-node list, legacy Nuxt outbox rows remain deliverable with an explicit unavailable-workspace fallback, focused web/Rust tests and `pnpm guardrail` pass, and the merged implementation is deployed and verified through the relay worker.

## Scope

### In scope

- Add a required display-safe `workspace` field to the Nuxt and relay task-completion contracts.
- Persist workspace context in the Nuxt PostgreSQL outbox and migrate existing rows safely.
- Include workspace, task title, report summary, and optional result URL in the Telegram message.
- Generate an orchestration report from the actual task graph, including bounded node/task details.
- Require workspace in the explicit `task_completed` completion signal schema and validate it at the relay boundary.
- Update focused web/Rust tests, operator documentation, canonical memory, and deployment evidence.

### Out of scope

- Telegram messages for individual tools, progress updates, or activity events.
- Sending the local filesystem path or other private workspace metadata to Telegram.
- A generic Telegram/HTTP MCP tool or caller-supplied Telegram destination.
- Changing the existing standalone relay/Hermes ownership boundary or forum-topic routing.

## Current State

- The relay message now contains `✅ title`, `Workspace`, a report summary, and an optional result URL.
- Nuxt's aggregate graph completion summary now includes the count plus a bounded list of task-node IDs, statuses, and objectives.
- The completion payload requires a display-safe workspace name, and the chat turn passes the resolved workspace name into orchestration completion generation.
- The deployed relay is already configured for the `Masih Awam` forum topic; the implementation changes report content and handoff metadata, while deployment of this plan remains pending.

## Constraints & Decisions

- `workspace` means the user-visible workspace name, not the local path. It is mandatory for new completion events and bounded/redacted like other notification text.
- Existing Nuxt outbox rows created before this change must remain deliverable; the database migration will assign a clear `Workspace unavailable` compatibility value only to those legacy rows. New enqueue calls reject missing workspace data.
- The relay formats the final message from structured fields so both Nuxt-originated and external MCP client-originated completion signals have the same report shape.
- The task graph report is bounded to the existing notification limits and includes each node ID, status, and objective in a compact form.
- A result URL remains optional and must continue to be HTTPS-only.
- No raw workspace path, token, chat ID, or credential material is added to the message or durable project documentation.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
|---|---|---|---|
| PHASE-01 | Extend the completion contract and report formatter | none | Workspace is mandatory, persisted, validated, and rendered in both Nuxt and Rust paths. |
| PHASE-02 | Make reports task-specific and prove compatibility | PHASE-01 | Graph reports list bounded task details; legacy outbox migration and focused tests pass; docs are aligned. |
| PHASE-03 | Deliver and verify the live report | PHASE-02 | Merged main is deployed, service is healthy, and a live synthetic completion is sent through the configured topic. |

## PHASE-01 — Contract and formatter

**Goal:** Carry a required display-safe workspace name from chat context through the outbox/completion signal to Telegram.
**Dependencies:** none

### TASK-001: Add workspace to the Nuxt handoff

**Outcome:** Nuxt completion events persist the workspace name and pass it to the relay client.

**Files:**

- Modify: `server/application/task-notifications.ts`
- Modify: `server/application/chat/contracts.ts`
- Modify: `server/application/chat/execute-chat-turn.ts`
- Modify: `server/infrastructure/ai/subagent-tool.ts`
- Modify: `server/infrastructure/database/task-notifications.ts`
- Modify: `server/database/schema.ts`
- Add: `server/database/migrations/0029_*` and matching Drizzle metadata
- Test: `test/unit/task-notifications.test.ts`

**Steps:**

- [x] Add a bounded required `workspace` field to task-completion inputs and sanitized events.
- [x] Pass the already-resolved workspace name into orchestration completion generation.
- [x] Persist workspace in the Nuxt outbox and include it in the private relay handoff.
- [x] Add a safe PostgreSQL migration for existing outbox rows without losing pending delivery.
- [x] Keep workspace name separate from the local path and never include path data in the Telegram payload.

**Validation:**

- `pnpm test:web` → 29 passed, including workspace-required, report-format, and relay-handoff assertions.
- The generated migration adds a temporary compatibility default, backfills existing rows, enforces `NOT NULL`, and drops the default for future writes.

**Commit boundary:** `feat(notifications): include workspace in task reports`

### TASK-002: Enforce the relay-side contract and report format

**Outcome:** The Rust relay rejects completion payloads without workspace and renders one consistent report format.

**Files:**

- Modify: `packages/rust-tools/interfaces/src/mcp/catalog/task_completion.rs`
- Modify: `packages/rust-tools/infrastructure/src/notifications.rs`
- Modify: `packages/rust-tools/infrastructure/src/notifications/telegram.rs` or shared formatter location as appropriate
- Modify: `server/infrastructure/mcp/modern-http-client.ts`
- Test: `packages/rust-tools/infrastructure/tests/task_notifications.rs`

**Steps:**

- [x] Add required workspace validation to the private completion signal and relay payload parser.
- [x] Format messages with title, workspace, report summary, and optional result URL.
- [x] Preserve redaction, byte bounds, HTTPS result URL validation, and topic routing.
- [x] Keep source and destination caller-controlled fields out of the Telegram payload contract.

**Validation:**

- `cargo test --manifest-path packages/rust-tools/infrastructure/Cargo.toml --test task_notifications` → 14 passed, including missing workspace, formatted report, redaction, and topic compatibility.
- Web unit tests prove workspace is included in `server/task_completed`.

**Commit boundary:** `feat(relay): require workspace in telegram reports`

## PHASE-02 — Task-specific report and compatibility proof

**Goal:** Make orchestration notifications useful as reports and preserve legacy delivery.
**Dependencies:** PHASE-01

### TASK-003: Build task-aware orchestration summaries

**Outcome:** A completed orchestration report names the workspace and lists the bounded tasks/nodes that settled.

**Files:**

- Modify: `server/application/task-notifications.ts`
- Modify: `server/infrastructure/ai/subagent-tool.ts`
- Test: `test/unit/task-notifications.test.ts`
- Test: relevant orchestration unit test if the graph fixture needs extension

**Steps:**

- [x] Generate a compact task report from node ID, status, and objective.
- [x] Bound the number and size of task entries under the existing 2000-byte summary contract.
- [x] Keep the one-notification-per-logical-task transition and deduplication invariant unchanged.
- [x] Update completion signal guidance so external MCP client supplies a useful report covering changes, validation, and remaining risks when applicable.

**Validation:**

- Web unit coverage asserts the workspace line and task-specific report lines are present and the formatted message remains within Telegram's 4096-byte bound.

**Commit boundary:** Included with TASK-001/TASK-002 if cohesive.

### TASK-004: Documentation and local review

**Outcome:** Operator and project guidance describes the required workspace/report contract accurately.

**Files:**

- Modify: `docs/configuration.md`
- Modify: `docs/remote-mcp.md` if completion signal schema is documented there
- Modify: `.agents/memories/README.md` only for the durable invariant
- Modify: this plan

**Steps:**

- [x] Document the report format and workspace-name-only policy.
- [x] Record that legacy rows use `Workspace unavailable` only as a migration compatibility fallback.
- [x] Review for secret/path leakage, stale schema examples, and accidental per-tool notification behavior.
- [x] Run focused web/Rust tests, `pnpm guardrail`, and `git diff --check`.

**Validation:**

- `pnpm guardrail` → pass: repository/agent/architecture/maintainability/test-layout policy, ESLint, Nuxt typecheck, all web unit tests, Rust format/Clippy/typecheck, and all workspace Rust tests.
- `git diff --check` → pass.

**Commit boundary:** `docs(plan): record telegram task report contract` when separate.

## PHASE-03 — Reviewed deployment and live report verification

**Goal:** Deploy the merged contract and prove a real completion report is accepted by the configured forum topic.
**Dependencies:** PHASE-02

### TASK-005: Merge, deploy, and verify

**Outcome:** The installed running binary matches merged main and the relay worker sends a workspace-bearing task report.

**Steps:**

- [ ] Revalidate branch/status and task-owned paths before Git writes.
- [ ] Run the required gates, commit, push, open, and merge a PR into `main`.
- [ ] Build from merged `main`, compare the installed/running binary identity, and restart the discovered user service.
- [ ] Apply the Nuxt database migration through the repository-native migration path if deployment requires it.
- [ ] Enqueue one clearly labeled bounded synthetic completion with workspace/report content through the relay worker.
- [ ] Confirm the row is `sent`, the service remains healthy, and no secrets or local paths appear in logs/docs.

**Validation:**

- `systemctl --user is-active ai-tools-relay.service` is `active` with no restart increment.
- Relay health returns HTTP 200.
- The synthetic completion is marked sent with no retry/error category.
- The configured topic remains the delivery target.

**Commit boundary:** Runtime state and installed binaries are never committed.

## Risks & Rollback

- Existing outbox rows cannot accept a required column directly → add a temporary safe default, backfill the compatibility value, then enforce not-null for new writes.
- Workspace names may contain control bytes or credentials → reuse the completion sanitizer and bounded text policy.
- A report can become too long when a plan has many nodes → cap entries and truncate at the established summary/message limits.
- Nuxt and Rust contracts drift → keep matching TypeScript/Rust fixtures and run both changed-stack gates.
- Runtime binary or database migration does not match reviewed main → stop deployment and compare exact commit/build/runtime evidence before restart.

## Final Acceptance Criteria

- [x] New completion payloads cannot be enqueued without a workspace name.
- [x] Telegram message visibly identifies the workspace and includes the task-specific report summary.
- [x] Orchestration reports include bounded task/node details rather than only a generic node count.
- [x] Legacy Nuxt outbox rows remain deliverable with an explicit compatibility workspace value.
- [x] No per-tool notification or generic Telegram tool is introduced.
- [x] Focused web/Rust tests, `pnpm guardrail`, and `git diff --check` pass.
- [ ] PR is merged, checkout returns cleanly to `main`, and the deployed binary matches merged `main`.
- [ ] Live relay-worker delivery marks a workspace-bearing synthetic completion `sent` to topic `3775`.

## Execution Handoff

- Execute TASK-001 and TASK-002 as one implementation boundary because the Nuxt and Rust payload schemas must remain synchronized.
- Execute TASK-003 before documentation closeout so the final message examples reflect the real graph summary.
- Execute TASK-005 only after changed-stack gates and staged-diff review pass.
