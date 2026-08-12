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
- Repository development/CI pins **Rust 1.95.0**; `Cargo.toml` separately declares MSRV 1.88.0.
- `pnpm build:tools` builds the native binaries used by the local tool/relay packages.

See [`project.md`](project.md) and [`../../packages/rust-tools/README.md`](../../packages/rust-tools/README.md) for current verification/release boundaries.

## Linting

`@nuxt/eslint` runs in flat-config mode. `eslint.config.mjs` extends the generated `.nuxt/eslint.config.mjs`; project-level Nuxt ESLint options are configured from `nuxt.config.ts`, while targeted overrides can use the chainable `withNuxt()` API in `eslint.config.mjs`.

Current conventions:

- **Stylistic linting** owns JS/TS/Vue formatting. Do not add Prettier for those files unless the project deliberately changes formatting ownership.
- **typescript-eslint strict rules** are enabled without type-aware linting. Nuxt 4's root `tsconfig.json` is references-oriented, so there is no single root program for type-aware ESLint to consume cleanly.
- **Formatters** cover formats such as CSS/JSON/Markdown through the Nuxt ESLint setup.
- **Checker integration** surfaces lint errors during development.

Example targeted override:

```js
export default withNuxt()
  .override('nuxt/vue/rules', {
    rules: {
      'vue/multi-word-component-names': 'off'
    }
  })
```

Config names are discoverable through the generated Nuxt ESLint configuration/inspector; verify against the installed version instead of copying names from an old plan.

## Type-checking caveat

Do not treat a green `pnpm typecheck` as the only compile proof for Vue/Nuxt changes. This repository has recorded cases where Nuxt typecheck returned success while the generated Vue project still contained errors.

For changes where type correctness matters, use the stronger generated-project gate after building:

```sh
pnpm build
pnpm exec vue-tsc -p .nuxt/tsconfig.json --noEmit
```

See [`../memories/007-typecheck-gate-was-silent.md`](../memories/007-typecheck-gate-was-silent.md) and [`../memories/013-nuxt-ui-slot-typecheck-gate.md`](../memories/013-nuxt-ui-slot-typecheck-gate.md).
