# Plan 039B — Git Read Intelligence and Patch Ergonomics

**Status:** CLOSED / VERIFIED — source, independent review, deployment, and live 18-tool MCP acceptance complete
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039A  

## Goal

Give coding agents structured, bounded, side-effect-controlled Git/change intelligence and a safe multi-hunk patch primitive so common code-review and implementation loops do not require fragile shell parsing or many exact-replacement calls.

## Scope

### Native read-only Git tools

- `git_status`
- `git_diff`
- `git_log`
- `git_show`
- `git_blame`

### Workspace mutation ergonomics

- `apply_patch`

`apply_patch` is not a general shell patch wrapper. It must parse a constrained patch format in-process, validate every target through the existing workspace security layer, preview the complete change set, and commit safely only when all targets pass validation.

## Non-goals

- `git_commit`, `git_push`, `git_rebase`, `git_reset`, `git_merge`, or other Git mutation tools;
- replacing `terminal_exec` for normal Git mutation workflows already governed by `github-delivery`;
- arbitrary `git -c ...` or user-supplied Git subcommands;
- running configured external diff/textconv tools;
- binary patch support in v1;
- patching outside `execution_root`;
- silently deleting files through a generic patch unless deletion semantics are explicitly designed, annotated, and approved.

## Current state

- Git operations are currently available through `terminal_exec` only.
- Plan 038 workspace tools already provide secure path containment and atomic edit/write primitives.
- `file_edit` is intentionally exact-match oriented, which is safe but inefficient for multi-hunk/multi-file source changes.
- Terminal subprocesses run inside Bubblewrap, but native Git tools need their own read-only execution contract rather than inheriting arbitrary command behavior.

## Architecture decision

Use the system Git executable through **fixed direct argv** from a dedicated Git application adapter. Do not embed libgit2 unless current Git CLI behavior proves insufficient.

Every Git tool must:

1. resolve the requested `cwd` under `execution_root`;
2. discover the containing Git worktree/root deterministically;
3. reject paths that are not in a Git worktree where required;
4. invoke a fixed, allowlisted Git subcommand and fixed safety flags;
5. disable pager/color/external diff/textconv/config-driven executable features as applicable;
6. parse machine-friendly output rather than human terminal decoration;
7. enforce hard result/byte limits independent of caller limits;
8. return stable structured fields plus concise text where MCP compatibility requires it.

Read-only Git must not accidentally execute repository-controlled helpers. During implementation, explicitly review and neutralize relevant Git features such as:

- external diff drivers;
- textconv filters;
- fsmonitor hooks;
- pager configuration;
- aliases;
- arbitrary `GIT_*` environment inheritance;
- repository hooks where a supposedly read-only command could trigger them;
- unsafe ownership/config discovery behavior.

## Proposed contracts

### `git_status`

Inputs:

```text
cwd?
include_untracked? = true
```

Structured result should include at least:

- repository root relative to execution root;
- current branch or detached HEAD;
- upstream when available;
- ahead/behind when available;
- staged changes;
- unstaged changes;
- untracked paths;
- conflicts;
- truncation metadata.

Prefer porcelain v2 / `-z` style machine output where practical.

### `git_diff`

Inputs should cover the high-value cases without exposing arbitrary command construction:

```text
cwd?
mode? = working | staged | refs
base_ref?
head_ref?
path?
context_lines?
continuation?
max_bytes?
```

Rules:

- `refs` mode requires validated refs;
- no external diff/textconv by default;
- bounded file count/hunk count/bytes;
- continuation token must bind to the original query parameters;
- indicate binary files without dumping binary content.

### `git_log`

Inputs:

```text
cwd?
ref?
path?
max_results?
continuation?
```

Return bounded commit metadata:

- SHA;
- parents when useful;
- author name only if needed for code archaeology (avoid unnecessary PII propagation into telemetry);
- timestamp;
- subject;
- optional bounded body only when explicitly requested.

### `git_show`

Inputs:

```text
cwd?
ref
path?
include_patch? = true
continuation?
```

Use the same safe diff configuration and output bounds as `git_diff`.

### `git_blame`

Inputs:

```text
cwd?
path
start_line?
end_line?
```

Return line-to-commit mapping with bounded ranges. Do not return an entire huge file by default.

## `apply_patch` design

### Required safety model

The tool should support source-level add/update operations across multiple files with **all-or-nothing preflight**.

Preferred flow:

```text
parse constrained patch
  -> reject malformed/oversized/binary patch
  -> collect target paths
  -> resolve every target under workspace policy
  -> reject symlink/protected-path/unsupported-entry targets
  -> load bounded originals
  -> apply every hunk in memory
  -> require deterministic context matches
  -> generate complete preview + hashes
  -> revalidate target identity/staleness
  -> atomically commit each file using existing secure primitives
  -> if multi-file atomicity cannot be guaranteed, use a journal/rollback strategy and never claim transaction semantics that are not actually provided
```

### Patch semantics

- use a documented constrained unified-diff-like grammar or an even simpler internal patch grammar;
- no shell `patch` invocation;
- no path traversal through patch headers;
- no absolute external paths;
- no binary payloads;
- explicit hard caps on files, hunks, lines, and total bytes;
- reject ambiguous/fuzzy hunks by default;
- optional `dry_run=true` must produce the same validation/preview without mutation;
- return changed paths, hunk counts, before/after hashes, and bounded diff summary.

Deletion support must be a deliberate later phase in this child plan, not an accidental consequence of empty output. If implemented, deletion requires explicit input/annotation and protected-path checks.

## Phases

### PHASE-01 — Git execution safety contract

**Goal:** Freeze a safe, reusable read-only Git invocation layer.

