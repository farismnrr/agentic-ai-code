---
name: 014-scripts-dir-not-gated
description: standalone files under scripts/ aren't covered by pnpm build, vue-tsc, or eslint — a broken one-off script only surfaces by actually running it
metadata:
  type: feedback
---

A one-off script under `scripts/` is outside every standard verification gate. `pnpm build` doesn't touch it (it's not part of the Nuxt app graph), `vue-tsc -p .nuxt/tsconfig.json --noEmit` doesn't include it (see [[007-typecheck-gate-was-silent]] for that project's actual scope), and `eslint .` checks syntax/style but not whether imports resolve at runtime.

**Why this matters:** plan [[014-reasoning-effort-and-model-cleanup]]'s data-cleanup script (`scripts/backfill-models.ts`) shipped importing `'dotenv/config'` with `dotenv` present only as a *transitive* dependency in the lockfile, never declared in `package.json`. Every standard gate (build, typecheck, lint) passed clean. The break — `ERR_MODULE_NOT_FOUND` before the script ever touched the database — only showed up when it was actually executed with `npx tsx scripts/<file>.ts` in review.

**How to apply:** a `scripts/*.ts` file (or anything else run standalone via `tsx`/`node` rather than through Nuxt/Nitro) needs to actually be **run**, not just built/linted/typechecked, before it's considered verified. `pnpm build && vue-tsc ... && eslint .` all passing is not evidence a script in `scripts/` works.
