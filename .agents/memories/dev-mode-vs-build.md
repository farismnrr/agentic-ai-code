---
name: dev-mode-vs-build
description: Local verification uses `pnpm build && pnpm preview`, not `pnpm dev` — the user asked for this after `pnpm dev`'s file-watcher threw spurious ENOTDIR errors during a session with heavy branch-switching.
metadata:
  type: feedback
---

Use `pnpm build && pnpm preview` for local app verification, not `pnpm dev`.

**Why:** during plan [[012-mcp-api-key]], a `pnpm dev` instance left running across several `git checkout`/branch-switch/worktree-remove operations started throwing `[nitro] ERROR: ENOTDIR: not a directory, stat '.../router-model.ts/package.json'` and `virtual:#imports could not be resolved` — Vite/Nitro's dev-mode file watcher got confused by the filesystem changing out from under it. The files were fine; a fresh `pnpm build` proved it. The user also independently flagged `pnpm dev`'s startup/compile latency as "kelamaan buat loadnya" (too slow to load) and asked to switch the default local-run flow to the built app.

**How to apply:**
- After every merge to `dev` (or any time you need to verify the app runs), do `rm -rf .nuxt .output && pnpm build && pnpm preview`, not `pnpm dev`.
- `pnpm preview` serves the real `.output` — same artifact CI/production would run — and starts instantly since nothing compiles per-request, unlike `pnpm dev`'s on-demand compilation.
- Kill any previously-running `pnpm dev`/`pnpm preview` process before starting a new one if you're about to do branch/worktree operations — don't leave a dev server running across git surgery.
- `pnpm dev` is still fine for a tight single-file HMR iteration loop, just not as the default "let me check this works" command.
