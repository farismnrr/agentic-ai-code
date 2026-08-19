# Plan 044B — GitHub Actions Observability

**Status:** PLANNED
**Parent:** [Plan 044](044-github-repository-operations-security-roadmap.md)
**Depends on:** Plan 044A source merged / live verification pending

## Goal

Add bounded GitHub Actions read/diagnostic tools so an agent can identify workflows, inspect run/job state, and retrieve a small sanitized failed-log preview without adopting GitHub Actions as this repository's CI system or exposing arbitrary workflow logs.

## Success criteria

The source implementation exposes exactly these six new tools:

- `workflow_list`
- `workflow_get`
- `workflow_run_list`
- `workflow_run_get`
- `workflow_run_jobs`
- `workflow_job_log_preview`

All six are read-only model-facing operations. They use validated repository identity, Actions-read credentials through the privileged forge bridge, bounded typed output, and deterministic redaction for log previews.

## Scope

### In scope

- repository workflow metadata;
- bounded recent run listing with safe filters;
- one workflow run detail;
- bounded jobs/steps for one run;
- bounded log preview for one job, defaulting to failed output only;
- safe handling of repositories with no workflows/runs;
- permission/unavailable classification;
- deterministic and live-read acceptance.

### Out of scope

- adding `.github/workflows/*` to `ai-code`;
- workflow dispatch/rerun/cancel (owned by 044D);
- workflow enable/disable/delete;
- artifact/cache read/delete;
- environment/deployment approval;
- Actions variables/secrets;
- runner administration;
- organization-level Actions administration;
- full run-log/archive download;
- arbitrary job log streaming/follow mode.

## Constraints and decisions

1. Use high-level `gh workflow list` and `gh run list/view` where they provide structured JSON.
2. `gh workflow view` does not provide structured JSON; `workflow_get` may use one fixed GitHub REST endpoint internally, derived from validated repository identity and a positive numeric workflow ID.
3. No model-facing workflow path/name is used as an API path selector when an immutable numeric ID is available.
4. Run and job identifiers are positive integers.
5. `workflow_run_list` may expose only reviewed filters such as workflow ID, branch, commit SHA, and normalized status; no arbitrary GitHub search expression.
6. `workflow_job_log_preview` is diagnostic, not a raw-log tool. It must hard-cap total provider bytes read, retained output bytes, returned lines, line length, execution time and redaction work.
7. Log preview defaults to failed-step output when GitHub CLI supports it. A caller may request a bounded whole-job preview only through an explicit boolean, never a full run archive.
8. Action logs are untrusted text and can contain credentials or personal data. Returned lines must pass a deterministic credential-shaped redaction primitive before model exposure.
9. Do not import infrastructure observability from the application crate. If reusable redaction is needed, extract only the pure credential-shaped redaction primitive to a lower layer and keep observability-specific path/attribute policy in infrastructure.

## PHASE-01 — Structured workflow and run reads

**Goal:** Expose workflow/run metadata using existing credential-isolated forge execution.

**Dependencies:** none

### TASK-001 — Add workflow list/get

**Outcome:** Agents can discover workflow IDs and inspect one workflow safely.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/actions.rs`
- Modify: `packages/rust-tools/application/src/git/forge/common.rs` if a fixed API helper needs shared typed support
- Modify: `packages/rust-tools/application/src/git/forge_process.rs` only for a fixed, non-model-facing API invocation primitive
- Modify: `packages/rust-tools/application/src/git.rs`

**Steps:**
- [ ] Implement `workflow_list` using `gh workflow list --all --json id,name,path,state` with a repository-owned result cap.
- [ ] Normalize workflow path as repository-relative metadata only; reject control characters/oversized values/provider repository mismatch.
- [ ] Implement `workflow_get(workflow_id)` through a fixed endpoint or equally structured provider path; the endpoint must be generated internally from validated owner/repo and numeric ID.
- [ ] If introducing internal `gh api`, expose only a typed/fixed adapter; no tool argument may select method, host, endpoint, header, jq expression or arbitrary form field.
- [ ] Normalize unsupported/disabled/no-workflow states without fabricating data.

**Validation:**
- deterministic fixture verifies exact argv/endpoint construction, repository binding, malformed ID rejection and output bounds.
- no model-facing schema contains `owner`, `repository`, arbitrary `url`, `endpoint`, `method`, `headers`, `api`, `command`, or `args`.

**Commit boundary:** `feat(044b): add github workflow reads`

### TASK-002 — Add workflow-run list/get

**Outcome:** Agents can inspect recent Actions runs and one run in detail.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/actions.rs`

