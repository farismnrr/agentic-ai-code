# Plan 060 — Industrial-Grade Closure and Hardening

**Status:** IMPLEMENTED / LOCAL ACCEPTANCE PASSED — FINAL COMMIT PENDING
**Goal:** Close the remaining repository quality, security, testing, runtime-hardening, documentation, and operational-hygiene gaps identified by the 2026-08-30 deep review without restarting the systemd relay or pushing any Git branch.
**Success Criteria:** The repository reaches a defensible industrial-grade local closure state: least-privilege database behavior is implemented and regression-tested without mutating production credentials, critical web security flows are behaviorally tested rather than source-inspected only, Plan 050 and related runtime paths receive the strongest available local/Nuxt E2E acceptance, Docker/runtime hardening is improved, Rust dependency auditing is reproducible, high-value maintainability hotspots are reduced only where responsibility boundaries justify it, stale plan state is reconciled truthfully, and the final branch is clean with all task-owned changes committed locally and not pushed.

## Scope

### In scope

- Plan 049 closure work that can be implemented safely in-repository:
  - separate least-privilege application-runtime and migration-role policy;
  - deterministic PostgreSQL role/grant acceptance using disposable/local database infrastructure;
  - migration/runtime configuration and documentation required to prevent application use of a superuser credential;
  - HTTP/API/database adversarial regression coverage;
  - truthful 049F/049G/049 parent status reconciliation based on evidence actually obtained.
- Replace or supplement high-value source-string security tests with real behavioral tests for authentication, session, CSRF, IDOR/ownership, content-type, cache, and database post-state semantics.
- Plan 050 activity-ledger reliability/security acceptance through available local and Nuxt E2E paths, including restart/recreate of the Nuxt application when useful.
- Truthful closure/reconciliation of Plans 036, 044, 056, and 058 where existing evidence and currently available runtime boundaries permit it.
- Docker production hardening: non-root runtime, deterministic dependency installation, and reduced runtime dependency surface where compatible with the existing OpenTelemetry preload architecture.
- Reproducible Rust dependency/advisory auditing without relying on a read-only advisory database location.
- Targeted maintainability refactors for files with verified independent reasons-to-change; no line-count-driven splitting.
- Small type-safety/panic-boundary cleanup when directly justified by reviewed production code.
- Local branch and plan/documentation hygiene relevant to active repository truth.
- Final repository validation and local commit(s).

### Out of scope

- Any `systemctl`, `systemd-run`, service restart/reload/stop/start, or modification of the running systemd relay.
- Installing/replacing the deployed relay binary or changing relay systemd drop-ins.
- Production database credential rotation, production grants, production role mutation, or other irreversible database operations without separate explicit approval.
- Git push, pull request creation, merge, remote branch deletion, release, tag mutation, or deployment.
- Fabricating closure for external MCP/OAuth/GitHub acceptance when the required authenticated fixture is unavailable.
- Large architecture rewrites, new frameworks, new state-management libraries, new databases, new CI infrastructure, or verification-script proliferation.
- Refactoring security-sensitive Rust files solely to satisfy a line-count target.

## Current State

Verified on 2026-08-30 before Plan 060 creation:

- Base branch `main` was clean at `df5f920821bcca9e031c7548f2537d82f1947e10`.
- Task branch: `fix/060-industrial-grade-closure`.
- `pnpm guardrail:all` passes, including web lint/typecheck/unit tests and Rust fmt/Clippy/typecheck/tests.
- JavaScript production dependency audit reports no known vulnerabilities at the reviewed state.
- Fresh RustSec audit is reproducible through a workspace-local temporary Cargo home; the final audit found no RustSec vulnerabilities (one transitive yanked `chacha20 0.10.1` warning remains upstream through `rand 0.10.2`, not a security advisory).
- Plan 036 remains partially verified with authenticated external OAuth/MCP acceptance unproven.
- Plan 044 remains partially verified with live authenticated GitHub issue/Actions/security acceptance unproven.
- Plan 049 remains OPEN because the configured database runtime credential is a PostgreSQL superuser and least-privilege production-role acceptance is unproven.
- Plan 050 is implemented with local gates passing but live composed acceptance remains unproven.
- Plan 056 status appears stale relative to subsequent Plan 057 Telegram topic-routing evidence and needs evidence-based reconciliation.
- Plan 058 implementation is merged into current history but its plan still reports deployment pending.
- Plan 059 is complete and is not reopened by this plan.
- The web test suite contains useful runtime tests but several high-value account/security tests still inspect source text instead of exclusively proving behavior through request/database boundaries.
- Docker runtime now runs as the explicit `node` user, installs with a frozen lockfile, prunes devDependencies before packaging, and keeps the production dependency tree required by the standalone OTel preload.
- The maintainability gate passes but reports multiple production files in the 400–500 line review band; those are review signals, not automatic violations.

