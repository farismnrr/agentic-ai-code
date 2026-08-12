# Tooling

## Environment and runtime config

Copy [`.env.example`](../../.env.example) → `.env` (gitignored) on a fresh clone. **`.env.example` is the environment-key inventory/source of truth**; keep it aligned with `nuxt.config.ts`/runtime consumers when configuration changes instead of maintaining a second exhaustive key list here.

Current configuration groups include:

- dev server/public site URL;
- router/model-provider credentials;
- workspace root;
- PostgreSQL (host and compose override);
- session sealing;
- SMTP and optional OAuth providers;
- OpenTelemetry/Jaeger/Loki.

Not every key is required for every workflow. Fill the values needed by the subsystem you are running; never commit secrets or real credentials to Markdown, plans, memories, fixtures, or examples.

### Stable conventions

- `NUXT_PORT` — dev port. Defaults to **3333** via `devServer.port` in `nuxt.config.ts`; the original dev machine reserved several common ports.
- `NUXT_HOST` — leave unset for the safe localhost-only default. When intentionally exposing dev to another device, bind to a specific trusted interface rather than `0.0.0.0`.
- `NUXT_PUBLIC_SITE_URL` — public runtime config; browser-visible by definition.
- `NUXT_WORKSPACES_ROOT` — operator-owned workspace filesystem boundary for the Nuxt application. Do not silently fall back to unrestricted filesystem browsing.

Nuxt runtime config binding is by convention: `NUXT_FOO_BAR` → `runtimeConfig.fooBar`, `NUXT_PUBLIC_FOO` → `runtimeConfig.public.foo`. A key must be represented by the runtime/config path that consumes it; adding an arbitrary environment variable does not automatically create application behavior. Prefer `useRuntimeConfig()`/Nuxt config surfaces in application code instead of ad-hoc `process.env` reads.

The Rust `relay-agent` has its own CLI/environment contract under [`../../packages/relay-agent/SKILL.md`](../../packages/relay-agent/SKILL.md). Do not assume Nuxt runtime config and relay process config are interchangeable.

## Package manager and native toolchain

- Use **pnpm**; the exact pnpm version is pinned in root `package.json`.
- The native workspace is under `packages/rust-tools/`.
- Repository development pins **Rust 1.95.0**; `Cargo.toml` separately declares MSRV 1.88.0.
- `pnpm build:tools` builds the native binaries used by the local tool/relay packages.
- This repository intentionally has **no CI** and **no unit-test suite**.

See [`project.md`](project.md) and [`../../packages/rust-tools/README.md`](../../packages/rust-tools/README.md) for current verification/release boundaries.

## Mandatory local commit gate

`pnpm install` ends by running [`../../scripts/install-git-hooks.sh`](../../scripts/install-git-hooks.sh). In a Git worktree it:

1. makes [`.githooks/pre-commit`](../../.githooks/pre-commit) executable;
2. configures local `core.hooksPath=.githooks`.

The hook executes:

```sh
pnpm verify:commit
```

That command runs repository policy enforcement, agent-doc/index integrity, `pnpm lint`, and `pnpm typecheck`. Repository policy enforcement rejects tracked CI workflows and conventional unit-test suites. If any gate fails, the commit must not be created. Do not use `git commit --no-verify` or alter `core.hooksPath` to bypass the repository gate.

If a clone/worktree does not have the hook active, run:

```sh
bash scripts/install-git-hooks.sh
```

## Linting

`pnpm lint` is the repository-wide linter gate. It runs:

```sh
eslint .
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

`pnpm lint:fix` applies ESLint fixes and Rust formatting. Clippy findings still require deliberate code fixes.

`@nuxt/eslint` runs in flat-config mode. `eslint.config.mjs` extends the generated `.nuxt/eslint.config.mjs`; project-level Nuxt ESLint options are configured from `nuxt.config.ts`.

## Type-checking

`pnpm typecheck` is the repository-wide type/compile gate. It intentionally uses the strongest locally proven Nuxt path rather than the historically silent bare `nuxt typecheck` wrapper:

```sh
nuxt build --dotenv .env.example
vue-tsc -p .nuxt/tsconfig.json --noEmit
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
```

The production build generates the complete `.nuxt` type project before the explicit Vue check. This is intentionally heavier than `nuxt prepare`: this repository previously observed incomplete generated-project state from prepare-only verification and silent success from bare `nuxt typecheck`.

Do not weaken this gate for commit speed. See [`../memories/007-typecheck-gate-was-silent.md`](../memories/007-typecheck-gate-was-silent.md), [`../memories/013-nuxt-ui-slot-typecheck-gate.md`](../memories/013-nuxt-ui-slot-typecheck-gate.md), and [`../memories/no-ci-local-commit-gates.md`](../memories/no-ci-local-commit-gates.md).

## Dependency/security verification

`pnpm audit` is not run by the pre-commit hook because it depends on registry/network state. It remains mandatory for dependency changes before merge. Security-sensitive Rust changes may additionally require `cargo audit` and the relevant deterministic acceptance/security scripts under `scripts/`.
