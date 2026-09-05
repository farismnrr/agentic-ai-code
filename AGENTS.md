# AGENTS.md

All repository-owned agent guidance lives in **[`.agents/`](.agents/)**. Start at [`.agents/README.md`](.agents/README.md), then read the knowledge and skill files relevant to the task, the single canonical [memory](.agents/memories/README.md), and any current numbered plan file.

This is the **only repository agent entrypoint**. Do not add client/vendor-specific agent instruction files or settings; shared guidance must remain usable by any coding agent.

This repository intentionally has **no CI**. Quality enforcement is local, but validation must stay proportional to the changed subsystem:

- New permanent isolated JavaScript/TypeScript/Vue unit tests are forbidden; temporary unit tests must be removed before staging. Existing `test/unit/` files are legacy/manual coverage, while boundary tests live under top-level `test/`.
- Rust tests live under the single native package at `packages/rust-tools/tests/` and run with `pnpm test:rust`.
- `scripts/` is reserved for repository guardrails and hook installation. Do not add plan-numbered `verify-*`, `phase-*`, acceptance, or one-off validation scripts.
- New plans must describe feature tests, not generate a new verification script for the plan number.
- A Nuxt-only change must not compile, lint, or test Rust unless the change actually touches a shared cross-stack contract; the inverse applies to Rust-only work.

Before every normal local commit, run:

```sh
pnpm guardrail:fast
```

The tracked pre-commit hook runs the fast guard after `pnpm install`; the pre-push hook runs full validation only for pushes targeting `main`. Never bypass either hook or commit/push while an applicable gate is failing.

Implementation delivery always uses a short-lived branch from `main`: commit the focused change, push the branch, open a pull request into `main`, merge the approved PR, then return to `main` and verify the checkout is clean. Do not implement directly on `main` or treat a pushed branch as delivered without the PR merge.

Historical plans through 029b are compacted and closed in [Plan 030](.agents/plans/030-previous-plans-summary.md). Future plans start at **031** and remain separate incrementing files. Historical references to removed acceptance scripts are evidence of past execution, not templates for new validation.

Before declaring work complete, follow the closeout rules in [`.agents/knowledge/self-improvement.md`](.agents/knowledge/self-improvement.md).

Keep this file a pointer. New durable guidance belongs in `.agents/`, not here.