## Constraints & Decisions

- **Hard runtime boundary:** Never restart, reload, stop, start, reconfigure, or redeploy the systemd relay during Plan 060.
- **Allowed E2E runtime action:** The Nuxt application/container may be rebuilt, restarted, or recreated when needed for end-to-end verification.
- **Delivery boundary:** Stop after local commit(s). Do not push.
- **Database safety:** Prove least privilege against disposable/local PostgreSQL roles. Repository changes may prepare production-safe role separation, but no production credential/grant mutation occurs in this plan without separate explicit approval.
- **Truthful closure:** A plan blocked by an unavailable external authenticated fixture remains blocked; Plan 060 should reduce ambiguity and stale documentation, not convert missing evidence into a false pass.
- **Testing direction:** Prefer behavior-level tests through handlers/application/database boundaries. Source-contract checks remain only where they protect a static invariant that runtime testing cannot express economically.
- **Maintainability direction:** Split code only after identifying independent reasons-to-change and stable ownership boundaries. Security-sensitive low-level code may remain large when cohesion and auditability are better than abstraction.
- **Verification architecture:** Use existing `test/`, package-local Rust tests, `pnpm guardrail`, and focused existing commands. Do not reintroduce plan-numbered verification scripts.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
|---|---|---|---|
| PHASE-01 | Re-baseline active gaps and test architecture | none | Exact affected routes/modules/tests and safe local fixtures are mapped |
| PHASE-02 | Close database least-privilege implementation gap | PHASE-01 | Runtime/migration roles are separated by policy/config and disposable DB acceptance proves grants |
| PHASE-03 | Upgrade critical web security tests to behavioral acceptance | PHASE-01, PHASE-02 where DB fixtures overlap | Critical auth/session/ownership/browser-security behavior is request/database tested |
| PHASE-04 | Harden Docker/runtime and dependency audit reproducibility | PHASE-01 | Non-root/minimal deterministic runtime and reproducible JS/Rust audits pass |
| PHASE-05 | Validate Plan 050 and currently available composed runtime paths | PHASE-01–04 | Activity/retry/recovery/Nuxt E2E evidence is fresh without touching systemd relay |
| PHASE-06 | Targeted maintainability and type/panic-boundary cleanup | PHASE-01 | Only justified hotspots are simplified and all affected tests remain green |
| PHASE-07 | Reconcile open/stale plan truth and repository hygiene | PHASE-02–06 | 036/044/049/050/056/058 statuses exactly match fresh evidence; no stale active-state claims remain |
| PHASE-08 | Final industrial-grade acceptance and local commit | PHASE-01–07 | Full allowed validation is green, worktree is task-clean, changes are committed locally, nothing is pushed |

## PHASE-01: Re-baseline Active Gaps and Test Architecture

**Goal:** Establish exact implementation/test ownership before changing production code.
**Dependencies:** none

### TASK-001: Map critical web security routes to runtime boundaries

**Outcome:** A verified matrix of handlers, application services, database operations, and existing tests for account/security flows.

**Files:**
- Inspect: `server/api/auth/**`
- Inspect: `server/api/devices/**`
- Inspect: `server/api/security/**`
- Inspect: `server/api/workspaces/**`
- Inspect: `server/api/activity/**`
- Inspect: `server/middleware/**`
- Inspect: `server/application/**`
- Inspect: `server/infrastructure/database/**`
- Inspect: `test/unit/**`

