# Plan 042B — Role Routing and Bounded Concurrency

**Status:** CLOSED / VERIFIED / MERGED
**Parent:** [Plan 042](042-multi-agent-orchestration-roadmap.md)
**Depends on:** 042A CLOSED / VERIFIED
**Delivery evidence:** PR #151; implementation head `b5ebffd84da2a2ab707358df06f8a29886bc7f0d`

## Goal

Allow the orchestrator to execute multiple independent ready tasks concurrently while preserving role-specific capability narrowing, worktree ownership, budgets, cancellation and policy boundaries.

## Implemented truth

- reuses the existing `SubagentRuntime` and one process-global `BackgroundTaskManager`; no second agent framework/runtime;
- deterministic role routing: planner→`plan`, researcher/general→`explore`, reviewer→`review`, verifier→`verify`, writer→`general-purpose`;
- bounded global concurrency 4 and per-parent concurrency 2 with deterministic backpressure;
- writer tasks always use isolated worktrees; non-writers remain read-only/shared;
- child authority is intersected with parent authority and profile maximum before dispatch;
- role/profile mismatch, capability widening and undeclared delivery authority fail closed;
- scheduler task identity is reused as background-task ownership identity;
- node/subtree/run cancellation propagates to owned child process trees;
- budget exhaustion, missing child state and policy denial settle truthfully and cannot deadlock the graph;
- a graph with no remaining progress becomes terminal `blocked`, not an indefinitely active run.

## Verification evidence

- `pnpm verify:042b` passed with 042A, subagent, background/worktree and task/context regressions;
- typecheck, lint, Rust fmt/clippy/check, architecture and maintainability gates passed;
- Plan-039/040/041 regression matrix passed before merge.

## Exit assessment

Bounded concurrent execution is merged with explicit authority narrowing and isolated writer ownership. No delivery/merge authority is inferred from child success.
