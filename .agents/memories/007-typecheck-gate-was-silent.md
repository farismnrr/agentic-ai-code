# Historical incident: bare `nuxt typecheck` silently passed real type errors

During Plan 007, bare `nuxt typecheck` was observed exiting `0` with no output while a direct generated-project Vue typecheck reported real errors. This was a genuine CI gap and is the reason the repository must not use a plain `"typecheck": "nuxt typecheck"` script as its correctness gate.

## Earlier failed fixes

Two earlier attempts were not kept because they were not reliable on a clean CI checkout at that time:

1. `vue-tsc --noEmit -p .nuxt/tsconfig.json` immediately after `nuxt prepare` failed because the full generated Nuxt project references were not present.
2. `nuxt build --dotenv .env.example && vue-tsc ...` later hit a then-current CI-only Rolldown/config-generation failure even though the same command worked locally, so the feature PR reverted rather than weakening CI around an unexplained environment mismatch.

Those failures are historical context, not a reason to accept the silent gate forever.

## Current invariant

The root `pnpm typecheck` script now deliberately generates the full Nuxt project through a build and then checks that generated project directly:

```sh
nuxt build --dotenv .env.example
vue-tsc -p .nuxt/tsconfig.json --noEmit
```

`.github/workflows/ci.yml` calls `pnpm run typecheck`, so this generated-project check is the repository type gate.

**Do not simplify it back to bare `nuxt typecheck`.** If a future Nuxt/Rolldown upgrade makes the stronger gate fail on a clean checkout, fix the generation/toolchain issue or replace it with an equally explicit generated-project typecheck. Do not silently fall back to the wrapper that previously missed real errors.

See also [`013-nuxt-ui-slot-typecheck-gate.md`](013-nuxt-ui-slot-typecheck-gate.md), which records a concrete class of UI errors this stronger gate is meant to catch.
