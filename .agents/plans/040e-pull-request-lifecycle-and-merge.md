# Plan 040E — Pull Request Lifecycle, Checks, Reviews, and Merge

**Status:** CLOSED / VERIFIED / MERGED (2026-08-19)
**Parent:** [Plan 040](040-git-github-delivery-roadmap.md)
**Depends on:** Plan 040D CLOSED / VERIFIED / MERGED

## Goal

Complete the remote review lifecycle so the agent can create/update a PR, inspect checks/reviews, remediate failures, merge through an approved strategy, and clean up branches without bypassing repository policy.

## Scope

- create/update PR/change request through 040C contracts;
- inspect required checks and bounded failure summaries;
- inspect review state / requested changes;
- refresh mergeability after new pushes;
- merge with an explicit supported strategy (merge/squash/rebase) only when policy allows;
- verify resulting integration commit/ref;
- remote feature-branch deletion as separate explicit mutation;
- local integration-branch sync and parity verification using 040A/040B primitives.

## Approval and policy rules

- creating/updating PR: external mutation;
- merge: high-impact external mutation and must never silently bypass required approvals/checks;
- branch delete: destructive remote mutation;
- repository-side branch protection remains authoritative;
- never self-approve or dismiss required review feedback to unblock a merge;
- admin/bypass flags must not be available through the normal tool contract;
- merge must re-check base/head and mergeability immediately before mutation to reduce stale-state races.

## Remediation loop

When checks or reviews fail:

1. fetch bounded structured evidence;
2. return to local workspace tools/subagents for remediation;
3. validate locally;
4. commit/push through 040A/040B;
5. refresh PR/check/review state;
6. only merge after current evidence satisfies policy.

Do not build an unbounded autonomous loop; existing task/budget/cancellation contracts remain authoritative.

## Acceptance scenarios

1. PR creation and current-state readback;
2. pending check is represented as pending, not failure/success;
3. failed check returns bounded useful evidence without raw secrets;
4. review requesting changes blocks merge path;
5. stale head changes are detected before merge;
6. allowed merge strategy succeeds and resulting integration ref is independently observed;
7. branch cleanup occurs only after verified merge when requested;
8. protected branch deletion/force/bypass attempts fail closed;
9. local integration branch can be synchronized and parity proven;
10. failed remote mutation does not falsely update local task/UI state.

## Exit criteria

- PR lifecycle from creation through merge/cleanup is represented by bounded native/forge tools;
- existing repository review/check protection is preserved rather than replicated insecurely;
- live GitHub acceptance proves the supported workflow;
- independent review reports zero unresolved P0/P1;
- 040E merged before 040F begins.
