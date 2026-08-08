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

## Phase 1 — Fetch conversations across all workspaces

1. `server/api/conversations/index.get.ts` currently requires
   `workspaceId` (`throw badRequest('Missing workspaceId')` if absent).
   Relax this: when `workspaceId` is omitted, return the user's
   conversations across **all** their workspaces (still scoped to
   `session.user.id` — never cross-user). Keep the `workspaceId`-scoped
   query path working too, since other call sites may still want it.
2. `app/composables/useConversations.ts`'s `loadAll()` currently hard-requires
   `activeWorkspaceId` and returns `[]` without one. Add a mode (or a
   second function) that fetches *all* workspaces' conversations for the
   sidebar, independent of which one is "active." Decide whether "active
   workspace" still means anything after this change — likely yes, but
   scoped down to just "which workspace `+ New chat` creates into" and
   "which workspace's picker state gates the empty `/chat` state," not
   "which workspace's chats are visible."

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

## Verification

- Live test: a user with 2+ workspaces, each with several chats, sees
  them grouped correctly in the sidebar without needing to switch the
  active workspace to see the other one's chats.
- Confirm `+ New chat` still creates into a sensible/expected workspace,
  and that the chat-before-workspace guarantee (plan 009) isn't
  regressed — still impossible to create a conversation with no
  workspace.
- Confirm the chat header workspace label matches the conversation's
  real `workspaceId`, including for a deep-linked conversation opened
  directly (not via the sidebar).
- `pnpm build && vue-tsc -p .nuxt/tsconfig.json --noEmit && pnpm run lint && pnpm audit` clean.

## Out of scope

- Not adding cross-workspace search/filtering beyond what already exists.
- Not changing the URL structure (still no `/workspace/:id/chat/:id` —
  per [[007-workspace-client-routing]], unless this phase reveals a real
  need to revisit that, which should be flagged, not silently done.