**Steps:**
- [ ] Identify tests that only assert source strings for security-critical behavior.
- [ ] Identify existing handler/application seams that allow direct request-level tests without spinning up a parallel framework.
- [ ] Identify required disposable PostgreSQL fixture support and reuse existing test helpers where available.
- [ ] Map ownership/IDOR, CSRF/origin, content-type, cache, reset/session/MFA, and activity-ingest boundaries.

**Validation:**
- Review matrix has one implementation owner and one intended behavioral test boundary for every critical invariant selected for Plan 060.

**Commit boundary:** none; inspection only.

### TASK-002: Re-audit Plan 049/050/036/044/056/058 evidence

**Outcome:** Fresh distinction between source-complete, locally proven, live-proven, stale documentation, and externally blocked items.

**Files:**
- Inspect/Modify later: `.agents/plans/036-public-remote-mcp-and-oauth-interoperability.md`
- Inspect/Modify later: `.agents/plans/044-github-repository-operations-security-roadmap.md`
- Inspect/Modify later: `.agents/plans/044a-github-issue-lifecycle.md`
- Inspect/Modify later: `.agents/plans/044b-github-actions-observability.md`
- Inspect/Modify later: `.agents/plans/044c-github-security-alert-visibility.md`
- Inspect/Modify later: `.agents/plans/044d-controlled-actions-mutations-and-closure.md`
- Inspect/Modify later: `.agents/plans/049-account-and-application-security-hardening-roadmap.md`
- Inspect/Modify later: `.agents/plans/049f-http-api-and-database-security-hardening.md`
- Inspect/Modify later: `.agents/plans/049g-adversarial-security-test-matrix-and-closure.md`
- Inspect/Modify later: `.agents/plans/050-workspace-activity-ledger-roadmap.md`
- Inspect/Modify later: `.agents/plans/056-telegram-task-completion-notifications.md`
- Inspect/Modify later: `.agents/plans/058-telegram-task-report-workspace.md`

**Steps:**
- [ ] Compare plan claims with current merged Git history and current source.
- [ ] Identify evidence that can be refreshed without relay deployment/restart.
- [ ] Preserve external blockers where the authenticated fixture is genuinely unavailable.

**Validation:**
- No implementation phase depends on an assumed external capability that cannot be exercised under current constraints.

**Commit boundary:** none; inspection only.

**Phase exit criteria:**
- [ ] Critical security behavior test matrix is verified.
- [ ] Active/stale plan evidence matrix is verified.
- [ ] No systemd relay action is required by later local phases.

## PHASE-02: Database Least-Privilege Closure

**Goal:** Remove application design dependence on PostgreSQL superuser privileges and prove runtime/migration role separation against a disposable database.
**Dependencies:** PHASE-01

### TASK-003: Define runtime and migration database role contracts

**Outcome:** One documented/configured least-privilege contract for the application runtime and a separate migration role.

**Files:**
- Inspect/Modify: `docs/security.md`
- Inspect/Modify: `.env.example`
- Inspect/Modify: database configuration modules discovered in PHASE-01
- Test: `test/**` or existing database test location discovered in PHASE-01

**Steps:**
- [ ] Define privileges required by normal runtime operations only.
- [ ] Define migration/DDL privileges separately.
- [ ] Reject or clearly fail configuration validation for known superuser-only development shortcuts where production mode can identify them safely without requiring privileged introspection on every startup.
- [ ] Preserve local developer ergonomics without weakening production guidance or acceptance.

**Validation:**
- Configuration/documentation clearly separates runtime and migration credentials and contains no secret values.

**Commit boundary:** `security(db): define least-privilege runtime roles`

### TASK-004: Add disposable PostgreSQL least-privilege acceptance

**Outcome:** Automated behavioral proof that the runtime role can perform required application CRUD but cannot perform prohibited privileged DDL/role operations.

**Files:**
- Test: existing database/integration test location discovered in PHASE-01
- Modify only if required: database test helpers/config