**Steps:**
- [ ] Implement `workflow_run_list` through `gh run list --json` with a hard result cap.
- [ ] Allow only typed optional filters: workflow ID, validated branch, exact 40-hex commit SHA, and a normalized status enum supported by current GitHub CLI.
- [ ] Normalize stable fields: run ID/number/attempt, workflow ID/name, display title, event, branch/SHA, status/conclusion, timestamps and validated GitHub URL.
- [ ] Implement `workflow_run_get(run_id)` using `gh run view <id> --json` and omit jobs from this result if jobs have their own tool, preventing duplicate oversized payloads.
- [ ] Reject malformed IDs/SHA/status/provider JSON and repository-mismatched URLs.

**Validation:**
- fixture covers queued/in-progress/completed success/failure/cancelled states and unknown provider enum fail-closed/normalized handling.

**Commit boundary:** `feat(044b): add github workflow run reads`

**Phase exit criteria:**
- [ ] workflow and run list/get work without log access.
- [ ] absence of Actions history returns a valid empty result, not an internal error.

## PHASE-02 — Job/step diagnosis

**Goal:** Provide enough job-level context to understand a failed run before requesting any log text.

**Dependencies:** PHASE-01

### TASK-003 — Add bounded run-job inspection

**Outcome:** One run exposes bounded jobs and steps with status/conclusion/timing metadata.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/actions.rs`

**Steps:**
- [ ] Implement `workflow_run_jobs(run_id)` from structured `gh run view --json jobs` or a fixed jobs endpoint if provider behavior is more reliable.
- [ ] Cap jobs and steps independently.
- [ ] Normalize job ID/name/status/conclusion/start/end and step number/name/status/conclusion/start/end only.
- [ ] Do not include runner labels, environment variables, matrices, annotations or raw log text unless separately reviewed and explicitly needed.
- [ ] Validate all returned URLs/identities where present.

**Validation:**
- fixture includes multiple jobs, skipped/cancelled jobs, failed steps, over-cap truncation, malformed provider objects and long names.

**Commit boundary:** `feat(044b): add github workflow job inspection`

## PHASE-03 — Secret-safe failed-log preview

**Goal:** Add a useful but bounded diagnostic preview without creating a raw GitHub log exfiltration surface.

**Dependencies:** PHASE-02

### TASK-004 — Establish one reusable credential-shaped text redaction primitive

**Outcome:** Model-visible action log text can reuse reviewed credential redaction without violating Layered Architecture or duplicating security regexes.

**Files:**
- Create or modify a suitable pure lower-layer module under: `packages/rust-tools/core/src/`
- Modify: `packages/rust-tools/infrastructure/src/observability.rs`
- Modify: `packages/rust-tools/core/src/lib.rs` if module export is needed
- Add deterministic regression acceptance under `scripts/` or existing observability example surfaces; do not add a unit-test suite.

**Steps:**
- [ ] Identify the credential-shaped substring redaction logic currently embedded in infrastructure observability.
- [ ] Extract only the pure reusable credential redaction portion to `relay_core`; leave observability field allowlisting, path-specific redaction and telemetry policy in infrastructure.
- [ ] Make infrastructure call the shared primitive so Plan-035 behavior does not fork.
- [ ] Preserve existing redaction categories and canary behavior exactly or make stricter changes only with explicit evidence.
- [ ] Add regression canaries for JSON-quoted credentials, bearer/API-key shapes, URL query credentials and representative ordinary text that must remain readable.

**Validation:**
- existing Plan-035 redaction/telemetry acceptance affected by the refactor → PASS.
- new core redaction canary acceptance → PASS.
- `pnpm verify:commit` → PASS.

**Commit boundary:** `refactor(security): share credential text redaction`

### TASK-005 — Implement bounded job-log preview

**Outcome:** `workflow_job_log_preview` returns only a small sanitized diagnostic excerpt.

**Files:**
- Modify: `packages/rust-tools/application/src/git/forge/actions.rs`
- Modify: `packages/rust-tools/application/src/git/forge_process.rs`

**Steps:**
- [ ] Use a fixed direct-argv `gh run view --job <id> --log-failed` path by default; optionally allow reviewed `failed_only:false` for one job, never an entire run archive.
- [ ] Add a specialized bounded text capture path that drains/terminates safely and cannot deadlock on large output.
- [ ] Enforce provider-read hard ceiling, retained byte ceiling, returned line ceiling, per-line ceiling and operation timeout.
- [ ] Prefer a bounded tail/diagnostic window when implementable safely; if provider output exceeds the hard ceiling, return explicit `truncated`/classification rather than enlarging limits unboundedly.
- [ ] Normalize control characters and pass every returned line through the shared credential redactor.
- [ ] Never return raw provider stderr, signed log URLs, redirect URLs, auth headers, archive data or continuation tokens that grant direct log access.
- [ ] Return metadata such as job ID, failed-only flag, returned line count, `truncated`, and sanitized lines.

**Validation:**
- fixture output contains credential canaries, very long lines, ANSI/control chars, >limit output and ordinary compiler/test failures.
- assert all canaries are absent while useful failure text remains.
- verify large log producer exits/gets killed cleanly with no zombie/process-group leak.

**Commit boundary:** `feat(044b): add bounded workflow log preview`

**Phase exit criteria:**
- [ ] No unbounded raw log path exists.
- [ ] Credential canaries are absent from model-visible preview.
- [ ] Existing telemetry redaction remains at least as strict as before.

## PHASE-04 — Catalog, capability policy and deterministic acceptance

**Goal:** Expose the six tools consistently across MCP/policy/presentation owners.

**Dependencies:** PHASE-03

### TASK-006 — Register Actions-read surface

**Outcome:** All read tools are typed, correctly annotated and safely summarized.

**Files:**
- Modify: `packages/rust-tools/interfaces/src/mcp/catalog/forge.rs`
- Modify: `packages/rust-tools/application/src/hooks/policy.rs`
- Modify: `shared/utils/capability-policy.ts`
- Modify if needed: `app/utils/tool-presentation.ts`
- Create: `scripts/verify-044b-actions-observability.sh`

**Steps:**
- [ ] Add strict schemas for the six read tools.
- [ ] Mark every tool read-only + open-world and non-destructive.
- [ ] Map every tool to `network_read + privileged_bridge` in both policy owners.
- [ ] Add malformed-input checks for IDs, SHA, branch, status, log bounds/flags.
- [ ] Ensure UI summaries never echo log lines into approval/input summaries; result preview remains bounded by existing presentation constraints.
- [ ] Verify catalog contains all prior tools exactly once plus six new 044B tools.

**Validation:**
- `bash scripts/verify-044b-actions-observability.sh` → PASS.
- prior 040/044A forge contract acceptance → PASS.
- `cargo test --workspace` → PASS.
- `pnpm verify:commit` → PASS.

**Commit boundary:** `feat(044b): expose github actions observability`

## PHASE-05 — Documentation and merge handoff

### TASK-007 — Integrate 044B source without changing repo CI policy

**Outcome:** Actions observability source is merged while docs remain explicit that `ai-code` still has no GitHub Actions CI workflow.

**Files:**
- Modify: `packages/rust-tools/README.md`
- Modify: `docs/external-mcp.md`
- Modify: this plan and parent Plan 044
- Modify `.agents/memories/README.md` only for durable redaction/architecture changes introduced by TASK-004.

**Steps:**
- [ ] Document tool semantics and log-preview bounds at a high level.
- [ ] Explicitly preserve no-CI/no-unit-test repository policy.
- [ ] Run mandatory closeout review and local gates.
- [ ] Deliver via short-lived implementation branch + PR to `main`.
- [ ] Mark `MERGED / LIVE VERIFICATION PENDING` after source integration.

**Validation:**
- exact merged source passes Plan-044B deterministic acceptance and repository gate.

**Commit boundary:** normal implementation PR squash merge to `main`.

## Risks and rollback

- **Log secret leakage:** shared deterministic redaction + hard output caps; if proof is weak, omit log preview rather than shipping unsafe raw logs.
- **Redaction refactor regression:** existing Plan-035 acceptance is blocking; revert/refine extraction if telemetry behavior changes unexpectedly.
- **Huge logs:** specialized capture must kill/drain safely; never increase global forge output caps just to accommodate logs.
- **No Actions in ai-code:** use a disposable/authorized external test repository for live 044D proof; do not add CI to manufacture local evidence.
- **Provider drift:** typed parser fails closed and official GitHub docs/CLI fields are rechecked at implementation start.

## Final 044B acceptance criteria

- [ ] Six Actions-read tools exist exactly once.
- [ ] workflows/runs/jobs are bounded structured data.
- [ ] log preview is bounded, failed-focused by default, sanitized and non-streaming.
- [ ] no raw log/archive/signed URL passthrough exists.
- [ ] existing GitHub credential isolation remains intact.
- [ ] existing Plan-035 redaction behavior remains green after shared-redactor extraction.
- [ ] deterministic 044B acceptance passes.
- [ ] `cargo test --workspace` passes.
- [ ] `pnpm verify:commit` passes.
- [ ] source PR merges to `main`.
- [ ] live Actions proof remains pending until 044D deployment.

## Handoff

After 044B is merged source-clean, continue to [Plan 044C](044c-github-security-alert-visibility.md).
