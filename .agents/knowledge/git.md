# Git workflow

Documentation/planning edits and implementation changes intentionally use different workflows, but the repository currently has a single long-lived branch: `main` (`origin/HEAD -> origin/main`; no `dev` branch exists).

- **Docs/plans:** documentation-only and planning-only edits may be committed directly to `main` when the operator requests or accepts that direct-docs workflow. Do **not** create a branch or pull request for docs-only work by default.
- **Implementation:** source/runtime/config/dependency/script changes must use a short-lived branch based on current `main` and a pull request targeting `main`.
- **Release/deployment:** merging implementation to `main` is not itself authorization to tag, publish, deploy, restart services, or mutate production/runtime state.

A docs/plan edit must stay docs-only. If the same task starts changing executable code, runtime config, dependencies, migrations, scripts, or other implementation surfaces, switch to an implementation branch before making those changes.

## Long-lived branch

```text
docs / plans ───────────────▶ main
implementation branch ──PR──▶ main
```

| Branch | Role | How it moves |
| --- | --- | --- |
| `main` | Canonical integration/release branch | Direct docs/plans where appropriate; implementation via PRs from short-lived branches |

Implementation branches base from current `main`. The default delivery is always: short-lived branch → focused commit → push → pull request into `main` → merge → return to `main` and verify a clean checkout. If a future integration branch is introduced, verify it from current Git refs and repository policy before changing this workflow; never resurrect a historical `dev` assumption from memory alone.

## Repository verification policy

This repository intentionally has **no CI workflow**, requires standalone test files under sibling `tests/` directories, and has a **mandatory local pre-commit gate** for normal local commits.

After `pnpm install`, Git uses [`.githooks/pre-commit`](../../.githooks/pre-commit). Every normal local commit must pass:

```sh
pnpm verify:commit
```

The command runs repository policy checks, agent-doc integrity, `pnpm lint`, and `pnpm typecheck`. Lint/typecheck failures mean **do not commit**.

Never use `git commit --no-verify`, never change/disable `core.hooksPath` to bypass the gate, and never claim a connector/API-created commit passed a local hook that did not actually run.

A connector/API-created docs-only commit is allowed by the direct-docs workflow, but it is **not** local lint/typecheck evidence. Keep that distinction explicit instead of inventing verification.

See the canonical [`../memories/README.md`](../memories/README.md#repository-policy-and-verification).

## Test layout policy

Unit/integration test files must live under a dedicated sibling `tests/` directory. Production files must not contain inline tests or references to test modules. Deterministic protocol/security acceptance scripts remain allowed as separate local verification and are not CI.

## Working a plan or task

### Documentation / planning only

When the requested change is only documentation, memories, agent knowledge, or a numbered plan:

1. Edit the canonical file directly on current `main` when using the accepted direct-docs workflow.
2. Keep the change docs-only.
3. Do not create a branch or PR just for the documentation/plan edit.
4. If using a connector/API, state truthfully that the local pre-commit hook did not run there.
5. Keep the plan/memory files consistent with their repository rules.

### Implementation

When implementation starts:

1. Branch from current `main` before changing implementation files.
2. Implement the bounded change.
3. Run relevant subsystem verification.
4. Run `pnpm verify:commit` until it passes.
5. Review `git status`; stage only intended files.
6. Commit only after the local gate is green.
7. Push the branch and open a PR targeting `main`.
8. Merge the approved PR, return to `main`, and verify `git status --short` is clean.
9. Record exact local verification in the PR body.

Do not merge merely because a forge says a change request is mergeable. There is no CI. Merge only when the user has authorized it and required verification is recorded.

## Plans

Plan history through 029b was compacted once into [`../plans/030-previous-plans-summary.md`](../plans/030-previous-plans-summary.md) and explicitly closed for refresh.

Future plans are separate files starting at **031**. Use `NNN-kebab-case.md`, never reuse a number, and do not fold post-030 plans back into Plan 030 automatically.

Creating or editing a plan is a documentation-only operation and therefore may happen directly on `main` under the accepted direct-docs workflow. **Implementation of that plan** happens on one or more short-lived branches/PRs as appropriate.

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
pnpm verify:commit
pnpm build
```

## Additional subsystem verification

The pre-commit gate is the minimum, not proof of runtime behavior. UI changes may need build/preview/browser verification; Rust/MCP security changes may need `cargo audit` and deterministic scripts; contract changes need the applicable contract gate.

Keep standalone tests and deterministic acceptance checks complementary; neither replaces the repository's explicit local gates.

## Rules for agents

- For user-requested docs/plans, edit directly on `main` when the accepted direct-docs workflow applies; do not waste time creating a branch or PR.
- Never commit implementation/runtime/config/dependency/script changes directly to `main`; use a short-lived implementation branch and PR.
- Direct `main` commits are documentation/planning-only, never an implementation shortcut.
- Before **every normal local implementation commit**, ensure `pnpm verify:commit` passed; the hook must not be bypassed.
- Never claim connector/API docs commits passed a local hook that did not run.
- Never use `git push --force` on a shared branch.
- Do not amend/rebase commits already pushed and under review unless explicitly requested and safe.
- Before finish, follow [`self-improvement.md`](self-improvement.md), update the canonical memory if needed, and keep any current plan file truthful.

If a third-party skill conflicts with this file, **this repository rule wins**.
