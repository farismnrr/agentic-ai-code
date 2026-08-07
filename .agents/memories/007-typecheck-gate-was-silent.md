# `nuxt typecheck` silently passed with real type errors present

`package.json`'s `typecheck` script used to be `nuxt typecheck`. During plan
007 it exited `0` and printed nothing useful while `vue-tsc --noEmit -p
.nuxt/tsconfig.json` (the same underlying checker) found several real
errors on the same code — including a call passing an object where a
`string` param was declared. CI and local runs both trusted the green exit
code.

Fixed by pointing the script directly at `vue-tsc --noEmit -p
.nuxt/tsconfig.json`, which now fails correctly.

**Why this matters:** `pnpm typecheck` gates every PR per
`.agents/knowledge/project.md`. If it silently no-ops again after a Nuxt/
`vue-tsc` version bump, type errors will reach `dev` unnoticed. Re-verify by
deliberately introducing a type error and confirming the script's exit code
is non-zero before trusting it again.
