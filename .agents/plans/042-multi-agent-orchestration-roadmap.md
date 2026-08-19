# Plan 042 — Multi-Agent Orchestration Roadmap

**Status:** CLOSED / VERIFIED / MERGED
**Created:** 2026-08-19
**Predecessor:** Plan 041 — Code Intelligence and Platform Polish
**Plan family:** 042A–042D

## Goal

Evolve the existing parent-managed subagent/background-agent capabilities into a bounded multi-agent orchestrator that can decompose work, assign independent roles, coordinate dependencies, collect evidence, and integrate results without creating an uncontrolled peer-to-peer swarm or widening authority.

The orchestrator builds on the stable primitives from Plans 039–041 rather than replacing them with a new agent framework.

## Core design principles

- one parent/orchestrator remains accountable for task truth and final integration;
- children inherit equal or narrower authority;
- explicit task graph/dependencies replace free-form agent-to-agent chatter;
- writer agents remain isolated in owned worktrees;
- read-only/review/planning agents must not mutate;
- bounded concurrency, depth, turns, tokens, wall time and retained output;
- cancellation propagates predictably;
- evidence/results are structured and sanitized;
- no hidden reasoning exchange is required;
- Git/PR delivery authority remains separately policy-gated through Plan 040.

## Execution guide — sequential only

1. Start 042A only after Plan 041 is CLOSED / VERIFIED.
2. Close/merge each child plan before the next begins.
3. Re-read current `main` and existing subagent/background contracts before every child plan.
4. Use deterministic fixtures first; introduce live multi-agent workflows only after lower-level orchestration primitives are proven.
5. Do not run sibling implementation child plans concurrently on the same worktree.
6. Do not restart/redeploy per child. Runtime restart/resync happens only at a genuine final live-runtime checkpoint.

## Child plans

| Plan | Capability | Depends on | Status | Evidence |
| --- | --- | --- | --- | --- |
| 042A | Task graph + orchestrator state machine | 041 | CLOSED / VERIFIED / MERGED | PR #150; deterministic graph/state acceptance |
| 042B | Role/capability routing + concurrent execution | 042A | CLOSED / VERIFIED / MERGED | PR #151; bounded scheduler/runtime regressions |
| 042C | Evidence integration + conflict/reconciliation workflow | 042B | CLOSED / VERIFIED / MERGED | PR #152; evidence/reconciliation acceptance |
| 042D | Multi-agent UX, observability, adversarial acceptance + closure | 042C | CLOSED / VERIFIED / MERGED | PR #153; independent adversarial remediation; deployed production-image orchestration smoke |

## Master todo

- [x] 042A — task graph + scheduler/state machine
- [x] 042B — role/capability routing + bounded concurrency
- [x] 042C — evidence integration + reconciliation
- [x] 042D — UX/observability/security closure and live proof

## Explicit non-goals

- autonomous peer-to-peer agents granting each other capabilities;
- shared unrestricted shell or credentials between agents;
- agents merging/pushing merely because another child asks them to;
- unlimited recursive delegation;
- custom distributed queue/service unless measured needs exceed the existing app/runtime primitives;
- free-form hidden chain-of-thought sharing;
- replacing Git worktree isolation or Plan-040 delivery contracts.

## Closure criteria

Plan 042 closes only when:

- 042A–042D are individually CLOSED / VERIFIED;
- independent tasks can run concurrently without write/worktree collisions;
- dependency ordering and failure propagation are deterministic;
- child authority never exceeds parent/operator policy;
- conflicting child outputs are surfaced and reconciled explicitly;
- cancellation/budget/timeouts clean up child processes/worktrees safely;
- first-party UX exposes structured progress/evidence without hidden reasoning;
- fresh adversarial review reports zero unresolved P0/P1;
- live end-to-end scenarios prove useful orchestration rather than merely parallel tool calls.

## Final closure evidence — 2026-08-19

- PR #153 merged Plan 042D with merge commit `407e95e56695d64bf6bc8d4da7daf396cb7f2aee`.
- Fresh independent adversarial review found stale writer-identity validation; the issue was remediated with regression coverage before merge. No unresolved P0/P1 remained after remediation.
- Exact merged `origin/main` was exported and built as production image `ai-code-app:042-final`; OCI revision label is `407e95e56695d64bf6bc8d4da7daf396cb7f2aee`.
- Deployed Nuxt server entry SHA256 is `7adbd0f14d02b56a71c583bdf58f50731a6bd0e41b1c6d5829034217059805a3`; the active container serves HTTP 200 and contains the merged orchestration bundle.
- Installed relay binary matches the release candidate SHA256 `69e4e3cfb0619615db1862ec3ccb4f98d473733a816fcc5fdf6bc34ff3e1ab64`; relay service is active.
- A live production-image orchestration smoke exercised the real merged `OrchestratorScheduler`, `BackgroundTaskManager`, `SubagentRuntime`, task graph and reconciliation modules inside the active container. Two dependency-ready children started concurrently, a dependent reviewer started only after its prerequisite completed, a separate running child was cancelled to terminal `cancelled`, and reconciliation completed with zero issues/blockers.
- The live smoke intentionally used a deterministic child execution port rather than user/provider credentials. Provider-model invocation was not claimed as part of this live proof; existing subagent/provider regressions remained green in the final composed matrix.
- Final source validation included `pnpm verify:042d`, lint, typecheck, commit gate, Rust workspace tests, Nuxt build and release build.

All child plans 042A–042D are closed, merged and reconciled; master Plan 042 is therefore CLOSED / VERIFIED / MERGED.
