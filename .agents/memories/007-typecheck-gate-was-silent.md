# Historical incident: bare `nuxt typecheck` silently passed real type errors

During Plan 007, bare `nuxt typecheck` was observed exiting `0` with no output while a direct generated-project Vue typecheck reported real errors. This was a genuine CI gap and is the reason the repository must not use a plain `"typecheck": "nuxt typecheck"` script as its correctness gate.

## Earlier failed fixes

Two earlier attempts were not reliable in the repository state/toolchain that existed at the time:

1. `vue-tsc --noEmit -p .nuxt/tsconfig.json` immediately after the then-current `nuxt prepare` output did not have the generated project shape needed by that older setup.
2. `nuxt build --dotenv .env.example && vue-tsc ...` later proved to be the wrong coupling for the type gate. On a clean CI checkout with Nuxt 4.5.1/Vite 8, the build path could enter Vite transformation while `.nuxt/tsconfig.app.json` was unavailable, producing dozens of `TSCONFIG_ERROR` failures before `vue-tsc` even ran.

Those failures are historical context, not a reason to accept the silent gate forever.

## Current invariant

With the current Nuxt toolchain, the repository uses Nuxt's dedicated type-generation command followed by a direct Vue TypeScript check:

```sh
nuxt prepare --dotenv .env.example
vue-tsc -p .nuxt/tsconfig.json --noEmit
```

The root `pnpm typecheck` script owns this sequence, and `.github/workflows/ci.yml` calls that script. `nuxt prepare` is specifically the Nuxt command for creating `.nuxt` and generating types in CI/postinstall workflows; bundling is intentionally kept separate as `pnpm build`.

**Do not simplify this back to bare `nuxt typecheck`.** If a future Nuxt/Vue toolchain changes the generated project shape, fix or replace the explicit generation + `vue-tsc` sequence with an equally strong check. Do not silently fall back to the wrapper that previously missed real errors.

See also [`013-nuxt-ui-slot-typecheck-gate.md`](013-nuxt-ui-slot-typecheck-gate.md), which records a concrete class of UI errors this stronger gate is meant to catch.
