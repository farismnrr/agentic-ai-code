# Plan 044A — GitHub Issue Lifecycle

**Status:** SOURCE COMPLETE / REVIEW-PR PENDING
**Parent:** [Plan 044](044-github-repository-operations-security-roadmap.md)
**Depends on:** Plan 043 CLOSED / VERIFIED / MERGED / DEPLOYED and Plan 040 forge boundary

## Goal

Add a bounded structured GitHub issue lifecycle to the native MCP relay so agents can discover, inspect, create, update, comment on, close, and reopen issues in the validated repository without arbitrary GitHub API access or credential exposure.

## Success criteria

The source implementation exposes exactly these seven new tools:

- `issue_list`
- `issue_get`
- `issue_create`
- `issue_update`
- `issue_comment`
- `issue_close`
- `issue_reopen`

Read tools are bounded network reads through the privileged forge bridge. Mutation tools are explicit external mutations, never silently retried, and verify observable post-state before reporting success where possible.

## Scope

### In scope

- repository-scoped issues only;
- state-filtered bounded listing;
- detailed single-issue view;
- create issue with validated repository remote;
- update issue (title/body/labels) with verified post-state;
- add comment with bounded content;
- close issue with normalized reason and optional comment;
- reopen issue with optional comment;
- clean decomposition of `git/forge/` into modular files (`common.rs`, `change_requests.rs`, `issues.rs`, `issues/model.rs`, `issues/validation.rs`).

### Out of scope

- issue deletion or transfer;
- project-board/project-field mutation;
- milestone mutation;
- assignee mutation;
- issue type mutation;
- dependencies, sub-issues, pin/unpin, lock/unlock;
- organization/user-wide issue queries;
- arbitrary GitHub search syntax passthrough;
- arbitrary `gh issue` flags;
- generic REST/GraphQL/`gh api` model-facing tools.

## Constraints and decisions

1. Prefer high-level `gh issue` direct-argv commands for this child because GitHub CLI already provides list/view/create/edit/comment/close/reopen and keeps credential use inside the existing privileged forge process.
2. Repository selection is always generated from the validated Git remote; model input cannot provide owner/repo or arbitrary URL.
3. `issue_list` returns summaries only; it does not return issue bodies or comments.
4. `issue_get` may return a body, but body bytes are hard capped and no comment thread is included.
5. Labels are bounded typed strings. Do not add project or assignee support merely because `gh issue` supports those flags.
6. Issue URLs must match `https://github.com/{validated-owner}/{validated-repository}/issues/{number}`.
7. GitHub's issue APIs can conceptually include pull requests; use `gh issue` behavior / normalized validation so the MCP issue tools remain issue-oriented and never silently treat PR objects as ordinary issues.
8. `issue_close` accepts a normalized close reason enum and verifies final state/reason after mutation.
9. `issue_create` and `issue_comment` are non-idempotent and must not be automatically replayed after an ambiguous result.

## PHASE-01 — Forge module decomposition without behavior change

**Goal:** Create safe ownership space for Plan 044 without regressing Plan 040 change-request behavior or violating maintainability budgets.

**Dependencies:** none

### TASK-001 — Split existing forge responsibilities behind the same facade

