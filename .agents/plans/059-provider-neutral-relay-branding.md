# Provider-Neutral Relay Branding Implementation Plan

**Status:** IMPLEMENTED — WRITABLE REMOTE HISTORY AND RELEASE METADATA VALIDATED; PR PENDING REVIEW
**Goal:** Make the repository present a provider-neutral MCP relay identity across product copy, operational guidance, evidence paths, and external-client task reporting while preserving required protocol and SDK compatibility.
**Success Criteria:** No provider-specific product branding remains in user-facing or agent-facing copy; external task completion uses a generic source identity; provider adapters and persisted compatibility values continue to work; local policy and stack-aware verification pass.

## Scope

### In scope

- Generalize product, operator, agent, plan, contract, and evidence wording to use MCP relay and external-client terminology.
- Replace the external task-completion source discriminator with `external_mcp`.
- Preserve the standard `OpenAI Compatible` / `Anthropic Compatible` provider labels while retaining persisted compatibility values and adapter behavior.
- Remove provider-specific names from comments, examples, test fixtures, and internal evidence filenames where they are not runtime contracts.
- Keep a clean branch and record exact validation before pull request review and merge.

### Out of scope

- Replacing stable third-party SDK package names, wire headers, protocol fields, or persisted provider type values that are required for runtime compatibility.
- Rewriting hidden GitHub pull-request refs, forks, caches, or other external copies that are outside normal writable repository refs.
- Deploying or restarting any runtime service.

## Current State

- The working tree was clean on `main` before this branch was created.
- A tracked snapshot ZIP and complete Git bundle were created and validated before implementation.
- Product and agent-facing text contained provider-specific client names in current guidance and historical plans.
- The task-completion contract now distinguishes `nuxt` and the generic `external_mcp` source.
- Provider adapters use stable SDK and wire identifiers that must remain compatible.
- Writable branch/tag history and GitHub release metadata were separately updated after explicit approval.
- GitHub hidden pull-request refs remain outside normal force-update controls and retain historical copies.

## Constraints & Decisions

- Use generic terms such as `external MCP client`, `provider-compatible`, and `MCP relay` for presentation and guidance.
- Preserve runtime identifiers when they are part of an SDK import, HTTP header, database value, migration, or published protocol contract.
- Do not claim that local history rewriting erases remote copies; treat any later remote rewrite as a separate explicitly reviewed operation.
- Keep the change on a short-lived branch and use the repository guardrail before committing.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
|---|---|---|---|
| PHASE-01 | Neutralize current product, guidance, evidence, and test surfaces | none | Targeted repository scan shows only approved technical identifiers; focused contract tests are updated |
| PHASE-02 | Validate and package the implementation | PHASE-01 | Diff is clean, stack-aware guardrail passes, and branch is ready for review |

## PHASE-01: Neutralize Current Surfaces

**Goal:** Remove provider-specific branding from presentation and operational surfaces without breaking runtime contracts.
**Dependencies:** none

### TASK-001: Generalize source and UI terminology

**Outcome:** External task reports use `external_mcp`, provider settings retain standard compatibility labels, and comments/fixtures no longer identify a specific client.

**Files:**

- Modify: `server/application/task-notifications.ts`, `packages/rust-tools/infrastructure/src/notifications.rs`, `packages/rust-tools/infrastructure/src/transport/task_completion.rs`
- Modify: `shared/utils/providers.ts`, `app/composables/useConversations.ts`, `ops/remote-mcp/start-relay.sh`
- Test: `test/unit/task-notifications.test.ts`, `packages/rust-tools/infrastructure/tests/task_notifications.rs`, `packages/rust-tools/infrastructure/tests/security.rs`

**Validation:**

- Focused web task-notification test passes.
- Focused Rust notification/security tests pass.
- No user-facing client-specific label remains in the settings options.

**Commit boundary:** `refactor(branding): generalize relay client terminology`

### TASK-002: Generalize repository guidance and evidence

**Outcome:** Agent guidance, plans, contracts, prompts, examples, and evidence paths use provider-neutral terminology and point to the generic MCP client documentation.

**Files:**

- Modify: `.agents/**`, `ai-self/**`, `agent-prompts/**`, `.gitignore`, `.agents/skills/nuxt-ui/references/layouts/chat.md`
- Evidence path rename completed: `.agents/contracts/036-evidence/external-mcp-live-2026-08-16.md` now carries the generic external-client name

**Validation:**

- Agent-doc guardrail passes.
- Repository scan confirms no provider-specific branding in guidance or user-facing copy.
- Historical evidence remains marked as historical and does not gain new runtime claims.

**Commit boundary:** included in the focused branding commit unless review shows a separate documentation-only boundary is clearer.

## PHASE-02: Validate and Package

**Goal:** Prove the neutralization is internally consistent and ready for review.
**Dependencies:** PHASE-01

### TASK-003: Run scoped verification and review

**Outcome:** Changed web/Rust contracts, repository policy, maintainability, and test layout are validated with no unrelated files staged.

**Files:**

- Review: all files changed by PHASE-01

**Validation:**

- `git diff --check` passes.
- `pnpm guardrail` passes for the touched stacks.
- Targeted repository scans distinguish approved SDK/wire identifiers from branding text.

**Commit boundary:** `refactor(branding): generalize relay client terminology`

## Risks & Rollback

- Changing the task source discriminator can affect old queued rows or consumers → use the pre-change Git bundle and keep the change scoped; inspect all current producers/consumers before commit.
- Overgeneralizing technical compatibility identifiers can break provider selection → preserve SDK imports, wire headers, migrations, and persisted values; validate focused tests.
- Historical guidance may contain factual client-specific evidence → rewrite labels only and retain the original evidence status and limitations.
- A repository rewrite cannot erase external clones or hidden forge history → report those limits explicitly and keep backups for rollback.

## Final Acceptance Criteria

- [x] Product and agent-facing copy uses provider-neutral MCP relay terminology.
- [x] External task completion source is `external_mcp` end to end.
- [x] Required provider adapters and persisted compatibility contracts remain intact.
- [x] Focused tests and `pnpm guardrail` pass.
- [x] Writable remote branch/tag history and release metadata were updated; hidden PR-ref limits are recorded.
- [x] Working tree contains only task-owned changes and the branch is ready for review.

## Execution Handoff

- Execute TASK-001 before TASK-002 because source/test contract changes establish the terminology used by guidance.
- TASK-002 is mostly mechanical and can be reviewed file-by-file after the source contract is stable.
- TASK-003 blocks commit and any later PR action.
- Remote branch/tag rewrite and release metadata update are complete; PR review/merge and deployment remain separate approval checkpoints.
