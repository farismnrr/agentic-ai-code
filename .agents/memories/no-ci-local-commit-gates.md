# No CI; local commit gates are mandatory

**Decision (2026-08-12):** this repository intentionally has **no CI workflow** and **no unit-test suite**. Quality enforcement happens locally before every commit.

## Required commit gate

Every normal Git commit must pass the tracked pre-commit hook at [`.githooks/pre-commit`](../../.githooks/pre-commit). `pnpm install` activates the hook through [`../../scripts/install-git-hooks.sh`](../../scripts/install-git-hooks.sh).

The hook runs [`../../scripts/verify-commit.sh`](../../scripts/verify-commit.sh), which requires all of the following to succeed:

1. `bash scripts/check-agent-docs.sh`
2. `pnpm lint`
3. `pnpm typecheck`

The root commands cover both application and native code:

- `pnpm lint` = ESLint + Rust formatting check + Clippy with warnings denied.
- `pnpm typecheck` = generated Nuxt/Vue type verification + Rust `cargo check` with warnings denied.

A failed gate means **do not commit**. Fix the failure first.

## Bypass policy

- Do not use `git commit --no-verify`.
- Do not disable or replace `core.hooksPath` to avoid the repository gate.
- Do not make routine commits through a path that bypasses the hook and then claim them as verified.
- If the hook is missing on a clone/worktree, run `bash scripts/install-git-hooks.sh` before committing.

The hook is a local guardrail, not a remote proof system. The working developer/agent remains responsible for actually running and respecting it.

## No CI

`.github/workflows/` is intentionally absent. Pull requests do not wait for a GitHub Actions status, and release packaging is not performed by CI.

PR descriptions must state the local verification performed. A PR is not merge-ready merely because GitHub reports it mergeable.

## No unit tests

Do not introduce a unit-test framework or unit-test suite by default. Existing deterministic acceptance/security scripts may remain when they verify protocol or security boundaries, but they are not a CI service and are not a substitute for the mandatory lint/typecheck commit gate.

If the project later wants unit tests or CI again, treat that as an explicit policy change rather than quietly reintroducing either one.

## Dependency changes

`pnpm audit` remains a manual dependency-change/merge gate. There is no CI safety net, so dependency changes must run it explicitly in addition to the mandatory commit gate.
