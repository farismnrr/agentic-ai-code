# `nuxt typecheck` doesn't reliably surface real type errors — known, not yet fixed

During plan 007, `nuxt typecheck` was observed exiting `0` with no output
on code that had real type errors present (confirmed once by comparing
against a direct `vue-tsc --noEmit -p .nuxt/tsconfig.json` run, which did
report them). This is a genuine gap in the CI gate, but **two fix attempts
in this PR both made CI worse, not better**, so the script was reverted to
plain `"typecheck": "nuxt typecheck"` (the pre-plan-007 baseline) rather
than ship something unproven:

1. `vue-tsc --noEmit -p .nuxt/tsconfig.json` directly — fails outright on a
   clean checkout: `nuxt prepare` (what `postinstall` runs) only writes
   `.nuxt/tsconfig.server.json`, not the full `tsconfig.app.json` /
   `tsconfig.shared.json` / `tsconfig.node.json` / `tsconfig.json` set the
   root `tsconfig.json`'s project references need — `TS5058: The specified
   path does not exist`.
2. `nuxt build --dotenv .env.example && vue-tsc ...` — generates the full
   tsconfig set (confirmed locally), but failed in CI with
   `TSCONFIG_ERROR: Tsconfig not found .nuxt/tsconfig.app.json` from a
   `rolldown@1.1.5` stack frame, while `pnpm-lock.yaml` pins
   `rolldown@1.2.1` everywhere and the same build succeeded locally every
   time. `rolldown` is resolved outside the deterministic lockfile graph
   (likely via `vite`'s optional-bundler resolution), so **the exact
   version a clean `pnpm install` picks up is not reproducible between this
   machine and GitHub Actions** — not something to chase further inside a
   feature PR.

**Why this matters:** `pnpm typecheck` gates every PR per
`.agents/knowledge/project.md`, and it is currently back to a state that
may silently pass real type errors — this is a known, open gap, not a
resolved one. Before trusting it again (or attempting another fix), verify
on a genuinely clean checkout in CI itself, not just locally — local
success here did not predict CI's behavior twice in a row.
