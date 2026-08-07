# Tooling

## Environment

Copy `.env.example` → `.env` (gitignored) on a fresh clone.

- `NUXT_PORT` — dev port. Defaults to **3333** via `devServer.port` in `nuxt.config.ts`; 3000/3001/3004/3100 are occupied on this machine.
- `NUXT_PUBLIC_SITE_URL` — maps to `runtimeConfig.public.siteUrl`, readable in client and server code via `useRuntimeConfig().public.siteUrl`.

Runtime config binding is by convention: `NUXT_FOO_BAR` → `runtimeConfig.fooBar`, `NUXT_PUBLIC_FOO` → `runtimeConfig.public.foo`. A key **must** exist in `nuxt.config.ts` for the env var to be picked up — env vars alone do nothing. Never read `process.env` in app code; use `useRuntimeConfig()`.

## Linting

`@nuxt/eslint` in flat-config mode. `eslint.config.mjs` extends the generated `.nuxt/eslint.config.mjs`; **rules are configured in `nuxt.config.ts` under `eslint.config`, not in `eslint.config.mjs`**. Enabled:

- **stylistic** — formatting as lint rules. No Prettier for JS/TS/Vue; don't add one, the two will fight. House style: no trailing commas, 1TBS braces, plus enforced `nuxt.config.ts` key ordering.
- **typescript: strict** — the typescript-eslint strict preset. Type-aware rules are deliberately off: Nuxt 4's root `tsconfig.json` is references-only (`files: []`), so there's no single project for the type checker to resolve. `pnpm typecheck` (vue-tsc) covers types instead.
- **formatters** — `eslint-plugin-format` handles CSS, JSON, and Markdown, which stylistic doesn't reach.
- **checker: true** — lint errors surface in the dev server output and browser overlay, via `vite-plugin-eslint2`.

Add rule overrides with the chainable API in `eslint.config.mjs`:

```js
export default withNuxt()
  .override('nuxt/vue/rules', { rules: { 'vue/multi-word-component-names': 'off' } })
```

Config names are discoverable in the ESLint config inspector (Nuxt DevTools → ESLint).
