# Plan 057 — Telegram Forum Topic Routing Implementation Plan

**Status:** CLOSED / VERIFIED


> **Superseded live architecture:** Plan 063 replaces this completion-coupled design with the explicit `telegram_send_message` MCP tool. This file is retained as historical implementation/evidence only.
**Goal:** Route the existing task-completion Telegram notification through the configured `Masih Awam` forum topic while preserving standalone relay ownership and the no-generic-Telegram-tool boundary.
**Success Criteria:** A configured `message_thread_id` survives relay-state persistence/restart, task-completion delivery sends it to Telegram, legacy relay state migrates safely, focused Rust tests and `pnpm guardrail` pass, and a live task-notification smoke test is accepted by topic `3775`.

## Scope

### In scope

- Add an optional Telegram forum-topic ID to the relay-owned notification configuration.
- Read and validate `TELEGRAM_HOME_CHANNEL_THREAD_ID` only during the one-time bootstrap/import path.
- Migrate existing relay SQLite state without losing the encrypted bot token or configured chat.
- Include `message_thread_id` in `sendMessage` only when configured.
- Update focused Rust tests, operator documentation, and the current Plan 057 evidence.
- Re-seed the deployed relay state for the verified `Masih Awam` target and perform a live topic delivery check after the reviewed merge.

### Out of scope

- A generic Telegram or HTTP MCP tool.
- Per-tool or activity notifications.
- Runtime dependency on Hermes or runtime reading of the Hermes environment.
- Changing Nuxt task-completion semantics or the Nuxt/Rust service boundary.
- Supporting arbitrary caller-supplied chat IDs, tokens, or topic IDs.

## Current State

- The relay stores an encrypted Telegram bot token, one fixed `chat_id`, and an optional `message_thread_id` in `telegram_configuration`.
- The deployed target is the verified `Masih Awam` forum supergroup with base chat ID `-1003975534063` and topic ID `3775`.
- The user supplied a forum-topic link ending in `/3775`; the Telegram Bot API accepts `message_thread_id` for forum topics, and a direct smoke test to topic `3775` succeeded with message ID `3776`.
- `TELEGRAM_HOME_CHANNEL_THREAD_ID` is parsed only during bootstrap, persisted in relay-owned state, and sent only by the internal task-completion sender.
- Runtime delivery remains task-level and relay-owned; the completion signal is not a generic Telegram capability.

## Constraints & Decisions

- Keep the bot token encrypted with the existing relay-owned key and never add credentials to source, tests, plans, logs, or PR text.
- Keep the configured recipient fixed and owner-controlled. The topic ID is configuration data, not an MCP argument.
- Treat an empty thread ID as `None` for backward compatibility; a non-empty value must be a positive Telegram integer within the Bot API integer range.
- Keep legacy databases readable by adding the nullable column during ledger initialization before loading credentials.
- Use the existing `sendMessage` integration and omit `message_thread_id` when unset so existing root/general behavior remains valid.
- The live target is the verified `Masih Awam` forum supergroup and topic `3775`; this is a forum supergroup target, not a Telegram broadcast channel.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
|---|---|---|---|
| PHASE-01 | Extend the relay configuration contract safely | none | Schema, parser, persistence, sender, and runtime wiring are implemented with no secret exposure. |
| PHASE-02 | Prove compatibility and behavior | PHASE-01 | Focused Rust tests cover import, migration, validation, restart, and request shaping; docs are aligned. |
| PHASE-03 | Deliver and verify the live topic route | PHASE-02 | Reviewed PR is merged, deployed binary is rebuilt from merged `main`, DB is re-seeded, service is healthy, and a topic delivery row is `sent`. |

## PHASE-01 — Extend the relay topic configuration

**Goal:** Carry an optional validated forum-topic ID from one-time bootstrap through encrypted relay state into Telegram `sendMessage`.
**Dependencies:** none

### TASK-001: Add optional topic configuration and migration

**Outcome:** Existing and fresh notification databases support a nullable `message_thread_id` without losing legacy rows.

**Files:**