**Steps:**
- [ ] Create roles/database objects only inside disposable/local PostgreSQL acceptance setup.
- [ ] Prove runtime-required reads/writes succeed.
- [ ] Prove schema ownership, role creation, database creation, extension/privilege mutation, or equivalent privileged operations fail.
- [ ] Prove migration role can perform the narrow required migration operation.
- [ ] Ensure credentials remain test-local and are never logged/committed.

**Validation:**
- Focused DB acceptance passes on a disposable database.
- No production database connection is mutated.

**Commit boundary:** `test(db): prove least-privilege runtime access`

**Phase exit criteria:**
- [ ] Application no longer requires a superuser runtime contract by design.
- [ ] Disposable DB behavior proves allowed and denied privilege boundaries.
- [ ] Production credential rotation remains explicitly separate unless independently approved.

## PHASE-03: Behavioral Web Security Acceptance

**Goal:** Make critical security guarantees executable at request/database behavior level rather than source-text presence only.
**Dependencies:** PHASE-01; reuse PHASE-02 fixture where appropriate.

### TASK-005: Behavioral account recovery and session tests

**Outcome:** Request/database tests prove anti-enumeration, token expiry/replay, session invalidation, and fresh-auth semantics.

**Files:**
- Modify/Test: `test/unit/account-recovery-security.test.ts`
- Modify/Test: `test/unit/account-security-runtime.test.ts`
- Add focused integration tests under `test/` only if the current runner requires a separate file.
- Modify application code only when a real defect is found.

**Steps:**
- [ ] Exercise existing/non-existing forgot-password requests and compare public semantics.
- [ ] Exercise valid/expired/replayed reset tokens and verify database post-state.
- [ ] Verify successful reset increments/revokes session generation behavior.
- [ ] Verify fresh-auth expiry behavior through the application boundary.
- [ ] Retain static source checks only for invariants that are truly compile/source contracts.

**Validation:**
- Focused tests pass using real application/database behavior.

**Commit boundary:** `test(auth): exercise recovery and session behavior`

### TASK-006: Behavioral authorization, CSRF, content, and cache tests

**Outcome:** Request-level tests prove owner isolation and browser mutation policy.

**Files:**
- Test: `test/**`
- Inspect/Modify if defects found: relevant `server/api/**`, middleware, application services.

**Steps:**
- [ ] Prove cross-user access is denied for selected high-value workspace/activity/session resources.
- [ ] Prove state-changing cookie-auth routes reject invalid cross-origin requests according to current policy.
- [ ] Prove unsupported mutation content types fail closed.
- [ ] Prove authenticated sensitive responses use the intended no-store/cache policy.
- [ ] Prove generic error handling does not expose raw provider/database/internal details.

**Validation:**
- Behavioral tests cover the Plan 049 adversarial matrix selected during PHASE-01.

**Commit boundary:** `test(security): add request-level adversarial coverage`

**Phase exit criteria:**
- [ ] High-value security invariants are not protected solely by source-string tests.
- [ ] IDOR/CSRF/content/cache/session/recovery behavior has deterministic regression coverage.

## PHASE-04: Docker and Dependency Hardening

**Goal:** Reduce runtime privilege/supply-chain surface while preserving the existing Nuxt and OpenTelemetry runtime contract.
**Dependencies:** PHASE-01

### TASK-007: Harden the Nuxt runtime image

**Outcome:** Production container runs unprivileged with deterministic package installation and a minimized dependency/runtime footprint compatible with OTel preload.

**Files:**
- Modify: `Dockerfile`
- Modify if required: `.dockerignore`, package scripts/config
- Test: existing container/Nuxt checks

**Steps:**
- [ ] Add an explicit unprivileged runtime user/group and ownership strategy.
- [ ] Make lockfile fidelity explicit during image build.
- [ ] Determine the smallest reliable runtime dependency copy/prune strategy that still allows `otel-preload.mjs` and Nitro to resolve required packages.
- [ ] Avoid adding a new package manager/runtime layer unless the existing pnpm/Nitro mechanisms cannot satisfy the requirement.
- [ ] Build and run the image locally; verify writable paths required by the application remain functional without root.

