# Plan 044 — GitHub Repository Operations and Security Visibility Roadmap

**Status:** PLANNED
**Created:** 2026-08-19
**Predecessor:** Plan 043 — Broad Git, Worktrees, Timing, and Workspace Authorization (CLOSED / VERIFIED / MERGED / DEPLOYED)
**Plan family:** 044A–044D

## Goal

Complete the GitHub-side engineering lifecycle beyond pull requests so a coding agent can safely track issue work, inspect GitHub Actions execution, inspect repository security alerts, and perform a narrowly controlled set of workflow mutations without exposing GitHub credentials to ordinary terminal execution or introducing an arbitrary GitHub API passthrough.

Plan 044 extends the credential-isolated forge boundary established by Plan 040 and the structured native-tool surface expanded by Plan 043.

## Success criteria

Plan 044 is successful only when the live authenticated MCP connector can perform all of the following through structured tools:

1. issue lifecycle: list/get/create/update/comment/close/reopen;
2. Actions observability: workflow list/get, run list/get, job inspection, bounded failed-log preview;
3. security visibility: Dependabot, code-scanning, and secret-scanning alert list/get, plus secret-scanning locations;
4. controlled Actions mutation: rerun, cancel, and explicit `workflow_dispatch` only;
5. no secret value, GitHub credential, auth header, protected credential path, raw provider error, or unbounded action log reaches model-visible output;
6. repository identity is always derived from a validated GitHub remote rather than arbitrary owner/repository arguments;
7. the final MCP contract is versioned and deterministic, the operator relay is rebuilt/restarted from the reviewed merged commit, and external MCP client connector rediscovery proves the new tools live.

The planned public tool surface adds 23 tools to the current 77-tool v8 catalog, producing an expected 100-tool v9 catalog if no tool is added or removed during reviewed implementation.

## Current state

Verified on 2026-08-19:

- `main` is the only long-lived branch; implementation work must use short-lived branches and PRs targeting `main`.
- Plan 043 is deployed and the authenticated connector exposes the current v8 77-tool contract.
- GitHub support currently covers validated remotes, fetch/push/remote branch deletion, and forge-neutral pull-request/change-request lifecycle.
- Existing GitHub operations run through `packages/rust-tools/application/src/git/forge_process.rs`, which invokes `gh` with cleared environment, fixed safe PATH, bounded output/time, hidden stderr, and only forwarded `GH_TOKEN` / `GITHUB_TOKEN` credentials.
- `packages/rust-tools/application/src/git/forge.rs` is already approximately at the maintainability ceiling and `packages/rust-tools/application/src/git/` has 14 direct Rust files; new GitHub domains must not be appended monolithically.
- `packages/rust-tools/interfaces/src/mcp/catalog.rs` is also close to the 500-line source budget; new Plan-044 declarations need a nested catalog module rather than another large inline block.
- capability/effect ownership is duplicated intentionally across relay hook policy and first-party UI capability policy and both must stay lockstep.
- the repository intentionally has no GitHub Actions CI workflow and no unit-test suite. Plan 044 adds GitHub Actions *tooling support* but does not adopt Actions as this repository's quality gate.

## External GitHub constraints verified for planning

The implementation agent must re-check these facts against current official GitHub documentation before implementation if the API/CLI version has changed:

- GitHub REST API documentation currently uses version `2026-03-10`.
- issue operations require repository Issues read/write permission as appropriate; high-level `gh issue` commands exist for list/view/create/edit/comment/close/reopen.
- workflow/run/job reads require Actions read permission; rerun/cancel/dispatch require Actions write permission.
- Dependabot alert reads require Dependabot-alert read permission; code-scanning reads require Code-scanning-alert read permission; secret-scanning reads require Secret-scanning-alert read permission.
- secret-scanning alert responses can include a literal `secret` field. The REST API supports `hide_secret=true`; Plan 044 additionally requires server-side removal of the field even when upstream claims it is hidden.
- GitHub Actions run/job logs can contain arbitrary workflow output; Plan 044 never exposes unbounded raw logs.

## Architecture decisions

### AD-001 — Preserve the narrow privileged forge bridge

Reuse the validated repository/remote identity and credential-isolated `gh` process boundary. Ordinary `terminal_exec` must remain unable to access `~/.config/gh`, `GH_TOKEN`, or `GITHUB_TOKEN`.

### AD-002 — No generic GitHub API tool

Do not expose any model-facing tool resembling:

- `github_api(method, path, body)`;
- arbitrary `gh` command execution;
- arbitrary `gh api` path/method/header/body passthrough.

Where a GitHub feature has no safe high-level `gh` command, an internal adapter may use a fixed, enumerated endpoint template derived only from validated repository identity and bounded typed arguments. The endpoint/method/header set must not be supplied by model input.

### AD-003 — Split forge implementation by responsibility before growth

Keep `git/forge.rs` as a thin facade and place GitHub-domain logic under `packages/rust-tools/application/src/git/forge/`:

