# `nuxt typecheck` silently passed with real type errors present

`package.json`'s `typecheck` script used to be `nuxt typecheck`. Confirmed
(by deliberately reintroducing two real bugs and rerunning) that both `nuxt
typecheck` and standalone `nuxi typecheck` exit `0` with no output on this
Nuxt 4.5.1 / vue-tsc 3.3.9 combo, even with a fully-populated `.nuxt/`. It's
not a missing-file problem — the checker step itself never surfaces errors
here.

Calling `vue-tsc --noEmit -p .nuxt/tsconfig.json` directly does catch real
errors, **but only if `.nuxt/tsconfig.json` exists** — and `nuxt prepare`
(what `postinstall` runs on `pnpm install`) only generates
`.nuxt/tsconfig.server.json`, not the full `tsconfig.app.json` /
`tsconfig.shared.json` / `tsconfig.node.json` / `tsconfig.json` set that the
root `tsconfig.json`'s project references need. Those are only written by a
real `nuxt build` (confirmed: ~50s). A fresh CI checkout that just ran
`pnpm install` therefore has no `.nuxt/tsconfig.json`, and a bare `vue-tsc
-p .nuxt/tsconfig.json` fails with `TS5058: The specified path does not
exist` — this broke CI on PR #36 before being caught.

Fixed: `"typecheck": "nuxt build --dotenv .env.example && vue-tsc --noEmit
-p .nuxt/tsconfig.json"`. Costs a full build every typecheck run (~50s) but
is the only combination verified to actually catch errors on a clean
checkout.

**Why this matters:** `pnpm typecheck` gates every PR per
`.agents/knowledge/project.md`. Re-verify by deliberately introducing a type
error on a *clean* checkout (`rm -rf .nuxt .output`) and confirming the
script's exit code is non-zero before trusting any future change to it.
