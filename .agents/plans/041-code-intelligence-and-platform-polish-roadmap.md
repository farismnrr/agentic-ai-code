# Plan 041 — Code Intelligence and Platform Polish Roadmap

**Status:** CLOSED / VERIFIED / MERGED (2026-08-19)
**Created:** 2026-08-19
**Predecessor:** Plan 040 — Git + GitHub Delivery Workflow
**Plan family:** 041A–041C

## Goal

Improve practical coding-agent quality after the delivery loop is complete, focusing on proven code-intelligence gaps and targeted platform hygiene rather than adding new frameworks.

Plan 041 intentionally starts only after Plan 040 is CLOSED / VERIFIED so capability polish does not interfere with delivery-boundary work.

## Execution guide — sequential only

1. 041A closed/merged before 041B began.
2. 041B closed/merged before 041C began.
3. Each child re-read current `main` before implementation.
4. Plan 042 starts only after this Plan 041 closure is landed.
5. Runtime restart/resync remains operator-owned and is required only when a genuine live-runtime checkpoint needs it.

## Child plans

| Plan | Capability | Depends on | Status | Exit criterion |
| --- | --- | --- | --- | --- |
| 041A | LSP capability completion | 040 | CLOSED / VERIFIED / MERGED (2026-08-19) | Rust workspace-symbol and practical TS/Vue code-intelligence gaps were re-investigated and improved where upstream capabilities allowed, with truthful unsupported results otherwise |
| 041B | Dependency/toolchain hygiene | 041A | CLOSED / VERIFIED / MERGED (2026-08-19) | Actionable deprecated/security/toolchain debt was reduced through safe owner-level upgrades without forced transitive overrides |
| 041C | Observability/debugging polish | 041B | CLOSED / VERIFIED / MERGED (2026-08-19) | Measured observability gaps were closed using the existing telemetry/logging architecture without a parallel event system |

## Master todo

- [x] 041A — LSP capability completion
- [x] 041B — dependency/toolchain hygiene
- [x] 041C — observability/debugging polish

## Closure evidence

- 041A implementation and closeout merged through PRs #145 and #146.
- 041B security/toolchain remediation merged through PR #147; JavaScript audit and fresh RustSec audit were clean after the `h2` lockfile remediation.
- 041C observability/debugging implementation merged through PR #148 at remote `main` commit `8a02ccb46e7587b6ff3d31bf4ec98e7af0125a82`.
- 041C focused confidentiality/observability acceptance, Plan-039J regression, full typecheck, lint/fmt/clippy, architecture, maintainability, subagent/background/task-context gates, and current Plan-039H contract passed in the isolated task worktree.
- Existing Rust OTel instrumentation and W3C trace propagation were reviewed; no duplicate log-to-OTel bridge was introduced.
- Final internal adversarial review found zero unresolved P0/P1 across Plan 041 scope.
- No relay restart/resync is required for this docs-only closeout.

## Non-goals

- custom AST/index/RAG infrastructure;
- pretending unsupported language-server behavior works;
- blanket dependency overrides just to silence warnings;
- telemetry expansion without an operational debugging need;
- multi-agent orchestration (reserved for Plan 042).

## Closure criteria

- [x] every child plan verified and merged;
- [x] no regression to Plan-039 security or Plan-040 delivery boundaries;
- [x] code intelligence reports real server capabilities truthfully;
- [x] dependency changes are justified by owner-level upgrades and regression evidence;
- [x] observability remains bounded and secret-safe;
- [x] final review finds zero unresolved P0/P1.

**PLAN 041 CLOSED / VERIFIED / MERGED — NEXT: PLAN 042**