- Modify: `packages/rust-tools/infrastructure/src/notifications/ledger.rs`
- Test: `packages/rust-tools/infrastructure/tests/task_notifications.rs`

**Steps:**

- [x] Add `message_thread_id: Option<i64>` to the relay-owned credential model.
- [x] Add the nullable column to the fresh schema and idempotently migrate legacy databases during ledger initialization.
- [x] Select, insert, and update the topic ID together with the encrypted token envelope and fixed chat ID.
- [x] Preserve owner-only permissions and existing encrypted token handling.

**Validation:**

- Focused Cargo test proves a legacy schema opens, imports, and reloads credentials with a topic ID.
- Raw database bytes do not contain the test token.

**Commit boundary:** `feat(relay): persist telegram forum topic`

### TASK-002: Parse, validate, and send the topic ID

**Outcome:** Bootstrap accepts `TELEGRAM_HOME_CHANNEL_THREAD_ID` and the sender includes it in the Telegram request when configured.

**Files:**

- Modify: `packages/rust-tools/infrastructure/src/notifications/dotenv.rs`
- Modify: `packages/rust-tools/infrastructure/src/notifications.rs`
- Modify: `packages/rust-tools/infrastructure/src/notifications/telegram.rs`
- Test: `packages/rust-tools/infrastructure/tests/task_notifications.rs`

**Steps:**

- [x] Parse an empty thread value as `None` and reject malformed, zero, negative, or out-of-range values.
- [x] Pass the parsed value through bootstrap storage and runtime sender construction.
- [x] Build the request body with `message_thread_id` only when configured.
- [x] Preserve the existing channel/supergroup target validation and no-caller-controlled destination boundary.

**Validation:**

- Focused tests prove valid `3775`, empty compatibility, and invalid values.
- Request-shaping coverage proves the topic field is present only for configured topic delivery.

**Commit boundary:** `feat(relay): route telegram notices to forum topics`

## PHASE-02 — Tests, docs, and review readiness

**Goal:** Establish deterministic regression proof and keep the plan/operator guidance truthful.
**Dependencies:** PHASE-01

### TASK-003: Complete focused validation and documentation

**Outcome:** The topic route is covered by the Rust test suite and current operator/agent documentation.

**Files:**

- Modify: `.agents/plans/057-telegram-forum-topic-routing.md`
- Modify: `.agents/memories/README.md` only if the durable routing invariant changes
- Modify: relevant human/operator documentation discovered during implementation
- Test: `packages/rust-tools/infrastructure/tests/task_notifications.rs`

**Steps:**

- [x] Run focused notification tests and Rust gates for the changed subsystem.
- [x] Run `pnpm guardrail` before commit and record the exact successful checks.
- [x] Review the diff for secret-bearing output, stale “thread ignored” documentation, and accidental generic Telegram-tool exposure.
- [x] Update this plan with implementation and validation evidence without recording tokens or private credential values.

**Validation:**

- `cargo test --manifest-path packages/rust-tools/infrastructure/Cargo.toml --test task_notifications` → 14 passed.
- `pnpm guardrail` → pass: Rust format, Clippy warnings-denied, typecheck, all workspace Rust tests, architecture, maintainability, and test-layout checks.
- `git diff --check` → pass.

**Commit boundary:** `docs(plan): record telegram topic routing verification` when documentation is separate from implementation.

## PHASE-03 — Reviewed deployment and live topic verification

**Goal:** Deploy the merged implementation and prove a real notification reaches topic `3775`.
**Dependencies:** PHASE-02

### TASK-004: Deliver through the protected repository workflow

**Outcome:** The implementation is merged to `main`, rebuilt from that merged commit, and installed at the effective systemd binary path.

**Files:**

- Runtime state only after merge: relay-owned SQLite configuration and installed binary; never commit these artifacts.

**Steps:**