- [x] Inspect Git configuration/executable attack surfaces relevant to read-only commands.
- [x] Define fixed environment and config overrides.
- [x] Define repository discovery and cwd/worktree semantics.
- [x] Define bounds and stable errors.
- [x] Create deterministic acceptance script proving a hostile repo config cannot trigger external executable helpers through the new read tools.

**Exit:** one reviewed Git adapter contract exists before any public Git tool is exposed.

### PHASE-02 — `git_status`

- [x] Add MCP schema/annotations.
- [x] Implement structured porcelain parsing.
- [x] Bound untracked/change lists.
- [x] Cover detached HEAD, no upstream, conflicts, untracked, clean repo, nested cwd.
- [x] Prove no mutation occurs.

### PHASE-03 — `git_diff`

- [x] Implement working/staged/ref modes.
- [x] Add path filtering and bounded context.
- [ ] Add continuation support shared later with Plan 039H (deferred to Plan 039H; 039B uses hard bounded single-page output).
- [x] Prove external diff/textconv execution is disabled.
- [x] Handle binary/truncated diffs truthfully.

### PHASE-04 — `git_log`, `git_show`, `git_blame`

- [x] Add bounded schemas and parsing.
- [x] Validate refs/path ranges.
- [x] Reuse common safe Git runner and pagination.
- [x] Add hostile-config/large-history negative cases.

### PHASE-05 — `apply_patch` preflight engine

- [x] Define constrained grammar.
- [x] Implement parser with hard limits.
- [x] Resolve every path through workspace/protected-path policy.
- [x] Apply hunks in memory without mutation.
- [x] Produce deterministic preview/hashes.
- [x] Reject fuzzy/ambiguous/stale application.

### PHASE-06 — safe patch commit

- [x] Reuse secure atomic file mutation primitives.
- [x] Define truthful multi-file failure/rollback behavior.
- [x] Ensure permissions are preserved where expected.
- [x] Add dry-run and stale-target acceptance cases.
- [x] Classify `apply_patch` accurately for approval policy.

### PHASE-07 — MCP/live regression

- [x] Verify all new tools through `tools/list` and authenticated `tools/call`.
- [x] Verify malformed schemas fail before dispatch.
- [x] Verify OAuth/sandbox/path boundaries remain intact.
- [x] Exercise a real clean/change/diff/patch/read-back workflow.
- [x] Confirm worktree state is exactly expected after canary cleanup.

### Phase-07 closure evidence — 2026-08-17

- Final reviewed implementation baseline is `1b7ed8913070f1c4042d0e91fe8bcfc0418ffc4e`; the independent read-only review covered `6943edf3a8fe6a6ea24705873fb7c4ce50d7ccba..1b7ed8913070f1c4042d0e91fe8bcfc0418ffc4e` and returned `VERDICT: NO MATERIAL FINDINGS`.
- Final source validation passed on that exact implementation state: `git diff --check`, `pnpm verify:commit`, `pnpm build:tools`, `cargo test --workspace --all-targets --all-features --locked`, native Plan-039B acceptance, `scripts/verify-git-patch-tools.sh`, `scripts/phase7-external-mcp-contract.sh`, `scripts/check-architecture.sh`, maintainability, `scripts/verify-workspace-v1-integration.sh`, zero-bypass, workspace-path-security, and MCP error-confidentiality regression gates.
- Fresh release artifact `ai-tools 0.0.10` from the reviewed source hashed to `03d3ca5cad6add61eee91b02676f4b70dcc96ac1ac0a3632852d5f8e2295aa10`; manual installation produced the same installed hash and `ai-tools-relay.service` restarted `active`.
- After external MCP client MCP reconnect, discovery exposed all 18 relay tools: the existing 12 Plan-038 tools plus `git_status`, `git_diff`, `git_log`, `git_show`, `git_blame`, and `apply_patch`. Authenticated live calls exercised the complete catalog; the pre-existing Plan-038 positive-outbound `http_fetch` `connect_failed` limitation remained unchanged while its SSRF fail-closed behavior remained intact.
- A disposable nested Git repository under ignored `.tmp/` exercised real status/history/show/blame plus `apply_patch` dry-run and committed patch, followed by `git_diff` and file read-back. Empty `apply_patch.patch` was rejected by schema validation before dispatch; an external `/etc/passwd` read and traversal patch were rejected by execution-root/path validation.
- Terminal background-job start/get/cancel reached a final `cancelled` state; `web_search`, workspace list/search/read/write/edit, terminal execution, and the Git/patch tools all remained callable through the refreshed authenticated connection.
- The canary repository was removed after acceptance and the primary `feat/039b-git-read-patch` worktree returned exactly clean with zero staged, unstaged, or untracked paths.

## Validation

Use repository-authoritative commands. At child-plan closure, at minimum:

```bash
pnpm verify:commit
pnpm build:tools
```

plus deterministic Git safety/black-box scripts added by this plan and live MCP smoke testing when the deployed relay is part of acceptance.

## Commit boundaries

Prefer small logical commits such as:

```text
feat(relay): add safe git read adapter
feat(relay): expose structured git status and diff tools
feat(relay): add bounded git history tools
feat(relay): add safe multi-file patch tool
 test(relay): verify git and patch security contracts
```

Exact commit structure should follow the final diff, not this illustrative list.

## Acceptance criteria

- [x] Git inspection no longer requires shell parsing for standard status/diff/history/blame workflows.
- [x] All Git outputs are bounded and structured.
- [x] Repository-controlled external diff/textconv/fsmonitor-like execution is prevented or proven harmless for these tools.
- [x] `apply_patch` validates all targets before mutation and cannot escape/protected-path bypass.
- [x] Patch behavior is deterministic, bounded, and truthfully atomic/rollback-safe according to implementation.
- [x] Existing 12 Plan-038 tools have no regression.
- [x] Mandatory repository verification passes.