**Validation:**
- `docker compose build app` or repository-equivalent build succeeds.
- Recreated Nuxt app responds successfully and key authenticated/unauthenticated routes behave as expected where fixtures are available.
- Runtime process is non-root.

**Commit boundary:** `security(container): harden nuxt runtime image`

### TASK-008: Make dependency security audits reproducible

**Outcome:** JS and Rust dependency vulnerability audits can run fresh without relying on a read-only global advisory database.

**Files:**
- Modify only if required: package scripts/docs/tool configuration
- Do not create plan-numbered verification scripts.

**Steps:**
- [ ] Re-run production JS audit with lockfile/current dependencies.
- [ ] Configure Rust advisory auditing to use a writable task-local/cache location or supported no-cache/update behavior.
- [ ] Record actionable advisories only; do not add forced overrides for non-actionable transitive findings without compatibility review.

**Validation:**
- Fresh JS audit completes.
- Fresh Rust advisory audit completes from current lockfile.

**Commit boundary:** `chore(security): make dependency audits reproducible`

**Phase exit criteria:**
- [ ] Nuxt runtime is explicitly non-root.
- [ ] Build/install is lockfile-deterministic.
- [ ] Runtime dependency footprint is no broader than required by the verified OTel/Nitro contract.
- [ ] JS and Rust dependency audits complete freshly.

## PHASE-05: Plan 050 and Available Runtime Acceptance

**Goal:** Exercise the current activity/task/recovery system as deeply as possible without changing or restarting the systemd relay.
**Dependencies:** PHASE-01–04

### TASK-009: Refresh local activity reliability/security acceptance

**Outcome:** Fresh tests prove journal durability, retry/recovery, async progress, idempotency, redaction, and ownership contracts.

**Files:**
- Test/Modify if defects found: activity/task tests under `test/` and `packages/rust-tools/**/tests/`
- Inspect/Modify if defects found: `server/infrastructure/database/activity.ts`, relay activity modules, task progress modules.

**Steps:**
- [ ] Re-run existing activity, async progress, restart-recovery, and notification tests.
- [ ] Add missing behavioral regression only where the deep review exposes a real uncovered failure mode.
- [ ] Verify activity source authentication/authorization distinction remains correct.
- [ ] Verify no source/diff/credential payload leaks into general telemetry.

**Validation:**
- Focused web and Rust activity/task suites pass.

**Commit boundary:** `test(activity): close reliability acceptance gaps`

### TASK-010: Recreate Nuxt and run allowed E2E checks

**Outcome:** Fresh web-runtime evidence after Plan 060 changes without touching the systemd relay lifecycle.

**Files:** none unless defects are found.

**Steps:**
- [ ] Rebuild/recreate only the Nuxt application/container.
- [ ] Verify root/app health and representative auth/settings/activity routes.
- [ ] Exercise currently available first-party relay/activity integration only if it works against the already-running relay without requiring relay restart or configuration mutation.
- [ ] Do not infer acceptance for new Rust changes that are not deployed to the running relay.

**Validation:**
- Nuxt runtime is healthy after recreate.
- E2E evidence clearly states which running-relay behaviors were actually exercised.

**Commit boundary:** none unless an E2E-discovered defect is fixed.

**Phase exit criteria:**
- [ ] Plan 050 local reliability/security evidence is fresh.
- [ ] Nuxt E2E is fresh.
- [ ] Any remaining live relay acceptance limitation is explicitly documented rather than hidden.

## PHASE-06: Targeted Maintainability and Boundary Cleanup

**Goal:** Reduce verified multi-responsibility hotspots without weakening security auditability or creating abstraction churn.
**Dependencies:** PHASE-01

### TASK-011: Review and refactor only justified hotspots

**Outcome:** High-value files with multiple independent reasons-to-change are split or simplified; cohesive security-critical files remain intact when appropriate.