- `common.rs` — shared bounded forge DTO helpers and validated identity utilities;
- `change_requests.rs` — move existing PR/change-request implementation without behavior regression;
- `issues.rs` — Plan 044A;
- `actions.rs` — Plan 044B / 044D;
- `security.rs` — Plan 044C.

Do not add new direct files to `application/src/git/` if that would violate the direct-file budget.

### AD-004 — Keep catalog growth modular

Add a nested MCP catalog module for Plan-044 forge tools rather than appending 23 declarations directly to the existing near-budget `catalog.rs`. Preserve `find_tool()` and argument validation behavior.

### AD-005 — Read-first security model

Security alert mutation is explicitly out of Plan 044 scope. Dependabot dismissal, code-scanning dismissal, secret-scanning resolution/reopen, push-protection bypass, security-and-analysis setting mutation, and repository security configuration changes require a future separately reviewed plan.

### AD-006 — Existing UI capability taxonomy is sufficient

Avoid adding decorative category architecture merely for Plan 044:

- issue/workflow tools may remain in the existing Git/forge presentation category;
- security-alert reads should present as diagnostics/security-sensitive reads using existing safe summaries;
- capability effects remain the authority for approval risk.

If current code cannot express a truthful safe presentation without a small new category, the implementation agent may add one, but it must be justified by concrete UX ambiguity rather than aesthetics.

## Non-negotiable security and correctness rules

1. Validated GitHub remote identity is mandatory for every tool call.
2. Model input never chooses arbitrary owner/repository, API host, API route, HTTP method, header, or `gh` subcommand.
3. GitHub credentials never enter ordinary terminal/task sandbox environment or model-visible output.
4. All external output is size-bounded and parsed into typed normalized DTOs before returning.
5. Raw `gh` stderr and raw HTTP/provider errors remain hidden behind bounded static classifications.
6. Issue/workflow/security URLs returned to the client must match the validated owner/repository.
7. External mutations are independently re-read/verified before success is reported when GitHub exposes observable state.
8. Non-idempotent mutations must never be automatically replayed after an ambiguous transport result.
9. Security alert list/get tools must never return literal detected secrets.
10. Secret-scanning upstream requests must request hidden secrets and the normalized result must omit the upstream `secret` field unconditionally.
11. Secret-scanning metadata must be allowlisted; owner email/name/id metadata is not model-visible by default.
12. Action log tools return only bounded, sanitized diagnostic preview; no full-log passthrough and no archive/blob download surface.
13. The repository's no-CI policy remains unchanged; Plan 044 must not add `.github/workflows/*` merely to test Actions tooling.
14. No security alert dismiss/resolve mutation is implemented in this plan family.
15. No workflow enable/disable/delete, repository Actions policy mutation, artifact deletion, deployment approval, environment mutation, or Actions secret/variable management is implemented.

## Child plans

| Plan | Capability | Depends on | Status | Exit criterion |
| --- | --- | --- | --- | --- |
| 044A | GitHub issue lifecycle | Plan 043 / Plan 040 forge boundary | PLANNED | Seven structured issue tools work against validated GitHub repos with bounded DTOs, verified mutations, correct policy effects, and deterministic acceptance |
| 044B | GitHub Actions observability | 044A source architecture | PLANNED | Six read-oriented workflow/run/job/log-preview tools provide useful bounded diagnosis without adopting CI or exposing raw unbounded logs |
| 044C | GitHub security alert visibility | 044B | PLANNED | Dependabot/code/secret scanning reads are normalized, permission-aware, secret-safe, and proven not to disclose literal detected secrets |
| 044D | Controlled Actions mutations + integrated closure | 044C | PLANNED | Rerun/cancel/dispatch are narrowly controlled; composed v9 contract is reviewed, merged, deployed, restarted, rediscovered, and verified live |

## Master todo

- [ ] 044A — issue lifecycle
- [ ] 044B — Actions observability
- [ ] 044C — security alert visibility
- [ ] 044D — controlled Actions mutations and final integration
- [ ] fresh composed security/architecture review with zero unresolved P0/P1 findings
- [ ] final contract snapshot/hash
- [ ] merge to `main`
- [ ] release build/install of reviewed binary
- [ ] restart `ai-tools-relay.service`
- [ ] health/OAuth challenge proof
- [ ] authenticated live MCP proof
- [ ] external MCP client connector rediscovery proof
- [ ] documentation + canonical-memory reconciliation

## Planned tool surface

### 044A — Issues (7)

- `issue_list`
- `issue_get`
- `issue_create`
- `issue_update`
- `issue_comment`
- `issue_close`
- `issue_reopen`

### 044B — Actions reads (6)

- `workflow_list`
- `workflow_get`
- `workflow_run_list`
- `workflow_run_get`
- `workflow_run_jobs`
- `workflow_job_log_preview`

### 044C — Security reads (7)

- `dependabot_alert_list`
- `dependabot_alert_get`
- `code_scanning_alert_list`
- `code_scanning_alert_get`
- `secret_scanning_alert_list`
- `secret_scanning_alert_get`
- `secret_scanning_alert_locations`

### 044D — Actions mutations (3)

