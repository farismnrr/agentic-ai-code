# 016 — Group the sidebar by workspace, show it in the chat header

## Why

The active workspace is currently only visible as a small dropdown button
at the very top of the sidebar (`app/layouts/default.vue`'s
`#header` slot) — easy to miss, and the conversation list below it only
ever shows the *one* active workspace's chats (scoped via
`?workspaceId=` on `GET /api/conversations`), grouped by recency
(Today/Yesterday/…), not by workspace. Switching workspaces means the
whole list swaps out silently.

User wants an Antigravity-style structure: workspace is the outer
grouping, chats are nested under it — and a visible indicator of which
workspace a given conversation belongs to when actually reading it, not
just in the sidebar.

## Phase 1 — Backend: workspace ⇒ many chats API

The data model already supports this (`conversations.workspaceId` is a
required FK to `workspaces`) — this phase is about making the *API
surface* explicit and clean, not changing the schema.

1. `server/api/conversations/index.get.ts` currently requires
   `workspaceId` (`throw badRequest('Missing workspaceId')` if absent).
   Relax this: when `workspaceId` is omitted, return the user's
   conversations across **all** their workspaces (still scoped to
   `session.user.id` — never cross-user). Keep the `workspaceId`-scoped
   query path working too, since other call sites may still want it.
   This is the one-to-many read path the frontend groups by: fetch
   workspaces (`GET /api/workspaces`) and all conversations
   (`GET /api/conversations`, no `workspaceId`) as two flat lists, and
   group client-side by `conversation.workspaceId` — no nested-response
   endpoint needed, the FK is enough.
2. `app/composables/useConversations.ts`'s `loadAll()` currently hard-requires
   `activeWorkspaceId` and returns `[]` without one. Change it to always
   fetch *all* workspaces' conversations for the sidebar. Decide whether
   "active workspace" still means anything after this change — likely
   yes, but scoped down to just "which workspace `+ New chat` creates
   into" and "which workspace's picker state gates the empty `/chat`
   state," not "which workspace's chats are visible."

## Phase 2 — Sidebar: workspace-grouped structure

1. Restructure `app/layouts/default.vue`'s sidebar `#default` slot:
   outer loop over the user's workspaces (each as a collapsible section,
   icon + name — reuse `workspaceItems`' data), inner loop over that
   workspace's conversations (existing recency grouping can stay as a
   second-level grouping inside each workspace section, or be dropped in
   favor of a flat recent-first list per workspace — confirm which reads
   better once it's on screen, don't over-engineer this up front).
2. Keep `+ New chat` targeting the current "active" workspace (per Phase
   1's redefinition) — but now that all workspaces are visible in the
   sidebar at once, consider whether clicking into a specific workspace's
   section should also change which one `+ New chat` targets, so the
   two stay consistent from the user's point of view.
3. The existing top dropdown (`workspaceItems`) may become redundant if
   the sidebar itself now shows every workspace — evaluate removing it in
   favor of clicking a workspace's own section header to make it active,
   rather than keeping both a dropdown and an inline list as separate
   ways to do the same thing.

## Phase 3 — Workspace indicator in the chat view

1. `app/pages/chat/[id].vue`'s `UDashboardNavbar` currently only shows the
   conversation title. Add the owning workspace's name (and maybe icon)
   next to or below it — `conversation.value.workspaceId` is already on
   the `Conversation` object, resolve it against the loaded `workspaces`
   list (`useWorkspaces().get(id)`).
2. Keep it subtle (a small badge/label, not a second heading) — the
   title is still the primary thing being read.

## Phase 4 — Show the full folder path via a "…" detail, not the name

The workspace's display name stays exactly what it is today — a short
folder name (e.g. `BNSP`) — never the full path. `workspaces.path` is
already stored (plan 010) and already returned by `GET /api/workspaces`,
just never surfaced anywhere in the UI. Add a way to see it without
changing what's shown by default:

1. In the sidebar's per-workspace `UDropdownMenu` (`workspaceActionItems()`
   in `app/layouts/default.vue`, opened via the "…" button next to each
   workspace's group header), add a **"View details"** (or "Show full
   path") item. Clicking it should surface the full `path` — a
   `UTooltip`/`UPopover` anchored to the item, a small `UModal`, or an
   inline expand-in-place row; pick whichever is least disruptive to the
   existing dropdown, don't build a whole new settings surface for one
   string.
2. Do the same for the workspace picker's own per-workspace card
   (`WorkspacePicker.vue`) if it has an equivalent options affordance, so
   the two workspace UIs (picker + sidebar) are consistent about how you
   see the full path.
3. This is read-only — renaming/re-pointing the folder already goes
   through the existing "Confirm Folder"/rename flow
   (`WorkspaceFolderPicker`, `initial-path`), this phase doesn't touch
   that.

## Known bugs to fix in this same effort (found reviewing the in-progress branch)

- **The chat header's title (and this plan's own new workspace badge)
  never render.** `app/pages/chat/[id].vue` passes both `#left`
  (overridden with just `<UDashboardSidebarCollapse />`) and `#title` to
  `UDashboardNavbar` — but per the component's own source
  (`node_modules/@nuxt/ui/.../DashboardNavbar.vue`), the `<h1><slot
  name="title">` that actually renders `#title`'s content only exists
  inside `<slot name="left">`'s *default* (fallback) content. Overriding
  `#left` explicitly discards that fallback entirely, so `#title` is
  never mounted anywhere — confirmed via `DOM.querySelector('h1[data-slot="title"]')`
  returning nothing live in a running instance of this branch. This
  predates this plan (the old code used the `title` prop with the same
  `#left` override, which has the identical problem — the chat title has
  likely never visibly rendered), but the new workspace badge inherits
  it too, so it must be fixed here rather than shipped invisible. Fix by
  either dropping the custom `#left` override (let the component's
  default `#left` content render, passing `title`/leading/trailing
  through their intended props/slots instead of `#left`), or by moving
  the sidebar-collapse button *and* the title+badge into a single custom
  `#left` slot together, since `#left` is all-or-nothing.
