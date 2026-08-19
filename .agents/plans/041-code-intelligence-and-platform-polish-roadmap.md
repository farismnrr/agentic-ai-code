# Plan 041 — Code Intelligence and Platform Polish Roadmap

**Status:** IN PROGRESS
**Created:** 2026-08-19
**Predecessor:** Plan 040 — Git + GitHub Delivery Workflow
**Plan family:** 041A–041C

## Goal

Improve practical coding-agent quality after the delivery loop is complete, focusing on proven code-intelligence gaps and targeted platform hygiene rather than adding new frameworks.

Plan 041 intentionally starts only after Plan 040 is CLOSED / VERIFIED so capability polish does not interfere with delivery-boundary work.

## Execution guide — sequential only

Do not implement 041A–041C together.

1. Begin 041A only after Plan 040 is CLOSED / VERIFIED.
2. Close/merge 041A before starting 041B.
3. Close/merge 041B before starting 041C.
4. Re-read current `main` before each child plan; previously observed gaps may have changed through dependency upgrades or Plan 040 work.
5. Start Plan 042 only after 041 is CLOSED / VERIFIED.
6. Follow [Plans 040–042 execution guide](../roadmap-execution-guide-040-042.md): do not restart/redeploy the relay per child/phase. Runtime restart and external MCP client connector/action resync are operator-owned and occur only at a genuine live-runtime checkpoint after the largest safe implementation batch.

## Child plans

| Plan | Capability | Depends on | Status | Exit criterion |
| --- | --- | --- | --- | --- |
| 041A | LSP capability completion | 040 | CLOSED / VERIFIED / MERGED (2026-08-19) | Rust workspace-symbol and practical TS/Vue code-intelligence gaps are re-investigated and improved where upstream capabilities allow, with truthful unsupported results otherwise |
| 041B | Dependency/toolchain hygiene | 041A | CLOSED / VERIFIED / MERGED (2026-08-19) | Actionable deprecated/security/toolchain debt is reduced through safe owner-level upgrades without forced transitive overrides |
| 041C | Observability/debugging polish | 041B | IMPLEMENTED / VERIFIED — MERGE PENDING | Only measured observability gaps are closed using existing telemetry/logging architecture without a parallel event system |

## Master todo

- [x] 041A — LSP capability completion
- [x] 041B — dependency/toolchain hygiene
- [ ] 041C — observability/debugging polish

## Non-goals

- custom AST/index/RAG infrastructure;
- pretending unsupported language-server behavior works;
- blanket dependency overrides just to silence warnings;
- telemetry expansion without an operational debugging need;
- multi-agent orchestration (reserved for Plan 042).

## Closure criteria

- every child plan independently verified and merged;
- no regression to Plan-039 security or Plan-040 delivery boundaries;
- code intelligence reports real server capabilities truthfully;
- dependency changes are justified by owner-level upgrades and full regression evidence;
- observability remains bounded and secret-safe;
- independent final review finds zero unresolved P0/P1.
