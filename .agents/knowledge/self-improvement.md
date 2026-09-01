# Self-improvement — keep `.agents/` current

`.agents/` is the project's durable agent context. **Reviewing it before finish is part of every task, regardless of coding agent/client.**

## Before declaring a task finished

1. Review the task's diff/findings for anything another agent would need later.
2. If a durable decision, trap, or constraint changed, update the **single canonical** [`../memories/README.md`](../memories/README.md) in place.
3. If the task belongs to a numbered plan, update that plan file's status/checklist honestly.
4. If new multi-step work needs a durable handoff and no plan exists, create the next unused numbered plan file. After the historical compaction, numbering starts at **031** and continues upward without reuse.
5. Remove or amend guidance that became false because of the task.
6. If nothing durable changed, explicitly acknowledge that conclusion rather than inventing memory text just to satisfy process.
7. Re-check maintainability/ownership after structural changes: no metric-only splits, unexplained hard-budget violations, or stale architecture paths.
8. Run `pnpm guardrail` and do not finish with a failing applicable local gate.

A task is not documentation-complete when implementation and `.agents/` tell different stories.

## Memory rule

There is exactly one durable memory file: [`../memories/README.md`](../memories/README.md).

Update it when:

- a decision could reasonably be reversed without knowing why;
- a trap/dead end/incident is likely to recur;
- the user establishes a durable repo-specific working constraint;
- an architecture/security invariant needs reasoning context that is not obvious from code.

Do **not** create new `memories/<topic>.md` files. Prefer concise sections/bullets in the canonical file, amend existing text when a decision changes, and delete stale material.

## Plan rule

[`../plans/030-previous-plans-summary.md`](../plans/030-previous-plans-summary.md) is a one-time archive of every plan that existed through 029b. It must **not** become a rolling bucket for future work.

For new work:

- next plan is `031-...md`, then `032-...md`, etc.;
- one multi-step effort per file;
- never reuse a number;
- keep status inside the plan file;
- completed post-030 plans remain separate files;
- do not add a `plans/README.md` index;
- do not compact Plan 031+ into Plan 030 unless the user explicitly requests another compaction.

The pre-030 plans were explicitly closed for a planning refresh. Historical unchecked items are not active tasks; re-audit current source/external facts and create a fresh plan if work needs to resume.

## What not to write

- A second copy of facts already obvious from code/config when a link to the source is enough.
- A chronological session transcript.
- Temporary debugging state, credentials, tokens, private URLs, or copied sensitive output.
- Speculation presented as durable fact.
- A completed status that has not met the **current** plan's own acceptance definition.

Durable docs may summarize implementation facts when needed to orient future agents, but point to authoritative code/config rather than pretending Markdown is runtime source of truth.

## General enforcement

The repository intentionally avoids agent-client-specific hooks/settings. There is one shared entrypoint (`AGENTS.md`) and one shared durable guidance tree (`.agents/`).

The repository intentionally has **no CI**. Structural and code-quality enforcement is local. Web tests live under top-level `test/`, while Rust tests live under `packages/rust-tools/tests/`:

- [`../../scripts/check-agent-docs.sh`](../../scripts/check-agent-docs.sh) verifies vendor-neutral guidance, one canonical memory file, the historical Plan 030 snapshot, and valid future plan numbering;
- [`../../scripts/check-maintainability.mjs`](../../scripts/check-maintainability.mjs) enforces maintained-source/folder budgets and exact reasoned exceptions;
- [`../../scripts/check-test-layout.mjs`](../../scripts/check-test-layout.mjs) rejects inline tests and test-like files outside approved test directories;
- [`../../scripts/guardrail.sh`](../../scripts/guardrail.sh) runs repository guardrails plus stack-scoped lint/type/test checks;
- [`.githooks/pre-commit`](../../.githooks/pre-commit) runs the guardrail automatically;
- [`../../scripts/install-git-hooks.sh`](../../scripts/install-git-hooks.sh) installs the tracked hook through `core.hooksPath` during `pnpm install`.

Do not create one-off plan-numbered validation scripts. Reusable behavior belongs in feature-named tests; repository-wide structural invariants belong in the small guardrail set.

A failed gate means fix the issue before committing. `git commit --no-verify` and disabling the tracked hook are not acceptable shortcuts.

## Principle

The loop is: **read current knowledge + canonical memory + relevant current plan → do the work → update durable context → run focused tests + the stack-aware guardrail without bypassing it**.