**Candidate files to review, not automatic split targets:**
- `app/components/settings/SettingsMcpConnectionDialog.vue`
- `app/components/workspace/WorkspaceActivityView.vue`
- `packages/rust-tools/infrastructure/src/transport/mcp_http.rs`
- `packages/rust-tools/interfaces/src/mcp/catalog.rs`
- `packages/rust-tools/core/src/config.rs`
- `packages/rust-tools/application/src/git/mutation.rs`
- `packages/rust-tools/application/src/code.rs`

**Steps:**
- [ ] Map callers and independent reasons-to-change for each candidate before editing.
- [ ] Extract cohesive UI steps/state only when it reduces orchestration/presentation coupling.
- [ ] Preserve stable MCP/tool/security facades where callers depend on them.
- [ ] Prefer deletion/simplification over wrapper-only decomposition.
- [ ] Leave cohesive descriptor/syscall security code unsplit if splitting would reduce auditability.

**Validation:**
- `pnpm guardrail` maintainability and architecture checks pass.
- Affected focused tests pass.
- No new exception is added merely to hide a regression.

**Commit boundary:** `refactor: reduce verified responsibility hotspots`

### TASK-012: Tighten small production type/panic boundaries

**Outcome:** Remove avoidable broad `any` and convert avoidable recoverable startup panics to typed errors where this improves operational diagnostics without large redesign.

**Files:**
- Review: `server/infrastructure/observability/logger.ts`
- Review: `server/infrastructure/ai/langgraph/langgraph-chat.ts`
- Review: production Rust `.expect(...)` sites identified by PHASE-01 audit.

**Steps:**
- [ ] Replace broad TypeScript `any` only where concrete safe types are available.
- [ ] Distinguish invariant-only Rust `expect` from recoverable initialization failures.
- [ ] Convert only recoverable production initialization failures to bounded typed errors.

**Validation:**
- Web/Rust typecheck and focused tests pass.

**Commit boundary:** `refactor: tighten runtime type and startup boundaries`

**Phase exit criteria:**
- [ ] No hotspot is split for LOC alone.
- [ ] Any changed boundary has clearer ownership than before.
- [ ] Type/panic cleanup is behavior-preserving or has explicit regression coverage.

## PHASE-07: Plan Truth and Repository Hygiene

**Goal:** Make repository plans and active-state documentation reflect actual implementation/runtime evidence exactly.
**Dependencies:** PHASE-02–06

### TASK-013: Reconcile Plan 049 and Plan 050

**Outcome:** Parent/child statuses accurately reflect new local least-privilege and behavioral acceptance evidence plus any production-only remaining boundary.

**Files:**
- Modify: `.agents/plans/049-account-and-application-security-hardening-roadmap.md`
- Modify: `.agents/plans/049f-http-api-and-database-security-hardening.md`
- Modify: `.agents/plans/049g-adversarial-security-test-matrix-and-closure.md`
- Modify: `.agents/plans/050-workspace-activity-ledger-roadmap.md`
- Modify if repository convention requires: canonical `.agents` memory/status documentation.

**Steps:**
- [ ] Mark only acceptance proven by fresh local/E2E evidence complete.
- [ ] If production credential rotation remains unperformed, describe it as an operator deployment boundary rather than an implementation defect when source/local proof is complete.
- [ ] Keep Plan 050 live-runtime claims limited to paths actually exercised under the no-relay-restart constraint.

**Validation:**
- Plan text, source, tests, and Git history have no contradictory active-state claims.

**Commit boundary:** `docs(plans): reconcile security and activity closure`

### TASK-014: Reconcile Plans 036, 044, 056, and 058

**Outcome:** Remove stale implementation/deployment wording while preserving genuinely unavailable external acceptance blockers.

**Files:**
- Modify as evidence warrants: Plan 036, 044 family, 056, 058.

**Steps:**
- [ ] Reconcile 056 against Plan 057/live topic-routing evidence without double-counting acceptance.
- [ ] Reconcile 058 merged source and any Nuxt/runtime evidence obtained in PHASE-05.
- [ ] Preserve 036/044 authenticated external blockers unless a current safe fixture is actually available and exercised.
- [ ] Remove stale checkboxes that contradict verified merged implementation while keeping unproven live items open.

