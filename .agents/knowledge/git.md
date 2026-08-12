# Git workflow

**Never commit directly to `main` or `dev`.** Every change lands through a branch and pull request, including docs/config/`.agents` changes.

## Long-lived branches

```text
feature branch  ──PR──▶  dev  ──PR──▶  main
```

| Branch | Role | How it moves |
| --- | --- | --- |
| `dev` | Integration branch | PRs from short-lived work branches |
| `main` | Release branch | PR from `dev`, only when the user explicitly asks |

Feature branches base from `dev`, never `main`.

## Repository verification policy

This repository intentionally has **no CI workflow**, **no unit-test suite**, and a **mandatory local pre-commit gate**.

After `pnpm install`, Git uses [`.githooks/pre-commit`](../../.githooks/pre-commit). Every commit must pass:

```sh
pnpm verify:commit
```

The command runs repository policy checks, agent-doc integrity, `pnpm lint`, and `pnpm typecheck`. Lint/typecheck failures mean **do not commit**.

Never use `git commit --no-verify`, never change/disable `core.hooksPath` to bypass the gate, and never claim a connector/API-created commit passed a local hook that did not actually run.

See the canonical [`../memories/README.md`](../memories/README.md#repository-policy-and-verification).

## No unit-test policy

Do not introduce a unit-test framework or unit-test suite unless the user explicitly changes this policy. Existing deterministic protocol/security acceptance scripts are allowed as targeted local verification; they are not unit tests and they are not CI.

## Working a plan or task

For each independent change:

1. Branch from current `dev`.
2. Implement the change.
3. Run relevant subsystem verification.
4. Run `pnpm verify:commit` until it passes.
5. Review `git status`; stage only intended files.
6. Commit only after the local gate is green.
7. Push/open a PR targeting `dev` when requested/appropriate.
8. Record exact local verification in the PR body.

Do not merge merely because GitHub says a PR is mergeable. There is no CI. Merge only when the user has authorized it and required verification is recorded.

`dev` → `main` promotion always requires an explicit user request.

## Plans

Plan history through 029b was compacted once into [`../plans/030-previous-plans-summary.md`](../plans/030-previous-plans-summary.md) and explicitly closed for refresh.

Future plans are separate files starting at **031**. Use `NNN-kebab-case.md`, never reuse a number, and do not fold post-030 plans back into Plan 030 automatically.

## Branch names

```text
<type>/<short-kebab-description>
```

Recommended types: `feat/`, `fix/`, `chore/`, `docs/`, `refactor/`, `build/`, `perf/`, `style/`, `revert/`.

For plan work, include the current plan/phase when useful, for example `feat/031-live-mcp-acceptance`.

Keep branches short-lived and base/rebase them on `dev` rather than merging `dev` into the branch.

## Commits

Use Conventional Commits:

```text
<type>(<scope>): <subject>
```

Common types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `build`, `chore`, `revert`.

Rules: imperative lowercase subject, no trailing period, ≤ 72 chars; explain *why* in the body when needed; keep commits atomic; never use a commit as a checkpoint for broken code.

## Pull requests

- Base feature/work PRs on `dev`.
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

Do not create a unit-test suite as a substitute for these explicit local checks unless repository policy changes.

## Rules for agents

- Never commit or push unless the user asks.
- Never commit directly to `main` or `dev`.
- Before **every** normal local commit, ensure `pnpm verify:commit` passed; the hook must not be bypassed.
- Never use `git push --force` on a shared branch.
- Do not amend/rebase commits already pushed and under review unless explicitly requested and safe.
- Before finish, follow [`self-improvement.md`](self-improvement.md), update the canonical memory if needed, and keep any current plan file truthful.

If a third-party skill conflicts with this file, **this repository rule wins**.