- [x] Revalidate workspace identity, branch, status, and task-owned paths before Git writes.
- [x] Commit only reviewed task-owned source/tests/docs and push the short-lived branch.
- [x] Open and merge the PR targeting `main` under repository policy.
- [x] Return to `main`, verify clean checkout, build from merged `main`, compare installed/running binary identity, and restart the discovered user service.
- [x] Re-seed relay-owned state with the configured channel base ID and topic `3775` through the encrypted application path; do not store plaintext token material.

**Validation:**

- Merged `main` commit is the source of the deployed binary.
- `systemctl --user is-active ai-tools-relay.service` → `active`.
- `NRestarts=0` and `/health` returns HTTP 200.
- Relay logs do not report Telegram delivery disabled due to missing credentials.

**Commit boundary:** No runtime state is committed.

### TASK-005: Verify live topic delivery

**Outcome:** A synthetic task-completion delivery is accepted by Telegram for the configured forum topic and the relay ledger marks it `sent`.

**Files:**

- Runtime ledger evidence only; no source changes expected.

**Steps:**

- [x] Enqueue one clearly labeled bounded smoke-test notification through the relay delivery path.
- [x] Confirm the ledger status is `sent` with no retry/error category.
- [x] Confirm the sent message was accepted for topic `3775`; do not claim visual/UI proof unless independently observed.
- [x] Record the live evidence in this plan without logging token material.

**Validation:**

- `telegram_configuration` contains one configured row with the expected non-secret topic ID.
- The synthetic notification row has `status='sent'` and a non-null sent timestamp.
- Service remains active with no restart increment.

**Commit boundary:** No source commit expected.

### Deployment Evidence

- PR #193 was squash-merged into `main` at `4c72804`.
- The release binary was built from merged `main` and installed at
  `/home/farismnrr/.local/bin/ai-tools`; the build and installed binary both
  had SHA-256 `782e9bd9c0f4c32f1003f2fd00c36b6cd1f92aebaf1ab614e9087b33f936469e`.
- Relay-owned state contains one Telegram configuration row with the verified
  `Masih Awam` base chat and topic `3775`; no credential value is recorded here.
- `ai-tools-relay.service` restarted successfully with `MainPID=3847556`,
  `NRestarts=0`, and `/health` returned HTTP 200.
- A synthetic task notification queued after the restart was marked
  `status='sent'`, `attempts=0`, with no error category, proving the deployed
  worker accepted the topic-targeted Telegram request.

## Risks & Rollback

- Legacy database lacks the new column → initialize with an idempotent nullable-column migration before credential reads; rollback is a binary rollback that leaves the additive column unused.
- Invalid topic ID or a deleted topic → reject at bootstrap and preserve the prior valid root/general configuration; do not silently fall back from an explicitly configured invalid topic.
- Bot lacks permission for the forum topic → Telegram returns a permanent delivery failure; verify bot membership/admin rights before treating delivery as healthy.
- Runtime binary does not match reviewed `main` → compare SHA-256 of build and installed binary before restart; stop deployment if they differ.
- Secret-bearing test/log output → use redacted status probes only; never store tokens in plans, tests, PR text, or shell output.

## Final Acceptance Criteria

- [x] `TELEGRAM_HOME_CHANNEL_THREAD_ID=3775` is parsed, validated, persisted, and loaded after restart.
- [x] A configured topic request contains `message_thread_id=3775`; an unset topic omits the field.
- [x] Existing root/general configurations remain compatible.
- [x] No generic Telegram/HTTP MCP tool or per-tool notification is introduced.
- [x] Focused Rust tests and `pnpm guardrail` pass.
- [x] PR is merged, checkout returns cleanly to `main`, and the deployed binary matches merged `main`.
- [x] Live relay delivery marks one topic-targeted synthetic task notification `sent`.

## Execution Handoff

- Execute TASK-001 and TASK-002 together as the implementation boundary because schema and sender models must remain coherent.
- Execute TASK-003 after implementation tests are green; documentation updates stay in the same short-lived branch unless review identifies a separate docs-only commit.
- Execute TASK-004 only after local validation and staged-diff review pass.
- Execute TASK-005 after deployment and configuration reseeding; live delivery evidence is separate from static test proof.