**Validation:**
- `pnpm guardrail` agent-doc/plan checks pass.

**Commit boundary:** `docs(plans): reconcile remaining closure state`

### TASK-015: Clean task-relevant local branch metadata when safe

**Outcome:** Repository active-branch view contains no clearly merged Plan-060-related temporary branches created during this initiative.

**Steps:**
- [ ] Do not delete unrelated historical branches merely for cosmetic cleanup unless merged-state safety and task ownership are unambiguous.
- [ ] Do not delete remote branches.
- [ ] Keep the active Plan-060 branch intact through final commit.

**Validation:**
- Current task branch remains correct and worktree contains only task-owned changes.

**Commit boundary:** none.

**Phase exit criteria:**
- [ ] Plan truth matches actual evidence.
- [ ] No false external/live closure claim is introduced.
- [ ] Repository remains on the dedicated Plan-060 branch.

## PHASE-08: Final Industrial-Grade Acceptance and Local Commit

**Goal:** Prove the final local candidate and stop at a clean local commit boundary.
**Dependencies:** PHASE-01–07

### TASK-016: Run final validation matrix

**Outcome:** Fresh final-state evidence across all affected stacks.

**Steps:**
- [ ] Run focused tests after each changed subsystem.
- [ ] Run `pnpm guardrail:all` on the final source state.
- [ ] Run production JS dependency audit.
- [ ] Run fresh Rust advisory audit.
- [ ] Build the Nuxt production image.
- [ ] Recreate/restart only the Nuxt app/container and run allowed E2E smoke/behavior checks.
- [ ] Verify Git diff for accidental secrets, unrelated files, generated artifacts, and stale plan claims.
- [ ] Verify no systemd relay lifecycle action occurred during the plan.

**Validation:**
- All applicable local gates green.
- Any unavailable external acceptance is documented as unavailable rather than failed/passed.

**Commit boundary:** none until final review.

### TASK-017: Commit locally and stop

**Outcome:** All Plan-060 task-owned changes are committed locally on `fix/060-industrial-grade-closure`, with no push.

**Steps:**
- [ ] Revalidate branch, repository identity, and clean staging intent.
- [ ] Stage only Plan-060-owned files.
- [ ] Create logical local commit(s) using repository conventions; consolidate only when history remains clear.
- [ ] Verify final `git status` is clean.
- [ ] Verify the branch has not been pushed by this execution.

**Validation:**
- Current branch is `fix/060-industrial-grade-closure`.
- Worktree is clean after commit.
- No push/PR/merge/release/deployment action was performed.

**Commit boundary:** final local Plan-060 commit(s).

**Phase exit criteria:**
- [ ] Final allowed verification is green.
- [ ] Plan 060 status reflects exact closure evidence.
- [ ] All changes are committed locally.
- [ ] Nothing was pushed.

## Risks & Rollback

- **Database role tests accidentally target a real database** → require disposable/local fixture identity before any role/grant mutation; abort on ambiguous database target.
- **Container non-root change breaks writable runtime paths** → identify required writable paths explicitly, adjust ownership narrowly, and revert the hardening change if correctness requires broader privilege until a safe path is designed.
- **Pruning `node_modules` breaks OTel preload resolution** → verify the preload in the built runtime image; fall back to the smallest proven dependency packaging mechanism rather than shipping an unverified prune.
- **Behavioral tests duplicate framework internals** → test application/request outcomes through existing seams; avoid recreating Nitro/auth internals in mocks.
- **Maintainability refactor weakens security boundaries** → preserve stable facades and existing adversarial tests; revert structural changes that increase indirection without reducing responsibility.
- **Plan closure overstates runtime evidence** → separate `implemented`, `locally verified`, `Nuxt E2E verified`, `running-relay verified`, and `external authenticated verified` states explicitly.
- **Systemd relay restart temptation during E2E** → treat relay lifecycle as a hard forbidden boundary for this plan; use only the already-running relay and document version mismatch limitations.