- `workflow_run_rerun`
- `workflow_run_cancel`
- `workflow_dispatch`

## Target engineering lifecycle

```text
issue_get / issue_list
  ↓
implementation branch
  ↓
local verification
  ↓
push / change request
  ↓
change_request_checks
  ↓
workflow_run_get / workflow_run_jobs
  ↓
workflow_job_log_preview (only when diagnosis is needed)
  ↓
remediate
  ↓
merge
  ↓
issue_close
```

Security investigation remains read-first:

```text
*_alert_list
  ↓
*_alert_get
  ↓
contained source/dependency investigation
  ↓
remediation through normal branch/PR workflow
  ↓
re-read alert / repository state
```

Plan 044 does not automatically dismiss the alert after remediation.

## Execution order

044A → 044B → 044C → 044D.

Child implementation must remain sequential against one source authority. Read-only review subagents may run in parallel, but two child implementations must not mutate the same worktree concurrently.

Each child must pass its own deterministic acceptance and `pnpm verify:commit` before the next child begins. Deployment may be batched at 044D so the operator relay is restarted once with the composed candidate, but child plans remain truthfully marked live-verification pending until that final deployment checkpoint.

## Cross-plan risks

- **Credential expansion:** security APIs may require token permissions not currently granted. Missing permission must produce a bounded explicit unavailable/forbidden result; do not weaken credential isolation or scrape credentials.
- **Secret-scanning disclosure:** upstream can return literal secrets. Mitigation: `hide_secret=true`, typed normalization, unconditional omission of `secret`, deterministic canary acceptance.
- **Action logs:** logs can be huge and can contain arbitrary data. Mitigation: failed-job-focused preview, hard byte/line/time caps, deterministic redaction, no full-log endpoint.
- **Maintainer workflow mutation:** rerun/cancel/dispatch changes external state. Mitigation: explicit typed tool, user-approval effect, validated repo/ref/workflow identity, and post-action observation.
- **Catalog growth:** 23 declarations can break maintainability. Mitigation: nested forge catalog module from the first child.
- **Provider drift:** GitHub API/CLI fields change. Mitigation: typed parser rejects malformed provider output and planning facts are revalidated against current official documentation at implementation time.

## Final acceptance criteria

Plan 044 closes only when all of the following are true:

- [ ] 044A–044D are individually source-verified and integrated according to repository workflow.
- [ ] expected v9 tool catalog contains the complete planned Plan-044 surface exactly once, with no regression of the current 77 tools.
- [ ] no generic GitHub API/CLI passthrough exists.
- [ ] ordinary terminal execution still cannot access GitHub credential material.
- [ ] issue mutation lifecycle succeeds live against a disposable/authorized issue fixture and verifies final state.
- [ ] Actions reads work against a repository that actually has Actions history; absence of workflows in `ai-code` itself is not treated as implementation failure.
- [ ] Action log preview proves byte/line bounds and secret redaction.
- [ ] Dependabot and code-scanning tools return normalized safe data when the repository/token has access, or a bounded permission/unavailable classification when it does not.
- [ ] secret-scanning tools prove the upstream literal secret is never model-visible, including a deterministic canary fixture/mock boundary when live alerts are unavailable.
- [ ] workflow rerun/cancel/dispatch are live-proven only on an explicitly safe test workflow/repository; no production workflow mutation is inferred from source tests.
- [ ] `cargo test --workspace` passes.
- [ ] relevant deterministic Plan-044 acceptance scripts pass.
- [ ] `pnpm verify:commit` passes.
- [ ] dependency/security-sensitive changes, if any, pass the repository-required audits.
- [ ] implementation PR is reviewed and merged to `main` without bypass.
- [ ] exact merged commit is release-built, installed, and binary identity/hash recorded.
- [ ] `ai-tools-relay.service` is restarted from that reviewed binary and healthy.
- [ ] live authenticated MCP catalog and representative Plan-044 calls succeed.
- [ ] external MCP client connector rediscovery exposes the new tool surface.
- [ ] docs, Plan 044 status, and `.agents/memories/README.md` are reconciled with actual live state.

## Explicit non-goals

- adding GitHub Actions CI to this repository;
- generic GitHub REST or GraphQL passthrough;
- arbitrary `gh api` or arbitrary `gh` execution;
- issue deletion, transfer, project-board mutation, milestone mutation, pin/lock management, dependency/sub-issue graph mutation;
- workflow enable/disable/delete or repository/organization Actions permission mutation;
- artifact/cache deletion;
- deployment/environment approval;
- Actions secret/variable/environment-secret management;
- Dependabot/code-scanning/secret-scanning dismiss/resolve/reopen mutation;
- GitHub security setting changes or push-protection bypass;
- organization-wide issue/security/action administration;
- GitLab/Gitea implementation in this plan family.

## Handoff

Execution begins with [Plan 044A](044a-github-issue-lifecycle.md). The implementation owner must create a short-lived branch from the then-current `main`, preserve unrelated local changes, and must not begin 044B until 044A source acceptance is green.
