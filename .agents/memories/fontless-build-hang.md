---
name: fontless-build-hang
description: pnpm build finishes and writes .output/ correctly but the CLI process never exits — fontless (a transitive dependency of @nuxt/ui via @nuxt/fonts) leaks an esbuild service.
metadata:
  type: project
---

`pnpm build` can hang indefinitely *after* a fully successful build — `.output/server/index.mjs` and `.output/public/*` are written correctly, but the `nuxt build` process itself never exits on its own.

**Why:** `fontless` (pulled in transitively by `@nuxt/fonts`, which `@nuxt/ui` depends on) spawns an esbuild service process that never shuts itself down. This is a confirmed upstream bug, not something in this repo's code — reproduces on a clean `dev` checkout with zero application changes. Tracked at [nuxt/nuxt#33987](https://github.com/nuxt/nuxt/issues/33987), still unresolved upstream as of this writing.

**How to apply:** `nuxt.config.ts` has a `hooks.close` that calls `process.exit(0)` to force the process to terminate once Nuxt's own build lifecycle says it's done. If this repo's `pnpm build` ever seems to hang again, check `ps` for a lingering `esbuild` process before assuming it's a real regression — `ps -o pid,etimes,time,pcpu -p <pid>` with `TIME` frozen while `ELAPSED` keeps growing is the tell (the actual build work is done; nothing is computing anymore). Don't remove the `hooks.close` workaround without re-testing a full `pnpm build` to completion first.

See also [[background-command-output]] for how to tell a genuinely stuck process from one that's just slow/buffered.
