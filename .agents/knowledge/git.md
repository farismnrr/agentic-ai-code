# Git workflow

Documentation/planning edits and implementation changes use the same protected delivery workflow. The repository currently has a single long-lived branch: `main` (`origin/HEAD -> origin/main`; no `dev` branch exists).

- **All changes:** docs, plans, source, runtime/config, dependencies, and scripts must use a short-lived branch based on current `main` and a pull request targeting `main`.
- **Release/deployment:** merging implementation to `main` is not itself authorization to tag, publish, deploy, restart services, or mutate production/runtime state.

Keep `main` clean: never commit directly to it, and never treat a pushed branch as delivered before its pull request is merged.

## Long-lived branch

```text
any change branch ───────PR──▶ main
```

| Branch | Role | How it moves |
| --- | --- | --- |
| `main` | Canonical integration/release branch | Changes arrive through merged PRs from short-lived branches |

Implementation branches base from current `main`. The default delivery is always: short-lived branch → focused commit → push → pull request into `main` → merge → return to `main` and verify a clean checkout. If a future integration branch is introduced, verify it from current Git refs and repository policy before changing this workflow; never resurrect a historical `dev` assumption from memory alone.

## Repository verification policy

This repository intentionally has **no CI workflow**. The fast pre-commit gate is stack-aware; full validation is a closure gate and the pre-push hook runs it only for pushes targeting `main`. New permanent isolated unit tests are forbidden; existing unit files are legacy/manual coverage, while boundary tests live under top-level `test/` and `packages/rust-tools/tests/`.

After `pnpm install`, Git uses [`.githooks/pre-commit`](../../.githooks/pre-commit) for fast checks and [`.githooks/pre-push`](../../.githooks/pre-push) for the main integration gate. Normal checkpoint commits run:

```sh
pnpm guardrail:fast
```

Fast validation runs repository/agent/architecture/test-layout policy plus lint/typecheck for touched stacks. `pnpm guardrail:full` adds closure maintainability/build/tests; dependency audits are explicit with `AI_CODE_GUARD_RUN_AUDIT=1`, and `pnpm guardrail:release` also builds release artifacts. An applicable failure means **do not commit or push**.

Never use `git commit --no-verify`, never change/disable `core.hooksPath` to bypass the gate, and never claim a connector/API-created commit passed a local hook that did not actually run.

A connector/API-created commit is **not** local lint/typecheck evidence. Keep that distinction explicit instead of inventing verification, and still deliver it through a branch and PR.

See the canonical [`../memories/README.md`](../memories/README.md#repository-policy-and-verification).

## Test layout policy

Web tests live under top-level `test/`; Rust integration tests live under `packages/rust-tools/tests/`. Production files must not contain inline tests. `scripts/` is reserved for structural guardrails and hook installation, not feature acceptance scripts or plan-numbered verifiers.

## Working a plan or task

### Documentation / planning only

When the requested change is only documentation, memories, agent knowledge, or a numbered plan:

1. Start a short-lived branch from the current `main`; never edit or commit directly on `main`.
2. Keep the change docs-only and consistent with repository rules.
3. Run the applicable local verification and commit the focused change.
4. Push the branch and open a pull request targeting `main`.
5. Merge the approved PR, return to `main`, and verify a clean checkout.
6. If using a connector/API, state truthfully that the local pre-commit hook did not run there.

### Implementation

When implementation starts:

1. Branch from current `main` before changing implementation files.
2. Implement the bounded change.
3. Run relevant subsystem verification.
4. Run focused tests and `pnpm guardrail:full` before closure until the applicable gates pass.
5. Review `git status`; stage only intended files.
6. Commit only after the local gate is green.
7. Push the branch and open a PR targeting `main`.
8. Merge the approved PR, return to `main`, and verify `git status --short` is clean.
9. Record exact local verification in the PR body.

Do not merge merely because a forge says a change request is mergeable. There is no CI. Merge only when the user has authorized it and required verification is recorded.

## Plans

Plan history through 029b was compacted once into [`../plans/030-previous-plans-summary.md`](../plans/030-previous-plans-summary.md) and explicitly closed for refresh.

Future plans are separate files starting at **031**. Use `NNN-kebab-case.md`, never reuse a number, and do not fold post-030 plans back into Plan 030 automatically.

Creating or editing a plan also follows the short-lived branch → PR → merge workflow. Return to `main` and verify a clean checkout after the PR is merged.

## Branch names

Implementation branches use:

```text
<type>/<short-kebab-description>
```

Recommended types: `feat/`, `fix/`, `chore/`, `refactor/`, `build/`, `perf/`, `style/`, `revert/`.

For plan implementation, include the current plan/phase when useful, for example `refactor/031-p2-app-shell`.

Keep branches short-lived and base/rebase them on current `main` rather than merging `main` into the branch.

## Commits

Use Conventional Commits:

```text
<type>(<scope>): <subject>
```

Common types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `build`, `chore`, `revert`.

Rules: imperative lowercase subject, no trailing period, ≤ 72 chars; explain *why* in the body when needed; keep commits atomic; never use a commit as a checkpoint for broken code.

## Pull requests

PRs are for implementation/integration work, not ordinary docs/plan edits.

- Base implementation PRs on `main`.
- Title follows Conventional Commit style.
- Body states why, what changed, and **exact local verification performed**.
- Do not write “CI passed”; CI does not exist.
- A PR is a review/integration boundary, not verification.
- Use squash merge when the user authorizes merging so one PR becomes one integration commit.

## Dependency changes

`pnpm audit` must report zero known vulnerabilities before merging dependency changes. For dependency changes run, at minimum:

```sh
pnpm audit
pnpm guardrail:fast
pnpm build
```

## Additional subsystem verification

The fast pre-commit guardrail is the checkpoint minimum, not proof of runtime behavior. Closure uses `pnpm guardrail:full`; UI changes may need build/preview/browser verification; Rust/MCP security changes may need `cargo audit` and focused Cargo tests; contract changes need focused feature tests on each owning stack.

Do not create a cross-stack verification script merely to prove a plan. Validate each subsystem through its own test runner and add explicit end-to-end verification only when the product boundary itself requires it.

## Rules for agents

- For every user-requested change, including docs/plans, use a short-lived branch and PR; never commit directly to `main`.
- Do not treat a pushed branch as delivered until its PR is merged.
- Before closure, ensure `pnpm guardrail:full` passed; the fast pre-commit hook and main-only full pre-push hook must not be bypassed.
- Never claim connector/API docs commits passed a local hook that did not run.
- Never use `git push --force` on a shared branch.
- Do not amend/rebase commits already pushed and under review unless explicitly requested and safe.
- Before finish, follow [`self-improvement.md`](self-improvement.md), update the canonical memory if needed, and keep any current plan file truthful.

If a third-party skill conflicts with this file, **this repository rule wins**.