**Outcome:** `git/forge.rs` becomes a thin facade while existing change-request behavior moves to a nested forge module with no model-facing contract change.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge.rs`
- Create: `packages/rust-tools/application/src/git/forge/common.rs`
- Create: `packages/rust-tools/application/src/git/forge/change_requests.rs`
- Modify only if shared helper extraction is necessary: `packages/rust-tools/application/src/git/forge_process.rs`

**Steps:**
- [x] Move existing PR/change-request DTOs and operations into `forge/change_requests.rs` while preserving public dispatch signatures.
- [x] Move genuinely shared bounded text/JSON/repository identity helpers into `forge/common.rs`; do not create generic abstractions that only wrap one call.
- [x] Keep credential execution in the existing `forge_process` boundary.
- [x] Preserve all existing change-request validation, merge preconditions, output caps, static error behavior, and URL/repository identity checks.
- [x] Confirm `application/src/git/` remains within the direct-file budget; nested `git/forge/` owns the new responsibility growth.

**Validation:**
- `bash scripts/verify-040de-forge-contract.sh` → existing change-request contract still passes.
- `cargo test --workspace` → no Rust regression.
- `pnpm verify:commit` → architecture/maintainability/lint/type gates pass.

**Commit boundary:** `refactor(forge): split github domain adapters`

**Phase exit criteria:**
- [x] No change to existing PR/change-request MCP schemas or behavior.
- [x] No maintained source file/folder budget regression.

## PHASE-02 — Bounded issue read contract

**Goal:** Add normalized issue list/get reads without exposing arbitrary GitHub query surfaces.

**Dependencies:** PHASE-01

### TASK-002 — Implement issue DTOs and list/get operations

**Outcome:** Safe issue summaries and one-issue detail can be retrieved from the validated GitHub repository.

**Files:**
- Create: `packages/rust-tools/application/src/git/forge/issues.rs`
- Modify: `packages/rust-tools/application/src/git.rs`

**Steps:**
- [x] Define a bounded `IssueSummary` containing only stable workflow fields such as number, title, state/state reason, URL, labels, author login, created/updated/closed timestamps and comment count where available.
- [x] Define `IssueDetail` as summary + bounded body; do not include the full comment thread.
- [x] Implement `issue_list` with a repository-owned maximum result count, state enum (`open|closed|all`), optional bounded label filters, deterministic ordering from provider output, and `truncated` when applicable.
- [x] Implement `issue_get` for one positive issue number.
- [x] Reject malformed issue numbers, labels, provider JSON, oversized title/body/label arrays, and repository-mismatched URLs.
- [x] Ensure a pull-request-shaped provider object is rejected or normalized out of the issue-only surface rather than silently returned as an issue.

**Validation:**
- Add `scripts/verify-044a-issue-contract.sh` covering schemas, annotations, forbidden model-facing fields, malformed numbers/state/labels, provider identity rejection, body/output bounds, and PR-vs-issue discrimination.
- Candidate relay `tools/list` shows `issue_list` and `issue_get` exactly once.

**Commit boundary:** `feat(044a): add bounded github issue reads`

## PHASE-03 — Explicit issue mutation lifecycle

**Goal:** Add narrow issue mutation operations with typed arguments and verified external state.

**Dependencies:** PHASE-02

### TASK-003 — Implement issue create/update/comment

**Outcome:** Agents can create and maintain issue content without arbitrary CLI/API flags.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/issues.rs`

**Steps:**
- [x] `issue_create`: require bounded non-empty title; bounded body defaults to empty; allow only bounded label names as optional metadata.
- [x] Parse the created issue URL/number and immediately re-read it through the normalized issue-get path before reporting success.
- [x] `issue_update`: support only explicit title, body, add-labels, and remove-labels; reject empty update objects.
- [x] Verify issue still exists in the validated repository after update and return normalized post-state.
- [x] `issue_comment`: require bounded non-empty body; parse/validate returned comment identity when available; return bounded mutation evidence rather than raw command output.
- [x] Mark create/update/comment as non-idempotent external mutations where appropriate; no internal auto-retry after uncertain completion.

**Validation:**
- deterministic fake/fixture `gh` acceptance proves direct argv shape, repository binding, bounded content, static error mapping, and no arbitrary flags.
- negative cases include newline/NUL/oversized text, invalid label arrays, owner/repo injection attempts, and provider URL mismatch.

**Commit boundary:** `feat(044a): add github issue mutations`

### TASK-004 — Implement close/reopen state transitions

