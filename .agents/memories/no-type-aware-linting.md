# Type-aware ESLint rules are off on purpose

`@nuxt/eslint` supports type-aware linting via `eslint.config.typescript.tsconfigPath`, but it can't work here: Nuxt 4's root `tsconfig.json` is references-only (`"files": []`, pointing at the four generated `.nuxt/tsconfig.*.json` projects). typescript-eslint needs a single project that actually includes the source files, so enabling it reports "file is not included in any project" for everything.

Type safety is covered by `pnpm typecheck` (vue-tsc) instead. Don't "fix" this by pointing `tsconfigPath` at one of the `.nuxt/` files — those are regenerated on every `nuxt prepare` and only cover a subset of the codebase each.