## Implementation evidence — 2026-08-30

- Critical HTTP mutation policy moved behind behavior-testable application functions: cookie-authenticated mutations require same-origin proof, explicit API bearer requests may omit browser `Origin`, and body-bearing API mutations require JSON. Prerendered `/` now receives the same static security-header baseline through route rules.
- Database runtime and migration credentials are separated. `ops/database/least-privilege.sql` defines the reviewed grant contract, disposable PostgreSQL proves runtime CRUD/sequence access and denial of schema creation/cluster privileges, and `ops/runtime-web-entry.mjs` verifies the configured role before importing Nitro. Runtime E2E proved a safe role reaches `Listening on`, while `postgres` superuser exits code 1 with no listener.
- Web behavior suite increased to 40 passing tests, adding HTTP policy, database role policy, fresh-auth, workspace ownership, and activity limit behavior while retaining static contract guards where useful.
- Docker production image builds successfully as non-root `node`, uses `pnpm install --frozen-lockfile`, prunes devDependencies, and preserves only the production dependency tree needed by Nitro plus standalone OTel preload.
- Candidate-container HTTP E2E returned root `200`, missing/cross-origin mutation `403`, invalid content type `400`, and same-origin unauthenticated mutation `401`; static landing headers were present.
- `pnpm audit --prod --audit-level=moderate` reports no known vulnerabilities. Fresh `cargo audit` with a workspace-local Cargo home reports no security vulnerabilities; it reports one upstream yanked-package warning for `chacha20 0.10.1` via `rand 0.10.2`.
- Plans 036 and 044 remain externally blocked because no authenticated external fixture was fabricated. Plan 049 repository/disposable acceptance is now stronger but production credential rotation remains operator-controlled. Plan 058 is confirmed merged through squash commit `cd1e608`; deployment/live worker acceptance remains pending. Plan 056 remains deployed with visible completion delivery pending operator configuration.
- No systemd relay action, production database role/credential mutation, Git push, PR, merge, release, or remote mutation occurred.

## Final Acceptance Criteria

- [x] One dedicated Plan-060 branch contains all task work.
- [x] Production application design no longer depends on PostgreSQL superuser runtime privileges.
- [x] Disposable PostgreSQL tests prove least-privilege allowed/denied behavior.
- [x] Critical account/session/authorization/browser-security guarantees have behavior-level regression coverage.
- [x] Plan 049 status is reconciled truthfully with local proof and any remaining production credential boundary.
- [x] Plan 050 local reliability/security acceptance is fresh; available Nuxt/runtime E2E is fresh.
- [x] Plans 036/044 retain external blockers unless actually proven; stale wording is removed.
- [x] Plans 056/058 accurately reflect merged/deployed/live evidence.
- [x] Nuxt production container runs as an explicit non-root user.
- [x] Production dependency installation is lockfile-deterministic.
- [x] Runtime dependency packaging is no broader than required by the verified Nitro/OTel contract.
- [x] Fresh JS and Rust dependency audits complete.
- [x] Targeted maintainability changes are responsibility-driven and pass architecture/maintainability gates.
- [ ] `pnpm guardrail:all` passes on final state.
- [x] Nuxt app/container may be recreated for E2E and remains healthy.
- [x] No systemd relay restart/reload/stop/start/redeploy occurs.
- [x] No production database privilege/credential mutation occurs without separate approval.
- [ ] Final worktree is clean after local commit(s).
- [x] No Git push, PR, merge, release, or remote mutation occurs.

## Execution Handoff

- Execute phases sequentially because PHASE-02/03 share database/security fixtures and PHASE-07 depends on evidence from all implementation phases.
- PHASE-04 can be developed independently after PHASE-01, but final container E2E belongs in PHASE-05/08.
- Keep the already-running systemd relay untouched for the entire initiative.
- Production database role/credential changes remain a separate operator action requiring explicit approval; Plan 060 should make the repository and disposable acceptance ready for that deployment boundary without performing it.
- Delivery ends after local commit(s) on `fix/060-industrial-grade-closure`; do not push.
