# 010 — Workspaces are folders, not just names

> **Status: complete.** Shipped via PR #40, squash-merged to `dev` as `a1347f8`. The first implementation pass (`f0f8056`) had a real, live-exploitable path traversal bug in the traversal guard, caught and fixed (`9ee2c3c`) before merge — see Verification below.

## Context

Plan 007 introduced "workspaces" purely as a named grouping for chats — a `text` name in Postgres, no tie to the filesystem at all. That doesn't match how the tools this app is explicitly modeled after actually work. Confirmed by research, not assumption:

- **opencode / Claude Code / Antigravity CLI** — all single-operator local tools. "Workspace" is whatever directory you launch the CLI from (or pass as an argument); access is bounded by your own OS user permissions, not by the app. There's no multi-user server-side directory picker in any of them, because there's no multi-user concept at all.
- **opencode web** — a local server (`127.0.0.1`, or `0.0.0.0` for LAN with a shared password) that a browser connects to. Still one operator, one filesystem, no per-account scoping.
- **OpenClaw** — the closest real precedent. Each agent has exactly **one workspace directory as its `cwd`**, set via `agents.defaults.workspace` or `agents.entries.*.workspace` — a path **configured by the operator**, not picked live by browsing an unrestricted filesystem.

This app is architecturally different from all four: it's a real multi-tenant web server (Postgres-backed accounts, register/login/OAuth) that happens to run on one person's machine today. None of the reference tools solve "let an authenticated web user browse the server's filesystem" — because none of them need to. The closest legitimate analog is OpenClaw's model: an **operator-configured root**, with each workspace being a real directory *within* that root. That's the design this plan follows — not a free-roam filesystem browser, and not pure invention either.

**The fix**: creating a workspace means picking a real folder (under a configured root) that becomes that workspace's working directory, not typing an arbitrary name.

## Decisions (confirmed with the user)

- Root is a single operator-configured path via env var (`NUXT_WORKSPACES_ROOT`), matching OpenClaw's configured-workspace pattern — not an unrestricted filesystem browser.
- `workspaces.path` becomes required. Existing workspaces (name-only, no path) are backfilled to the root path during migration, then flagged so the UI can prompt the user to confirm/change it — not silently left inconsistent, not a hard blocking gate.

## Changes

1. **Config** — `NUXT_WORKSPACES_ROOT` added to `nuxt.config.ts` runtimeConfig (server-only) and `.env.example`, e.g. defaulting to a sensible dev value. This is the one boundary every path operation below is checked against.

