# Plan 040F — Delivery Orchestration, UX, Observability, and Closure

**Status:** CLOSED / VERIFIED / MERGED (2026-08-19)
**Parent:** [Plan 040](040-git-github-delivery-roadmap.md)
**Depends on:** Plan 040E CLOSED / VERIFIED / MERGED

## Goal

Integrate Plans 040A–040E into one truthful delivery workflow in the first-party agent experience and remote MCP, then close Plan 040 with adversarial and live evidence.

## Integrated workflow

The agent should be able to represent and execute, step-by-step:

- current repository/branch status;
- branch creation/switch;
- local edits and validation;
- commit evidence;
- push evidence;
- change-request/PR identity;
- checks/reviews/mergeability;
- remediation cycles;
- merge approval/result;
- remote/local branch cleanup;
- integration branch parity.

Do not hide intermediate states behind a single opaque “ship it” tool. Composition remains explicit so users and policy can see each high-impact boundary.

## UX requirements

Reuse Plan-039 category-driven presentation and approval surfaces:

- local Git mutation card/state;
- structured conflict card with conflicted paths and next actions;
- push/remote mutation approval summary;
- PR/check/review summary;
- merge strategy + base/head + risk summary;
- cleanup/parity result.

Never expose credential values, raw authenticated request headers, protected paths, unrestricted command payloads, or raw provider errors.

## Observability

Reuse existing semantic telemetry. Add only bounded identifiers/classifications needed to answer:

- local Git operation + outcome;
- remote transport operation + remote identity class;
- forge/provider type;
- change-request ID/number where non-secret;
- check/review/merge state classification;
- approval source/effect class;
- merge/cleanup/parity outcome.

Do not create a parallel delivery event database.

## Final security/falsification matrix

Attack at least:

- repo/origin substitution;
- cross-repository PR/push;
- malicious branch/ref names;
- arbitrary refspec injection;
- force push and admin merge attempts;
- stale base/head races;
- conflict-state confusion and unsafe continue/abort;
- protected credential access through forge/transport bridge;
- arbitrary `gh api` / shell escape attempts;
- raw token/error/path leakage into UI/telemetry;
- direct remote MCP calls without first-party approval UI;
- cancellation during long remote operations;
- branch cleanup before verified merge.

## End-to-end acceptance

Use a disposable/test repository or safe feature branch where practical, then authorized live GitHub proof against this repository when appropriate:

1. feature branch creation;
2. small safe change in fixture;
3. commit;
4. push via narrow transport while terminal network/credentials remain isolated;
5. PR creation;
6. checks/review read;
7. conflict scenario in disposable fixture and structured remediation;
8. merge with supported strategy under policy;
9. remote feature branch cleanup;
10. local integration sync and local/remote parity proof;
11. direct external MCP invocation of new read/write tools with hard relay policy intact.

## Closure review

Before closing 040:

- fresh independent read-only security/architecture review;
- zero unresolved P0/P1;
- all 040A–040D are CLOSED / VERIFIED;
- `pnpm verify:commit`, builds, Rust tests, focused Plan-040 deterministic contracts and relevant Plan-039 regressions pass;
- exact final release artifact is deployed if relay code changed;
- affected live MCP/GitHub boundaries are re-verified after deployment;
- docs and durable knowledge are reconciled;
- Plan 040 master status is updated only after real evidence.

## Exit criteria

Plan 040 can be marked CLOSED / VERIFIED only when the complete local-Git + remote-Git + GitHub PR/merge/cleanup workflow is proven without widening ordinary terminal credential/network authority.
