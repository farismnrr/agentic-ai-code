# Plan 044D — Controlled GitHub Actions Mutations and Plan 044 Closure

**Status:** SOURCE COMPLETE / V9 CONTRACT FROZEN / LIVE VERIFICATION PENDING
**Parent:** [Plan 044](044-github-repository-operations-security-roadmap.md)
**Depends on:** Plans 044A, 044B and 044C source merged / live verification pending

## Goal

Add only the three GitHub Actions mutations needed for practical engineering control, then perform the composed Plan-044 security review, contract versioning, integration, release build, relay restart, live MCP verification, and external MCP client connector rediscovery required to close the entire Plan 044 family.

## Success criteria

The source implementation exposes exactly these final three tools:

- `workflow_run_rerun`
- `workflow_run_cancel`
- `workflow_dispatch`

After integration, the complete planned Plan-044 surface contains 23 new tools over the 77-tool v8 baseline, for an expected 100-tool v9 catalog. The exact merged binary is installed and restarted, and the live authenticated connector proves representative issue, Actions and security operations.

## Scope

### In scope

- rerun an existing workflow run;
- rerun failed jobs only or one job when safely representable by one typed schema;
- cancel a queued/running workflow run;
- dispatch a workflow by positive workflow ID at a validated safe Git ref with bounded string inputs;
- post-mutation observation without false completion claims;
- complete Plan-044 contract/security/architecture acceptance;
- PR/merge/release-build/install/restart/live verification;
- connector rediscovery;
- final issue/plan/docs/memory closeout.

### Out of scope

- enable/disable/delete workflow;
- modify Actions repository/org policy;
- runner administration;
- artifact/cache deletion;
- deployment/environment approval;
- Actions secret/variable/environment management;
- security alert dismissal/resolve/reopen;
- arbitrary workflow file/YAML mutation as part of the tool call;
- arbitrary GitHub API/CLI passthrough;
- adding an Actions workflow to `ai-code` solely for acceptance.

## Mutation safety decisions

1. Every mutation derives repository identity from validated GitHub remote state.
2. Every tool is an explicit external mutation with `network_read + network_write + external_mutation + privileged_bridge` effects.
3. Rerun/cancel operations require positive numeric run IDs; optional job ID is also positive numeric.
4. `workflow_dispatch` uses a positive numeric workflow ID and a validated safe Git ref. No arbitrary workflow path from model input is needed.
5. Dispatch inputs are a bounded flat string map only: maximum 25 entries (matching current GitHub API limit), bounded key/value bytes, no nested JSON and no file/stdin indirection from model input.
6. Mutation success means GitHub accepted the request and the relay performed a bounded post-read. It does not mean the rerun/dispatch completed successfully.
7. No indefinite polling. Long-running workflow completion remains observable through existing read tools.
8. Non-idempotent dispatch must not be automatically replayed after an ambiguous transport result.
9. Live mutation acceptance must use an operator-approved disposable/safe GitHub repository/workflow. Do not mutate production workflows merely to prove the tool.

## PHASE-01 — Controlled workflow rerun and cancellation

**Goal:** Add run-state mutations through narrow direct-argv GitHub CLI operations with truthful post-observation.

**Dependencies:** none

### TASK-001 — Implement workflow rerun

**Outcome:** `workflow_run_rerun` can request rerun of all jobs, failed jobs, or one job without arbitrary CLI flags.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/actions.rs`
- Modify: `packages/rust-tools/application/src/git.rs`

**Steps:**
- [ ] Require positive `run_id`.
- [ ] Support an explicit bounded mode (`all`, `failed`, `job`) or equivalent schema; `job` requires positive `job_id` and incompatible combinations are rejected.
- [ ] Map to fixed direct argv `gh run rerun` arguments only.
- [ ] Do not expose debug logging flag by default; it changes workflow logging behavior and is outside this plan unless independently justified.
- [ ] After GitHub accepts the rerun, perform one or a small bounded number of `workflow_run_get` observations to capture attempt/status change when visible.
- [ ] Return normalized mutation evidence: requested mode, run ID, observed attempt/status/conclusion, and `observation_pending` when GitHub has not reflected the new attempt yet.
- [ ] Never claim the rerun passed merely because the rerun request succeeded.

**Validation:**
- fixture verifies exact argv for all/failed/job modes, invalid combinations, provider failure classification, no auto-retry and bounded post-read.

**Commit boundary:** `feat(044d): add workflow rerun control`

### TASK-002 — Implement workflow cancel

**Outcome:** `workflow_run_cancel` requests cancellation without pretending asynchronous cancellation is immediately complete.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/actions.rs`