2. **`server/database/schema.ts`** — `workspaces` gains:
   - `path: text('path').notNull()`
   - `pathConfirmed: boolean('path_confirmed').notNull().default(true)` (new workspaces created through the real picker set this `true` immediately; the migration's backfilled rows set it `false` so the UI knows to prompt).
   Generate the migration the same way plan 007's `0002` did (`drizzle-kit generate`, then hand-edit the SQL for the backfill step): add the column, backfill `path = <root>` and `path_confirmed = false` for all existing rows, then apply the `NOT NULL` constraint — same shape as `0002_friendly_layla_miller.sql`'s existing `workspace_id` backfill pattern on `conversations`.

3. **New server utility `server/utils/fs-browse.ts`** — given a relative path segment, resolve it against `NUXT_WORKSPACES_ROOT` with `path.resolve` and verify the result still starts with the root (standard traversal guard — reject `..` escapes). Used by:

4. **New `server/api/fs/browse.get.ts`** — `requireUserSession`-gated, query param `path` (relative to root, defaults to `''`), returns `{ root, path, entries: [{ name, path }] }` listing only subdirectories (via `fs.readdir(..., { withFileTypes: true })`, filtered to `isDirectory()`). No file listing, no reading file contents — directories only, since this is for picking a workspace root, nothing else.

5. **`server/api/workspaces/index.post.ts`** — body gains a required `path`; validate it resolves within the root and exists as a real directory (reuse `fs-browse.ts`'s resolver) before inserting. Sets `pathConfirmed: true`.

6. **New `server/api/workspaces/[id].put.ts`** extension (or the same route, already exists) — allow updating `path` (to support "confirm/change" on a backfilled workspace), same validation, sets `pathConfirmed: true` on success.

7. **UI — replace the name-only modal with a folder picker.** Both `app/components/WorkspacePicker.vue` and `app/layouts/default.vue`'s sidebar currently duplicate near-identical "create workspace" modal logic (`workspaceCreating`/`workspaceName` refs, `confirmCreateWorkspace()` — this was already flagged as a DRY issue independent of this plan). Fold the fix in here: a new **`app/components/WorkspaceFolderPicker.vue`** — a `UModal` with breadcrumb navigation (root → current path segments, each clickable) and a list of subdirectories (`GET /api/fs/browse`) to descend into, a "Select this folder" action, and the workspace name auto-filled from the folder's basename (editable, matches existing UX). Both `WorkspacePicker.vue` and `default.vue` use this one component instead of each having their own modal.

8. **Confirm-path nudge** — where a workspace with `pathConfirmed: false` is shown (workspace switcher dropdown, `WorkspacePicker` grid), a small badge/action opens the same `WorkspaceFolderPicker`, pre-navigated to its current (backfilled root) path, calling the `PUT` from step 6 instead of the `POST` from step 5.

## Out of scope

- No actual tool execution using the workspace path yet — plan 007 already scoped tool-*execution* out; this plan only makes the *data model* correct (workspace really does have a real directory), execution wiring is future work.
- No file browsing/editing UI beyond directory selection — this is a folder picker, not a file explorer.
- No per-user root scoping (`agents.entries.*.workspace`-style per-account roots) — single shared root for now, matches the app's current single-operator reality; revisit if this ever becomes genuinely multi-tenant.

## Verification

- ✅ `pnpm lint`, a real `nuxt build` + `vue-tsc -p .nuxt/tsconfig.json` (not just `nuxt typecheck` — see `.agents/memories/007-typecheck-gate-was-silent.md`), and `pnpm audit` all clean on the merged commit.
- ✅ **Real path traversal vulnerability found and fixed before merge.** The first pass's guard (`resolvedPath.startsWith(absoluteRoot)`) allowed a sibling-directory bypass — live-verified with a freshly registered, zero-privilege user: `GET /api/fs/browse?path=../ai-code-005-p3` returned `200` with a full directory listing of a sibling checkout on the machine, escaping `NUXT_WORKSPACES_ROOT` entirely. Fixed to `resolvedPath !== absoluteRoot && !resolvedPath.startsWith(absoluteRoot + path.sep)`; re-verified live with three separate escape attempts (`../<sibling>`, `../..`, a nested `.agents/../../ai-code-005-p3`) all correctly returning `403`, while normal in-root browsing still works.
- ✅ `WorkspaceFolderPicker.vue` uses this project's semantic color tokens (`border-default`, `bg-elevated`, `text-muted`, `text-dimmed`), not raw Tailwind palette classes — caught in the same pass as the traversal bug.
- ✅ Both `WorkspacePicker.vue` and `default.vue`'s sidebar use the one shared `WorkspaceFolderPicker.vue` instead of each having their own create-workspace modal (this also folds in a DRY fix flagged independently during plan 009's review).
- ✅ `/security-review` run against the merged diff (`f4ac161..a1347f8`) as a fast-follow (the plan required it before merging, but the branch had already been merged before this pass ran — see `.agents/knowledge/git.md` process gap noted below). It found one real bug and one already-accepted tradeoff:
  - **HIGH, fixed as a follow-up commit**: `resolveWorkspacePath()` only did a string-prefix check, never resolving symlinks — `fs.readdir`/`fs.stat` follow symlinks by default, so a symlink under the root pointing outside it (not contrived — pnpm/node_modules link structures and deploy `current ->` conventions make this a realistic accident, not just a deliberate attack) would bypass the boundary entirely even after the sibling-prefix fix. This is exactly what the plan's own verification list asked for ("`fs.readdir` never follows symlinks outside the root") but was never actually tested in the first pass. Fixed by resolving both the root and the candidate path with `fs.realpath()` and re-checking the boundary against the *real* paths, in `resolveWorkspacePath()` itself so all three call sites inherit it. **Live-verified**: created `ln -s /etc evil-link` inside the configured root, confirmed `GET /api/fs/browse?path=evil-link` and `POST /api/workspaces {"path":"evil-link"}` both now return `403` (previously would have followed straight through to `/etc`), while normal in-root browsing still returns real results.
  - **MEDIUM, accepted as-is, not fixed**: `GET /api/fs/browse` scopes access to *any* authenticated user via `requireUserSession`, not to paths the requesting user's own workspaces reference — since the root is shared across all accounts by explicit design (`.agents/memories/010-workspace-configured-root.md`), any registered user can walk the entire root tree and see every other user's workspace folder names. This is the direct, foreseeable consequence of the shared-root decision already made and documented in that memory, not a new gap — surfaced here so it's an explicit, acknowledged tradeoff rather than something nobody noticed. Revisit if this stops being a single-operator deployment.
- Create-workspace-via-picker and the confirm-path nudge on a backfilled workspace were verified by code reading (both wired through `handleSelectFolder`/`handleSelectCreateWorkspace`/`handleSelectConfirmWorkspace` correctly) rather than a live end-to-end click-through — worth a manual pass in a browser before relying on this further.

**Process note**: this plan's own "Verification" section required `/security-review` *before* merging, but the branch was pushed, reviewed, and merged without it — caught only when re-reading `.agents/knowledge/` after the fact. The symlink bug above is a direct consequence: it's exactly the class of issue that review step exists to catch. Run `/security-review` before opening the PR next time, not after merging.

## On completion

- [x] Plan file updated with status and real verification results.
- [x] Moved to the Completed list in `.agents/plans/README.md`.
- [x] `.agents/memories/010-workspace-configured-root.md` written — and indexed in `memories/README.md` (it existed on disk but wasn't indexed until this pass; same gap recurred from plans 008 and 009, worth watching for).
- [x] Branch pushed, PR opened against `dev`, CI green, merged, branch/worktree cleaned up per `.agents/knowledge/git.md`.
