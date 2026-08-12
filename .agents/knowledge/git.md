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

This repository intentionally has:

- **no CI workflow**;
- **no unit-test suite**;
- a **mandatory local pre-commit gate** instead.

After `pnpm install`, Git is configured to use [`.githooks/pre-commit`](../../.githooks/pre-commit). Every commit must pass:

```sh
pnpm verify:commit
```

The command runs:

1. `bash scripts/check-agent-docs.sh`;
2. `pnpm lint`;
3. `pnpm typecheck`.

`pnpm lint` covers the configured JS/Vue linting plus Rust fmt/Clippy. `pnpm typecheck` covers generated Nuxt/Vue typing plus warnings-denied Rust `cargo check`.

### Hard commit rules

- A lint/typecheck failure means **do not commit**. Fix it first.
- Never use `git commit --no-verify`.
- Never change/disable `core.hooksPath` to bypass the tracked gate.
- If the hook is missing, run `bash scripts/install-git-hooks.sh` before committing.
- Never claim a commit was verified if it was created through a path that skipped the required local gate.
- There is no remote status check to catch a local bypass later, so this rule is stricter than a normal CI-backed workflow.

See [`../memories/no-ci-local-commit-gates.md`](../memories/no-ci-local-commit-gates.md).

## No unit-test policy

Do not introduce a unit-test framework or unit-test suite unless the user explicitly changes this policy. Existing deterministic protocol/security acceptance scripts are allowed as targeted local verification; they are not unit tests and they are not CI.

## Working a plan or task

For each independent change:

1. Branch from current `dev`.
2. Implement the change.
3. Run the relevant extra verification for the subsystem.
4. Run `pnpm verify:commit` until it passes.
5. Review `git status`; stage only intended files.
6. Commit only after the gate is green.
7. Push/open a PR targeting `dev` when requested/appropriate.
8. Record local verification in the PR body.

Do not merge a PR merely because GitHub says it is mergeable. There is no CI. Merge only when the user has authorized the merge and the required local verification is recorded.

`dev` → `main` promotion always requires an explicit user request.

## Branch names

```text
<type>/<short-kebab-description>
```

Recommended types: `feat/`, `fix/`, `chore/`, `docs/`, `refactor/`, `build/`, `perf/`, `style/`, `revert/`.

For plan work, include the plan/phase when useful:

```text
feat/029-p6-live-acceptance
fix/028-p19-relay-boundary
```

Keep branches short-lived and base/rebase them on `dev` rather than merging `dev` into the branch.

## Commits

Use Conventional Commits:

```text
<type>(<scope>): <subject>
```

Common types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `build`, `chore`, `revert`.

Common scopes: `chat`, `mcp`, `settings`, `ui`, `agents`, `deps`, `config`, `auth`, `db`.

Rules:

- imperative lowercase subject, no trailing period, ≤ 72 chars;
- explain *why* in the body when it is not obvious;
- keep commits atomic;
- do not use a commit as a checkpoint for broken code — the mandatory local gate must be green first.

## Pull requests

- Base feature/work PRs on `dev`.
- Title follows Conventional Commit style.
- Body states why, what changed, and **exact local verification performed**.
- Do not write “CI passed”; CI does not exist.
- A PR is not verification. It is a review/integration boundary.
- Use squash merge when the user authorizes merging so one PR becomes one integration commit.

## Dependency changes

`pnpm audit` must report zero known vulnerabilities before merging dependency changes. It is intentionally not part of every pre-commit run because it depends on registry/network state.

For dependency changes run, at minimum:

```sh
pnpm audit
pnpm verify:commit
pnpm build
```

If an override is needed, use the lowest patched version that actually resolves in this workspace and remove the override when upstream no longer needs it.

## Additional subsystem verification

The pre-commit gate is the minimum, not proof of runtime behavior.

Examples:

- UI/runtime change: `pnpm build && pnpm preview` plus browser verification when relevant.
- Rust security-boundary change: relevant deterministic scripts under `scripts/` and `cargo audit` when appropriate.
- MCP contract change: run the applicable deterministic phase/acceptance script.

Do not create a unit-test suite as a substitute for these explicit local checks unless the repository policy changes.

## What is committed

Committed intentionally:

- `.agents/**`;
- `AGENTS.md`;
- `.githooks/**`;
- `scripts/verify-commit.sh` and `scripts/install-git-hooks.sh`;
- `.mcp.json`, `.env.example`, `skills-lock.json`.

Do not commit repository-owned vendor/client-specific agent settings, instruction files, discovery links, or lifecycle hooks.

Never commit secrets, `.env`, generated build output, `node_modules`, caches, or editor state.

Before staging, inspect `git status`. Do not blindly `git add -A` after builds.

## Rules for agents

- Never commit or push unless the user asks.
- Never commit directly to `main` or `dev`.
- Before **every** commit, ensure `pnpm verify:commit` passed; the hook must not be bypassed.
- Never use `git push --force` on a shared branch.
- Do not amend/rebase commits already pushed and under review unless explicitly requested and safe.
- Before finish, follow [`self-improvement.md`](self-improvement.md) and keep plan/memory indexes truthful.

If a third-party skill conflicts with this file, **this repository rule wins**.
