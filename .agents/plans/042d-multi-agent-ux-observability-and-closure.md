# Plan 042D — Multi-Agent UX, Observability, Adversarial Acceptance, and Closure

**Status:** IMPLEMENTED / SOURCE VERIFIED / LIVE CLOSURE PENDING
**Parent:** [Plan 042](042-multi-agent-orchestration-roadmap.md)
**Depends on:** 042C CLOSED / VERIFIED

## Goal

Integrate the multi-agent task graph, bounded concurrency, role routing, evidence reconciliation and Plan-040 delivery boundaries into one understandable first-party experience, then close Plan 042 with fresh adversarial and live evidence.

## Implemented source truth

- existing category-driven tool presentation classifies `orchestrator_*` as Agent task operations; no bespoke orchestration component framework was added;
- graph/reconciliation presentation exposes bounded counts/states rather than prompts, source, raw child output or private worktree paths;
- approval summaries keep task IDs, expected writer HEAD values and prompt/task text hidden;
- Plan-035 telemetry allowlist now explicitly admits only bounded orchestration run/node/state/count semantic fields;
- dispatch/poll/cancel/reconcile events carry allowlisted orchestration semantics and reuse the existing logger/OTel sanitizer path;
- raw child/provider errors, credentials, hidden reasoning, source/patch contents and private absolute paths remain excluded;
- composed deterministic acceptance covers graph bounds, role/capability narrowing, concurrency/backpressure, cancellation, reconciliation blockers, stale writer identity, presentation confidentiality and telemetry allowlisting;
- Plan-039 UX/observability, Plan-040 delivery and Plan-041 observability regressions remain part of final closure verification.

## Adversarial/source verification evidence

- cyclic/deep/oversized graphs fail closed;
- child capability/profile widening is denied;
- writer shared-checkout mutation is structurally unavailable through scheduler routing;
- stale completion/generation/lease is rejected;
- concurrency exhaustion queues deterministically and terminal policy-denied graphs do not spin;
- reviewer disagreement and P0/P1 findings block delivery readiness;
- poisoned/sensitive presentation fields are hidden or sanitized;
- delivery cannot advance from unreviewed/unintegrated/stale writer state;
- direct delivery remains separately constrained by Plan-040 Git/forge policy.

## Remaining closure evidence

Before changing this status to CLOSED / VERIFIED / MERGED:

- run the exact final composed deterministic matrix on the candidate;
- perform fresh adversarial P0/P1 review on the merged candidate;
- prove one live first-party orchestration workflow with real child dispatch, dependency ordering, evidence collection/reconciliation and cancellation/cleanup behavior;
- deploy/restart only if the live first-party runtime is not already running the merged candidate;
- reconcile final docs and master Plan 042 status;
- leave zero unresolved P0/P1 and no stale implementation branches/worktrees.

## Exit criteria

Plan 042 closes only when multi-agent orchestration is bounded, inspectable, cancellable, evidence-driven and no less secure than the single-parent subagent/background model it extends.