- **Hardcoded Tailwind colors.** The workspace-badge markup uses
  `text-gray-900 dark:text-white` — this project's convention
  (`.agents/knowledge/conventions.md`) requires semantic classes
  (`text-default`/`text-highlighted`/etc.) so dark mode and theming keep
  working; raw palette classes bypass both. Fix to use the same semantic
  class the original title text used before this branch touched it.

## Verification

- Live test: a user with 2+ workspaces, each with several chats, sees
  them grouped correctly in the sidebar without needing to switch the
  active workspace to see the other one's chats.
- Confirm `+ New chat` still creates into a sensible/expected workspace,
  and that the chat-before-workspace guarantee (plan 009) isn't
  regressed — still impossible to create a conversation with no
  workspace.
- Confirm the chat header workspace label **actually renders** (not just
  exists in the template — verify via a real screenshot/DOM check, per
  the bug above) and matches the conversation's real `workspaceId`,
  including for a deep-linked conversation opened directly.
- Confirm the workspace name shown everywhere stays the short folder
  name, and the full path is only visible after an explicit "…" action —
  never shown by default.
- `pnpm build && vue-tsc -p .nuxt/tsconfig.json --noEmit && pnpm run lint && pnpm audit` clean.

## Out of scope

- Not adding cross-workspace search/filtering beyond what already exists.
- Not changing the URL structure (still no `/workspace/:id/chat/:id` —
  per [[007-workspace-client-routing]], unless this phase reveals a real
  need to revisit that, which should be flagged, not silently done.

## On completion

- [x] Phase 1 — `GET /api/conversations` now returns all of a user's
      conversations across every workspace when `workspaceId` is
      omitted, still scoped to `session.user.id`; `useConversations().loadAll()`
      always fetches the full set for the sidebar.
- [x] Phase 2 — sidebar restructured to group by workspace (icon+name
      section, chats nested underneath) instead of a single
      active-workspace-scoped recency list; the old top dropdown was
      replaced with a "Workspaces" header + inline per-workspace
      sections, each clickable to make it active.
- [x] Phase 3 — chat header shows the owning workspace as a subtle badge
      next to the conversation title.
- [x] Phase 4 — workspace name stays the short folder name everywhere;
      the real full path is only visible via an explicit "View details"
      action, implemented consistently in both places a workspace can be
      picked from (`WorkspacePicker.vue`'s card and the sidebar's
      per-workspace "…" dropdown in `layouts/default.vue`), both opening
      the same read-only modal.
- [x] Both bugs found reviewing the in-progress branch fixed and
      verified: the `#left`/`#title` slot conflict in `UDashboardNavbar`
      (title and the new workspace badge now render, moved into a single
      `#left` slot alongside the sidebar-collapse button) and the
      hardcoded `text-gray-900 dark:text-white` (now `text-default`).
- [x] `pnpm build && vue-tsc -p .nuxt/tsconfig.json --noEmit && pnpm run
      lint && pnpm audit` all clean.
- [x] Merged to `dev` via PR (see plans/README.md), branch and worktree
      cleaned up per `.agents/knowledge/git.md`.
