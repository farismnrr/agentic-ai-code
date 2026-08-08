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

**A running `pnpm preview` does not pick up a new build on its own.** It loads `.output` once at process start and keeps serving that snapshot — it is not a file watcher. Rebuilding (`pnpm build`) while an old `preview` process is still bound to the port produces a **new** `.output` with different content-hashed asset filenames (`_nuxt/entry.<hash>.css`, `.js`), but the old process keeps serving the **old** `index.html`, which references those now-deleted filenames. The browser then 404s on every CSS/JS asset, and the page renders as unstyled/broken HTML — which looks exactly like "the UI is broken" when the code is actually fine. Hit for real reviewing plan 013's motion pass: the animations were correct in the new build, but the still-running old `preview` process made it look like nothing had shipped.

**Always kill the old `preview` process (`lsof -ti:3333 | xargs -r kill -9`, or whatever port is configured — see [[port-3333]]) before every fresh `pnpm build`, then start a new `pnpm preview`.** Don't just re-run `pnpm build` and assume an already-running preview will reflect it — check `lsof -i:3333` (or curl a known asset URL) if a rebuild doesn't seem to have changed anything in the browser.
