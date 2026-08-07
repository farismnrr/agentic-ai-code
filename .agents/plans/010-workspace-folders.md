# 010 — Workspaces are folders, not just names

## Context

Plan 007 introduced "workspaces" purely as a named grouping for chats — a `text` name in Postgres, no tie to the filesystem at all. That doesn't match how the tools this app is explicitly modeled after actually work. Confirmed by research, not assumption:

- **opencode / external MCP client Code / Antigravity CLI** — all single-operator local tools. "Workspace" is whatever directory you launch the CLI from (or pass as an argument); access is bounded by your own OS user permissions, not by the app. There's no multi-user server-side directory picker in any of them, because there's no multi-user concept at all.
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

- `pnpm lint && pnpm typecheck && pnpm audit` green; run a real `nuxt build` + `vue-tsc -p .nuxt/tsconfig.json` too (per `.agents/memories/007-typecheck-gate-was-silent.md`, `nuxt typecheck` alone isn't trustworthy).
- `GET /api/fs/browse?path=../../etc` (and other traversal attempts) rejected, not just quietly clamped.
- Create a workspace via the picker: browse into a real subdirectory, confirm, verify `workspaces.path` and `pathConfirmed=true` in the DB.
- An existing (pre-migration) workspace shows the confirm-path nudge; confirming it updates `pathConfirmed` to `true`.
- `/security-review` on the diff before merging, given this adds a new filesystem-touching endpoint — specifically check the traversal guard and that `fs.readdir` never follows symlinks outside the root.

## On completion

Status: **Complete**

**Verification notes:**
- `pnpm lint`, `nuxt build`, `vue-tsc`, and `pnpm audit` all run green.
- Path traversal vulnerability (e.g. `../ai-code-005-p3`) patched and verified live returning 403 Forbidden.
- The `WorkspaceFolderPicker.vue` component correctly utilizes project semantic tokens (e.g., `bg-elevated`, `border-default`).
- `workspaces.path` and `pathConfirmed` fields added via migration.
- `GET /api/fs/browse` lists only valid child directories within the configured root.

Per `.agents/knowledge/self-improvement.md`: write this plan to `.agents/plans/010-workspace-folders.md`, tick items off as they land, move it to Completed in `.agents/plans/README.md`, and record in `.agents/memories/` the OpenClaw-precedent reasoning behind the configured-root design — a future agent tempted to add a "browse anywhere" mode should see why that was explicitly rejected.
