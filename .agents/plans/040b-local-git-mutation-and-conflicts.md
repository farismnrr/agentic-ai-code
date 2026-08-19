# Plan 040B — Local Git Mutation and Structured Conflict Workflow

**Status:** IMPLEMENTED — LIVE VERIFICATION / FINAL CLOSURE PENDING
**Parent:** [Plan 040](040-git-github-delivery-roadmap.md)
**Depends on:** Plan 040A CLOSED / VERIFIED

## Goal

Add bounded native local-Git mutation primitives so agents do not need to parse arbitrary terminal output for routine branch/commit/merge/rebase workflows, while preserving existing protected-path, helper-disable, execution-root and approval boundaries.

## Scope

Design model-facing operations for the smallest useful mutation set, likely covering:

- branch list/current/create/switch;
- stage/unstage already-present primitives where appropriate;
- commit with explicit message and bounded result;
- merge start/status/continue/abort;
- rebase start/status/continue/abort when a safe bounded contract is justified;
- structured conflict discovery;
- local branch delete with clean/merged-state checks;
- post-operation status/parity evidence.

Do not expose a generic `git <anything>` MCP mutation tool.

## Structured conflicts

A merge/rebase conflict must return structured state such as:

- operation type;
- current branch/base/head identity;
- conflicted paths;
- conflict kind when Git exposes one safely;
- whether index/worktree resolution is required;
- allowed next actions: inspect/edit/stage/continue/abort.

The agent resolves content through ordinary safe file/patch tools, then validates diagnostics/tests, checks conflict markers/index state, and only then continues.

Never auto-resolve by choosing ours/theirs globally.

## Security requirements

- reuse hardened Git process construction; disable external diff/textconv/fsmonitor/pager and unsafe config execution;
- keep object database/alternates/protected-path protections from Plan 039;
- validate repository/worktree identity before and after mutation;
- reject cross-worktree ambiguity;
- reject protected-path lineage leaks in status/conflict output;
- direct argv only; no shell interpolation;
- branch/ref/message limits must be explicit;
- destructive local branch deletion requires deterministic preconditions and approval classification;
- abort/continue must prove an operation is actually active and owned by the target repository.

## Acceptance scenarios

1. create and switch feature branch;
2. stage + commit a normal file change;
3. clean merge with bounded result;
4. intentional two-file merge conflict returns structured paths;
5. resolve via safe edit/patch, validate, stage and continue;
6. abort restores expected pre-operation state in a disposable fixture;
7. malformed refs/options fail before Git mutation;
8. protected file involved in conflict remains hidden/denied;
9. hostile repo Git config cannot execute helpers;
10. local branch cleanup refuses unsafe deletion and succeeds after proven merge when policy permits.

## Verification

Add focused deterministic acceptance and integrate with existing Git/patch, workspace security, capability-policy, zero-bypass and commit gates. Run `cargo test --workspace --locked`, relevant Git black-box scripts and `pnpm verify:commit` before closure.

## Exit criteria

- bounded local mutation tools cover the intended workflow;
- conflict state is structured and truthful;
- no general terminal/Git credential expansion is required;
- independent review reports zero unresolved P0/P1;
- focused and repository-authoritative gates pass;
- docs/contracts updated;
- 040B is CLOSED / VERIFIED / merged before 040C begins.
