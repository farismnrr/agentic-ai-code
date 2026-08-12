# Tooling

## Environment and runtime config

Copy [`.env.example`](../../.env.example) → `.env` (gitignored) on a fresh clone. **`.env.example` is the environment-key inventory/source of truth**; keep it aligned with `nuxt.config.ts`/runtime consumers when configuration changes instead of maintaining a second exhaustive key list here.

Current configuration groups include dev server/public site URL, router/model-provider credentials, workspace root, PostgreSQL, session sealing, SMTP/optional OAuth providers, and OpenTelemetry/Jaeger/Loki.

Not every key is required for every workflow. Fill the values needed by the subsystem you are running; never commit secrets or real credentials to Markdown, plans, memories, fixtures, or examples.

### Stable conventions

- `NUXT_PORT` — dev port. Defaults to **3333** via `devServer.port` in `nuxt.config.ts`.
- `NUXT_HOST` — leave unset for the safe localhost-only default. When intentionally exposing dev to another device, bind to a specific trusted interface rather than `0.0.0.0`.
- `NUXT_PUBLIC_SITE_URL` — public runtime config; browser-visible by definition.
- `NUXT_WORKSPACES_ROOT` — operator-owned workspace filesystem boundary for the Nuxt application. Do not silently fall back to unrestricted filesystem browsing.

Nuxt runtime config binding is by convention: `NUXT_FOO_BAR` → `runtimeConfig.fooBar`, `NUXT_PUBLIC_FOO` → `runtimeConfig.public.foo`. Prefer `useRuntimeConfig()`/Nuxt config surfaces in application code instead of ad-hoc `process.env` reads.

The Rust `relay-agent` has its own CLI/environment contract under [`../../packages/relay-agent/SKILL.md`](../../packages/relay-agent/SKILL.md). Do not assume Nuxt runtime config and relay process config are interchangeable.

## Package manager and native toolchain

- Use **pnpm**; the exact pnpm version is pinned in root `package.json`.
- The native workspace is under `packages/rust-tools/`.
- Repository development pins **Rust 1.95.0**; `Cargo.toml` separately declares MSRV 1.88.0.
- `pnpm build:tools` builds the native binaries used by local tool/relay packages.
- This repository intentionally has **no CI** and **no unit-test suite**.

## Mandatory local commit gate

`pnpm install` runs [`../../scripts/install-git-hooks.sh`](../../scripts/install-git-hooks.sh), which makes [`.githooks/pre-commit`](../../.githooks/pre-commit) executable and configures local `core.hooksPath=.githooks`.

The hook executes:

```sh
pnpm verify:commit
```

That command runs repository policy enforcement, agent-doc integrity, `pnpm lint`, and `pnpm typecheck`. If any gate fails, the commit must not be created. Do not use `git commit --no-verify` or alter `core.hooksPath` to bypass it.

## Linting

`pnpm lint` is the repository-wide linter gate:

```sh
eslint .
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

`pnpm lint:fix` applies ESLint fixes and Rust formatting. Clippy findings still require deliberate code fixes.

## Type-checking

`pnpm typecheck` is the repository-wide type/compile gate:

```sh
nuxt prepare --dotenv .env.example
vue-tsc -p .nuxt/tsconfig.json --noEmit
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
```

The dedicated Nuxt prepare step generates the type project without coupling type verification to production bundling. Keep `pnpm build` as a separate runtime/bundling verification command when needed.

Do not simplify the type gate back to plain `nuxt typecheck`: this repository previously observed that wrapper exit successfully while real generated-project errors remained. The rationale and related Nuxt UI slot trap are preserved in the canonical [`../memories/README.md`](../memories/README.md#repository-policy-and-verification).

## Dependency/security verification

`pnpm audit` is not run by the pre-commit hook because it depends on registry/network state. It remains mandatory for dependency changes before merge. Security-sensitive Rust changes may additionally require `cargo audit` and relevant deterministic acceptance/security scripts under `scripts/`.
