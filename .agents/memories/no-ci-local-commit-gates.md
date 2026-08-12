# No CI; local commit gates are mandatory

**Decision (2026-08-12):** this repository intentionally has **no CI workflow** and **no unit-test suite**. Quality enforcement happens locally before every commit.

## Required commit gate

Every normal Git commit must pass the tracked pre-commit hook at [`.githooks/pre-commit`](../../.githooks/pre-commit). `pnpm install` activates the hook through [`../../scripts/install-git-hooks.sh`](../../scripts/install-git-hooks.sh).

The hook runs [`../../scripts/verify-commit.sh`](../../scripts/verify-commit.sh), which requires all of the following to succeed:

1. `bash scripts/check-repo-policy.sh`
2. `bash scripts/check-agent-docs.sh`
3. `pnpm lint`
4. `pnpm typecheck`

The repository-policy check fails if a tracked GitHub Actions workflow or conventional unit-test suite is introduced. The root lint/type commands cover both application and native code:

- `pnpm lint` = ESLint + Rust formatting check + Clippy with warnings denied.
- `pnpm typecheck` = Nuxt production build + explicit generated-project `vue-tsc` + Rust `cargo check` with warnings denied.

The Nuxt build is intentionally part of typecheck. Earlier `nuxt prepare`/bare `nuxt typecheck` paths were not strong enough for this repository's generated project.

A failed gate means **do not commit**. Fix the failure first.

## Bypass policy

- Do not use `git commit --no-verify`.
- Do not disable or replace `core.hooksPath` to avoid the repository gate.
- Do not make routine commits through a path that bypasses the hook and then claim them as verified.
- If the hook is missing on a clone/worktree, run `bash scripts/install-git-hooks.sh` before committing.

The hook is a local guardrail, not a remote proof system. Git itself technically permits local hooks to be bypassed, so the repository rule and agent instructions explicitly forbid doing so. With no CI/server-side verification, the working developer/agent is responsible for respecting the gate.

## No CI

`.github/workflows/` is intentionally absent. Pull requests do not wait for a GitHub Actions status, and release packaging is not performed by CI.

PR descriptions must state the local verification performed. A PR is not merge-ready merely because GitHub reports it mergeable.

Any older plan/memory text that describes CI or GitHub Actions as a current gate is historical evidence and is superseded by this policy unless that document is later updated explicitly.

## No unit tests

Do not introduce a unit-test framework or unit-test suite by default. Existing deterministic acceptance/security scripts may remain when they verify protocol or security boundaries, but they are not a CI service and are not a substitute for the mandatory lint/typecheck commit gate.

The local repository-policy script rejects conventional `test/`, `tests/`, `__tests__/`, `*.test.*`, `*.spec.*`, Rust `#[cfg(test)]` modules, and package `test` scripts.

If the project later wants unit tests or CI again, treat that as an explicit policy change rather than quietly reintroducing either one.

## Dependency changes

`pnpm audit` remains a manual dependency-change/merge gate. There is no CI safety net, so dependency changes must run it explicitly in addition to the mandatory commit gate.
