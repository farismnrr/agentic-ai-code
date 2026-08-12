# Historical incident: bare `nuxt typecheck` silently passed real type errors

During Plan 007, bare `nuxt typecheck` was observed exiting `0` with no output while a direct generated-project Vue typecheck reported real errors. This was a genuine verification gap and is the reason the repository must not use a plain `"typecheck": "nuxt typecheck"` script as its correctness gate.

## Earlier failed fixes

Two earlier attempts were not reliable in the repository state/toolchain that existed at the time:

1. `vue-tsc --noEmit -p .nuxt/tsconfig.json` immediately after an older `nuxt prepare` output did not have the generated project shape needed by that older setup.
2. `nuxt build --dotenv .env.example && vue-tsc ...` proved to be the wrong coupling for the type gate. On a clean GitHub Actions checkout with Nuxt 4.5.1/Vite 8, the build path could enter Vite transformation while `.nuxt/tsconfig.app.json` was unavailable, producing dozens of `TSCONFIG_ERROR` failures before `vue-tsc` even ran.

Those failures are historical context, not a reason to accept the silent gate forever.

## Current invariant

With the current Nuxt toolchain, the repository uses Nuxt's dedicated type-generation command followed by a direct Vue TypeScript check:

```sh
nuxt prepare --dotenv .env.example
vue-tsc -p .nuxt/tsconfig.json --noEmit
```

The root `pnpm typecheck` script owns this sequence and then performs the warnings-denied Rust workspace check. The mandatory local pre-commit gate calls `pnpm typecheck`; the repository intentionally has no CI workflow now.

`nuxt prepare` is used here specifically to generate Nuxt's type project without coupling type verification to production bundling. `pnpm build` remains a separate runtime/bundling verification command when needed.

**Do not simplify this back to bare `nuxt typecheck`.** If a future Nuxt/Vue toolchain changes the generated project shape, fix or replace the explicit generation + `vue-tsc` sequence with an equally strong check. Do not silently fall back to the wrapper that previously missed real errors.

See also [`013-nuxt-ui-slot-typecheck-gate.md`](013-nuxt-ui-slot-typecheck-gate.md) and [`no-ci-local-commit-gates.md`](no-ci-local-commit-gates.md).