**Outcome:** Issue completion can be represented truthfully in GitHub after work is merged.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/issues.rs`

**Steps:**
- [x] `issue_close` requires issue number and accepts only normalized reasons `completed`, `not_planned`, or `duplicate`.
- [x] Require a positive `duplicate_of` issue number when reason is `duplicate`; reject it for unrelated reasons unless GitHub semantics justify otherwise.
- [x] Optional closing comment is bounded and explicitly included in the same operation; do not expose arbitrary close flags.
- [x] Re-read the issue and verify closed state/state reason before success.
- [x] `issue_reopen` reopens one issue, optionally with one bounded comment only if this remains a single atomic high-level GitHub operation; otherwise keep comment separate through `issue_comment`.
- [x] Re-read and verify open state before success.

**Validation:**
- deterministic transition fixture covers open→closed→open, duplicate validation, already-closed/already-open behavior, and state mismatch fail-closed behavior.

**Commit boundary:** `feat(044a): complete github issue lifecycle`

## PHASE-04 — MCP catalog, policy, UX classification, and acceptance

**Goal:** Make the seven tools safe first-class MCP capabilities across every policy/presentation owner.

**Dependencies:** PHASE-03

### TASK-005 — Register issue tools and capability effects

**Outcome:** Tool schemas, annotations, hook effects, first-party approval policy and UI summaries agree.

**Files:**
- Create: `packages/rust-tools/interfaces/src/mcp/catalog/forge.rs`
- Modify: `packages/rust-tools/interfaces/src/mcp/catalog.rs`
- Modify: `packages/rust-tools/application/src/hooks/policy.rs`
- Modify: `shared/utils/capability-policy.ts`
- Modify if required for safe summaries: `app/utils/tool-presentation.ts`
- Modify/add deterministic acceptance: `scripts/verify-044a-issue-contract.sh`

**Steps:**
- [x] Refactor `tool_catalog()` minimally so existing declarations remain in place while Plan-044 forge declarations come from the nested catalog module `catalog/forge.rs`.
- [x] Declare strict JSON schemas with `additionalProperties:false` and no `owner`, `repository`, arbitrary `url`, command, args, endpoint, method, header, or API-path inputs.
- [x] Mark `issue_list/get` read-only + open-world.
- [x] Mark issue mutations non-read-only, destructive/effectful according to existing external-mutation convention, and open-world.
- [x] Map issue reads to `network_read + privileged_bridge`; mutations to `network_read + network_write + external_mutation + privileged_bridge` in both policy owners.
- [x] Update malformed-input checks for positive issue numbers and mutation-required fields.
- [x] Ensure model/UI summaries show issue number/title/state intent without echoing full bodies/comments.

**Validation:**
- `bash scripts/verify-044a-issue-contract.sh` → PASS.
- existing `bash scripts/verify-040de-forge-contract.sh` → PASS.
- existing capability/UX verification scripts affected by new tools → PASS.
- `cargo test --workspace` → PASS.
- `pnpm verify:commit` → PASS.

**Commit boundary:** `feat(044a): expose safe github issue tools`

## PHASE-05 — Documentation and merge handoff

**Goal:** Merge source safely while keeping deployment status truthful.

**Dependencies:** PHASE-04

### TASK-006 — Document source-complete 044A and integrate by PR

**Outcome:** 044A source is reviewed/merged, but live connector proof remains explicitly pending until Plan 044D.

**Files:**
- Modify: `packages/rust-tools/README.md`
- Modify: `docs/external-mcp.md`
- Modify: this plan
- Modify parent Plan 044 status table/todo
- Modify `.agents/memories/README.md` only if a durable security/architecture invariant changed beyond what source/docs already state.

**Steps:**
- [x] Update documented tool surface and issue lifecycle constraints.
- [x] Run mandatory closeout review.
- [x] Create a short-lived implementation branch from current `main`; do not commit implementation directly to `main`.
- [x] Stage only Plan-044A-owned changes; preserve unrelated user changes.
- [x] Push and create PR targeting `main`, recording exact local verification.
- [ ] Review exact pushed head and squash-merge only when authorized and clean.
- [ ] Mark 044A `MERGED / LIVE VERIFICATION PENDING`, not CLOSED, until 044D deployment proves the connector surface.

**Validation:**
- exact merged `main` contains the seven issue tools and passes required local verification at the reviewed boundary.

**Commit boundary:** normal implementation PR squash merge to `main`.

## Risks and rollback

- **Provider field drift:** typed parse fails closed; rollback the source commit/PR rather than weakening parsing to arbitrary JSON.
- **Issue/PR ambiguity:** use issue-specific high-level CLI and explicit provider-shape checks; never return PRs as issue results accidentally.
- **Duplicate create/comment after uncertain network completion:** no automatic retry; caller must inspect repository state before repeating.
- **Label breadth creates extra scope:** keep labels bounded and omit projects/assignees/milestones.
- **Forge refactor regression:** existing Plan-040 acceptance is a blocking gate before issue features proceed.

## Final 044A acceptance criteria

- [x] Seven issue tools exist exactly once.
- [x] Existing 77-tool baseline is otherwise preserved (84 tools total in v9 catalog).
- [x] Existing change-request lifecycle passes unchanged.
- [x] No generic GitHub API/CLI passthrough exists.
- [x] No GitHub credential becomes available to ordinary terminal execution.
- [x] Issue list/get outputs are bounded and repository-validated.
- [x] Issue create/update/comment/close/reopen are direct-argv, typed, bounded, and correctly effect-classified.
- [x] Close/reopen report verified post-state.
- [x] deterministic Plan-044A acceptance passes.
- [x] `cargo test --workspace` passes.
- [x] `pnpm verify:commit` passes.
- [ ] source PR is merged to `main`.
- [ ] live relay/external MCP client proof remains explicitly pending for 044D.

## Handoff

After 044A is merged source-clean, continue to [Plan 044B](044b-github-actions-observability.md). Do not deploy/restart solely for 044A unless the operator explicitly asks for an intermediate deployment.