**Steps:**
- [ ] Require positive run ID.
- [ ] Invoke only fixed `gh run cancel <id> --repo <validated-repo>` direct argv (or reviewed equivalent).
- [ ] Re-read run state once/boundedly after accepted cancellation.
- [ ] Return `cancel_requested` plus observed status/conclusion; allow transitional queued/in-progress/cancelling state without false failure.
- [ ] Already-completed/non-cancellable runs return a bounded provider/state classification, not raw CLI stderr.

**Validation:**
- fixture covers running→cancel requested, already completed, invalid ID, provider failure and delayed state reflection.

**Commit boundary:** `feat(044d): add workflow cancellation control`

**Phase exit criteria:**
- [ ] rerun/cancel have no arbitrary flags.
- [ ] asynchronous state is reported truthfully.

## PHASE-02 — Workflow dispatch

**Goal:** Trigger a reviewed dispatchable workflow without accepting arbitrary API requests or unconstrained input maps.

**Dependencies:** PHASE-01 and fixed API transport from 044C

### TASK-003 — Implement workflow dispatch with bounded inputs

**Outcome:** `workflow_dispatch` creates a workflow-dispatch event through one fixed GitHub endpoint and returns normalized run/request identity when GitHub provides it.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/actions.rs`
- Modify: `packages/rust-tools/application/src/git/forge_process.rs` / fixed API request enum as needed

**Steps:**
- [ ] Require positive workflow ID.
- [ ] Require a non-empty safe Git ref validated through existing Git ref/branch/tag safety rules; reject option-like values, `rev:path`, traversal-like syntax and arbitrary URL/SHA-expression tricks.
- [ ] Accept optional flat `inputs` object with max 25 entries; string keys/values only; independent key/value byte caps; reject control characters/NUL.
- [ ] Construct the fixed repository workflow-dispatch endpoint internally from validated owner/repo/workflow ID.
- [ ] Use fixed POST method and fixed JSON body shape `{ref, inputs}`; model input cannot set method/endpoint/headers.
- [ ] Parse returned run ID/URL only when current GitHub API provides them; validate returned URL against repository identity.
- [ ] If GitHub accepts dispatch but run identity is not immediately returned/observable, return accepted + bounded observation-pending state rather than guessing.
- [ ] Never automatically retry an ambiguous dispatch.

**Validation:**
- fixture asserts exactly one fixed endpoint/method, 25-input cap, key/value caps, malicious ref rejection, owner/repo injection rejection and no credential/log echo.

**Commit boundary:** `feat(044d): add bounded workflow dispatch`

## PHASE-03 — Catalog, capability policy and composed surface

**Goal:** Register the final mutation tools and verify the complete Plan-044 catalog/policy contract.

**Dependencies:** PHASE-02

### TASK-004 — Register Actions mutations

**Outcome:** MCP annotations, schemas, hook policy, first-party approval policy and UX summaries agree for all three mutation tools.

**Files:**
- Modify: `packages/rust-tools/interfaces/src/mcp/catalog/forge.rs`
- Modify: `packages/rust-tools/application/src/hooks/policy.rs`
- Modify: `shared/utils/capability-policy.ts`
- Modify if needed: `app/utils/tool-presentation.ts`
- Create: `scripts/verify-044d-actions-mutations.sh`

**Steps:**
- [ ] Add strict schemas with no arbitrary repo/API/CLI fields.
- [ ] Mark all three non-read-only, destructive/effectful by external-mutation convention, non-idempotent where appropriate, and open-world.
- [ ] Map all three to `network_read + network_write + external_mutation + privileged_bridge` in both policy owners.
- [ ] Ensure approval summaries expose only workflow/run/job/ref/input-key counts and never input values by default.
- [ ] Add malformed input handling for IDs/ref/input map/mode combinations.

**Validation:**
- `bash scripts/verify-044d-actions-mutations.sh` → PASS.
- existing capability-policy/UX acceptance affected by the new tools → PASS.

**Commit boundary:** `feat(044d): expose controlled workflow mutations`

### TASK-005 — Freeze the composed v9 MCP contract

**Outcome:** The full tool catalog is deterministic, versioned and regression-checked.

**Files:**
- Update the existing canonical catalog-contract family under `.agents/contracts/` using the current repository naming convention (v8 is `.agents/contracts/039h-tool-catalog-v8.*`; create the next v9 files rather than overwriting v8).
- Modify: `scripts/phase-039h-contract.sh` or its current authoritative successor.
- Create/modify: `scripts/verify-044-composed-contract.sh`

**Steps:**
- [ ] Generate the exact reviewed catalog from the candidate relay.
- [ ] Verify all current 77 v8 tools remain present exactly once.
- [ ] Verify all 23 Plan-044 tools are present exactly once.
- [ ] Expected count is 100 if the planned surface remains unchanged; if reviewed implementation changes tool count, update Plan 044 before contract freeze rather than forcing the number.
- [ ] Verify schemas, annotations and security schemes for every new tool.
- [ ] Hash the canonical v9 snapshot and make the contract verifier reject drift.
- [ ] Confirm no model-facing `github_api`, arbitrary `gh`, generic `api`, endpoint, method/header/body passthrough tool exists.

**Validation:**
- canonical contract verifier → PASS with exact hash.
- `tools/list` snapshot deterministic across repeated candidate starts.

**Commit boundary:** `test(044): freeze github operations contract v9`

## PHASE-04 — Full source verification and adversarial review

**Goal:** Prove the composed Plan-044 implementation is secure and regression-free before merge/deployment.

**Dependencies:** PHASE-03

### TASK-006 — Run complete deterministic verification matrix

**Outcome:** Every child acceptance plus repository gates pass on one exact candidate commit.

**Files:** no implementation file expected unless verification reveals a defect.

**Steps:**
- [ ] Run all Plan 044A issue acceptance.
- [ ] Run Plan 044B Actions/read/log-redaction acceptance.
- [ ] Run Plan 044C security-alert/secret-canary acceptance.
- [ ] Run Plan 044D mutation acceptance.
- [ ] Run existing Plan-040 forge regression acceptance.
- [ ] Run existing Plan-043 contract/security regressions affected by catalog/policy changes.
- [ ] Run existing Plan-035 redaction acceptance affected by shared-redactor extraction.
- [ ] Run `cargo test --workspace`.
- [ ] Run `pnpm verify:commit`.
- [ ] Run `pnpm build` because first-party tool presentation/policy integration changed.
- [ ] Run `cargo audit` / `pnpm audit` if dependencies changed; dependency changes are not expected and should be avoided unless necessary.

**Validation:** every executed gate passes; no skipped failure is represented as success.

**Commit boundary:** remediation commits only if review finds defects.

### TASK-007 — Perform fresh security/architecture falsification pass

**Outcome:** Zero unresolved P0/P1 findings before delivery.

**Review targets:**
- GitHub credential isolation and environment forwarding;
- arbitrary API/CLI injection attempts;
- repository identity substitution;
- output/error bounds;
- process timeout/cleanup;
- non-idempotent retry behavior;
- issue/Actions mutation effect classification;
- action-log credential canaries;
- secret-scanning literal-secret/PII canaries;
- URL/path/SHA/ref validation;
- maintainability budgets and Layered Architecture;
- duplicated policy owners remaining lockstep;
- stale tool-count/docs/contract claims.

**Exit rule:** any P0/P1 is fixed and reverified before merge; P2s are either fixed or documented with explicit non-blocking rationale.

## PHASE-05 — Delivery to main

**Goal:** Integrate the exact reviewed source candidate according to repository workflow.

**Dependencies:** PHASE-04

### TASK-008 — Push, PR, review and merge 044D

**Outcome:** `main` contains the complete Plan-044 implementation through reviewed PR history.

**Steps:**
- [ ] Revalidate repository identity, branch, upstream and unrelated local changes.
- [ ] Stage only Plan-044D-owned changes.
- [ ] Commit through the normal hook; never use `--no-verify`.
- [ ] Push the short-lived 044D branch.
- [ ] Create PR targeting `main` with exact verification evidence.
- [ ] Re-review pushed diff and mergeability; no unrelated files.
- [ ] Squash-merge only when user/repository policy authorizes and expected head still matches.
- [ ] Fetch/switch/fast-forward local `main` safely while preserving unrelated user changes.
- [ ] Record exact final merged main SHA.

**Validation:** local/remote `main` parity and exact reviewed source present.

## PHASE-06 — Release build, install and relay restart

**Goal:** Deploy the exact integrated Plan-044 binary, not a feature-branch candidate.

**Dependencies:** PHASE-05

### TASK-009 — Build and identify release artifact

**Outcome:** Exact merged source produces one release binary with recorded identity.

**Steps:**
- [ ] Determine authoritative release/install procedure from current docs/service configuration.
- [ ] Build release binary from exact merged main commit.
- [ ] Record binary version and SHA256.
- [ ] Install to the operator-managed path using existing non-sudo procedure when available.
- [ ] Verify installed binary SHA256 exactly matches reviewed build artifact.

**Validation:** artifact/install hash parity.

### TASK-010 — Restart operator relay

**Outcome:** `ai-tools-relay.service` runs the new binary and passes health/auth-edge checks.

**Steps:**
- [ ] Verify current user service unit and executable path before restart.
- [ ] Restart only the user-level `ai-tools-relay.service`; do not modify unrelated services.
- [ ] If restart requires privilege not available to the agent, stop at that hard boundary and provide the exact manual command; do not seek privilege bypass.
- [ ] Verify active/running state, main PID/executable identity and no crash/restart loop.
- [ ] Verify `GET /health` returns healthy response.
- [ ] Verify unauthenticated MCP request still returns the expected OAuth challenge rather than accidental open access.

**Validation:** service/runtime identity matches installed Plan-044 artifact.

## PHASE-07 — Live MCP and GitHub acceptance

**Goal:** Prove the deployed service through real MCP, then prove external mutations only on safe authorized fixtures.

**Dependencies:** PHASE-06

### TASK-011 — Live catalog and read-only smoke

**Outcome:** Deployed MCP exposes v9 and representative reads work.

**Steps:**
- [ ] Authenticated `tools/list` matches exact v9 catalog/hash.
- [ ] `issue_list` works against the validated repository.
- [ ] workflow reads return valid empty state for `ai-code` if it still has no workflows; do not treat emptiness as failure.
- [ ] security reads return normalized alerts when accessible or bounded permission/unavailable classification when not.
- [ ] timing metadata remains present on representative new tools.

### TASK-012 — Live issue mutation acceptance

**Outcome:** Complete issue create/update/comment/close/reopen behavior is proven without polluting production issue tracking unnecessarily.

**Guard:** use an operator-approved disposable GitHub repository/fixture. If none exists, obtain explicit authorization before creating a temporary acceptance issue in `ai-code`; do not silently create tracker noise.

**Steps:**
- [ ] create one clearly named acceptance issue;
- [ ] get/list it;
- [ ] update bounded title/body/labels;
- [ ] add one acceptance comment;
- [ ] close with `completed`, verify state;
- [ ] reopen, verify state;
- [ ] close finally with `completed` so fixture is left non-open;
- [ ] record issue number/URL as evidence without copying sensitive content.

### TASK-013 — Live Actions acceptance

**Outcome:** Reads and controlled mutations are proven on a safe dispatchable workflow.

**Guard:** use an explicitly authorized repository/workflow with Actions history and safe rerun/cancel/dispatch semantics. Do not add Actions to `ai-code` merely for this proof.

**Steps:**
- [ ] workflow list/get;
- [ ] run list/get/jobs;
- [ ] log preview on known harmless failed output with canary checks;
- [ ] rerun an approved disposable run and observe attempt/state change;
- [ ] cancel an approved disposable running/queued run when a safe fixture is available;
- [ ] dispatch an approved workflow with non-sensitive inputs and observe returned/new run identity.

**Truthfulness rule:** if one live mutation cannot be safely arranged, mark that exact live case `UNPROVEN` rather than manufacturing a production mutation. Deterministic acceptance remains mandatory; master closure requires operator decision on whether an unavailable live fixture blocks closure.

### TASK-014 — Live security confidentiality acceptance

**Outcome:** Public tool output cannot reveal literal detected secrets.

**Steps:**
- [ ] use live alerts only if the operator token/repository legitimately has access; do not request or expose credentials to gain access.
- [ ] independently inspect serialized result shape and confirm no `secret` field/value.
- [ ] retain deterministic hostile provider canary as the mandatory proof even when live secret-scanning is unavailable/empty.
- [ ] confirm ordinary `terminal_exec` still cannot read GitHub credential stores/environment.

## PHASE-08 — external MCP client connector rediscovery and documentation closure

**Goal:** Close the client side and durable repository state, not merely the server deployment.

**Dependencies:** PHASE-07

### TASK-015 — Rediscover live connector tools

**Outcome:** external MCP client sees and can invoke the new Plan-044 surface.

**Steps:**
- [ ] refresh/reconnect the configured MCP connector through supported client flow.
- [ ] verify discovery metadata indicates the full new function surface; account for listing pagination/caps rather than mistaking a page size for total count.
- [ ] specifically discover and invoke safe representative new tools from each family (`issue_get/list`, workflow read, security read where permitted).
- [ ] verify one new mutation schema is visible without invoking unsafe production action merely for discovery.

### TASK-016 — Reconcile plans/docs/memory and close linked issue

**Outcome:** Durable project state reflects what was actually merged/deployed/proven.

**Files:**
- Modify: Plan 044 master and 044A–044D statuses/checklists.
- Modify: `.agents/memories/README.md` with durable final invariants only.
- Modify: `packages/rust-tools/README.md`, `docs/external-mcp.md`, and other directly affected operator docs.
- Modify current contract references/hash documentation.

**Steps:**
- [ ] Record PR numbers, merged SHAs, final binary hash, v9 hash, relay restart evidence and live proof.
- [ ] Mark each child CLOSED only when its own deterministic criteria and applicable final live checkpoint are satisfied.
- [ ] Mark master Plan 044 CLOSED only when the parent acceptance definition is satisfied.
- [ ] If Plan 044 has a linked GitHub issue, use the now-live issue tools to verify and close it only after plan closure; include a concise completion comment if appropriate.
- [ ] Preserve unrelated local user changes.

## Final Plan-044 acceptance criteria

- [ ] 23 planned new tools exist exactly once, or parent plan is explicitly amended before freeze if reviewed scope changes.
- [ ] v9 catalog is deterministic and hashed.
- [ ] all v8 tools remain present with no unintended contract regression.
- [ ] no generic GitHub API/CLI passthrough exists.
- [ ] issue lifecycle is source-verified and live-proven on safe fixture.
- [ ] Actions reads are source-verified and live-proven where fixture exists.
- [ ] rerun/cancel/dispatch are deterministically verified and live-proven on safe authorized fixture or explicitly escalated as an unavailable-fixture closure decision.
- [ ] Dependabot/code/secret reads are typed, bounded and permission-aware.
- [ ] secret-scanning literal secret/PII canaries never appear in model-visible results/errors.
- [ ] action-log credential canaries never appear in preview.
- [ ] ordinary terminal remains credential-isolated.
- [ ] full deterministic regression matrix passes.
- [ ] `cargo test --workspace` passes.
- [ ] `pnpm verify:commit` passes.
- [ ] `pnpm build` passes.
- [ ] fresh adversarial review has zero unresolved P0/P1.
- [ ] implementation is merged to `main` through PRs.
- [ ] exact merged binary is release-built/installed with hash parity.
- [ ] relay restarted and healthy.
- [ ] authenticated live MCP contract matches v9.
- [ ] external MCP client connector rediscovery exposes Plan-044 tools.
- [ ] docs/memory/plan files are truthful.
- [ ] linked Plan-044 GitHub issue, if any, is closed only after actual completion.

## Rollback

- Source defect before merge: fix on current short-lived branch and re-run affected/full gates.
- Contract/security defect after child merge but before deployment: create a focused remediation branch from current `main`; do not deploy known-defective main.
- Deployment defect: reinstall the last reviewed known-good binary using established operator procedure and restart the user service; do not rewrite Git history.
- Live confidentiality failure: treat as P0, stop further live calls that could expose sensitive data, roll back deployed binary, remediate source, and repeat full canary acceptance before redeploy.

## Closure statement target

Only after all applicable criteria pass may the team report:

**Plan 044 — CLOSED / VERIFIED / MERGED / DEPLOYED / LIVE CONNECTOR VERIFIED.**
