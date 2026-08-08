# 017 — Explicit workspace targeting when starting a chat

## Why

User feedback tagged live via agentation on `/chat` (annotations quoted
below, translated) — flagged as critical, all pointing at the same root
gap: plan 016 grouped the sidebar by workspace and showed the active one,
but there is still **no explicit way to choose which workspace a new
chat goes into** at the moment you're actually starting it. Today,
`create()` always uses whatever `activeWorkspaceId` happens to be
(`app/composables/useConversations.ts`), set implicitly by whichever
workspace you last clicked in the sidebar — the chat prompt form itself
has no control for it, and the sidebar's `+ New chat` / per-workspace
"…" menu don't let you target a specific workspace on the spot either.

**Annotation #2** — `app/pages/chat/index.vue`'s `UChatPrompt` form:
> "di bagian ini kenapa ga ada pilihan workspace? harusnya ada dong? biar
> ketika gw chat lgsung masuk ke workspace" — the model/effort `USelect`s
> already live in the form's `#footer` (lines ~115-129); there is no
> equivalent for workspace.

**Annotation #3** — the sidebar's per-workspace "…" dropdown trigger
(`workspaceActionItems()` in `app/layouts/default.vue`):
> "dibagian ini harusnya ada add conversations, jadi pas di klik dia
> ngarahin ke chat, cuma sama kyk sebelumnya dia ngarahinnya lgsung ke
> workspace yg di tag" — wants a "New chat" item in that per-workspace
> menu that starts a chat targeted at *that* workspace specifically, not
> whatever happens to be active.

**Annotation #4** — the sidebar's generic empty-state block (`app/layouts/default.vue`,
the `v-if="!workspaceGroups.some(g => g.conversations.length > 0)"` block,
"No conversations yet. / Start one"):
> "kl ga bener bener kosong tanpa workspace mending ini ilangin aja" — this
> block currently shows whenever *no workspace has any chats*, even when
> the user has real workspaces (each already rendering its own, self-evidently
> empty section above it). Wants it limited to the genuinely-zero-workspaces
> case, or removed as redundant otherwise.

**Annotation #5** — just a pointer at the per-workspace conversation-list
container (`itemsFor(group.conversations)`'s `UNavigationMenu`), noting
"di dalem ini nanti ada list conversations" — not a bug on its own, just
context confirming where things render; no action needed beyond what
the other three already cover.

## Phase A — Workspace selector in the chat prompt form

1. `app/pages/chat/index.vue`: add a `USelect` for workspace next to the
   existing model/effort selects in `UChatPrompt`'s `#footer`, sourced
   from `workspaces.value` (already available via `useWorkspaces()`).
   Default it to `activeWorkspaceId` so the common case (you're already
   in the workspace you want) needs no extra click.
2. `start()` currently calls `create({ title, modelId, reasoningEffort })`,
   which internally reads `activeWorkspaceId` — change `create()`
   (`useConversations.ts`) to accept an explicit `workspaceId` override
   (matching the pattern `modelId`/`reasoningEffort` already use as
   `overrides.*`), and pass the form's selected workspace through
   explicitly rather than relying on the implicit active one.
3. Selecting a different workspace in this form should probably also
   call `setActive()` on submit (so the sidebar and this pick agree
   afterward) — decide whether to do it on selection change or only on
   submit; submit-time avoids switching the whole sidebar's view just
   because the user is browsing the dropdown.

## Phase B — Per-workspace "New chat" in the sidebar "…" menu

1. `workspaceActionItems(w)` in `app/layouts/default.vue` currently
   returns Confirm Folder / View details / Rename / Delete. Add a
   **"New chat"** item (icon `i-lucide-square-pen`, matching the
   existing top button) that targets workspace `w` specifically:
   `setActive(w.id)` then `router.push('/chat')` — same two-step the
   top-level `newChat()` does, just pinned to a specific workspace
   instead of "whatever's active."
2. Since Phase A adds an explicit workspace field to the landing form
   too, decide whether this menu item should skip that form entirely
   (go straight to a blank prompt pre-targeted at `w`) or just pre-select
   `w` in the Phase A selector when landing on `/chat` — pre-selecting is
   probably the more consistent choice, avoids a second code path for
   "create a conversation."

## Phase C — Clean up the sidebar empty-state block

1. `app/layouts/default.vue`'s bottom empty-state block
   (`v-if="!workspaceGroups.some(g => g.conversations.length > 0)"`,
   "No conversations yet. / Start one") currently fires whenever every
   workspace's chat list is empty, including when real workspaces exist
   and each already renders its own (visibly empty) section above it —
   redundant in that case per the annotation.
2. Narrow the condition to the genuinely-zero-workspaces case
   (`workspaces.value.length === 0`) — when workspaces *do* exist but
   have no chats yet, let each workspace's own section speak for itself
   without this extra block. Confirm what should render instead for the
   zero-workspace case specifically (probably still worth a short nudge
   toward creating one, since `+ New chat`/the per-workspace items don't
   apply without a workspace to attach to).

## Verification

- Live test: from `/chat` (landing form), pick a non-active workspace in
  the new selector, send a message, confirm the resulting conversation's
  `workspaceId` matches what was picked (not whatever was active before).
- Live test: from the sidebar, use a specific workspace's "…" → "New
  chat," confirm it lands you composing into that workspace, not
  whichever was previously active.
- Live test: a user with 2+ workspaces where all are empty of chats sees
  no redundant "No conversations yet" block; a user with zero workspaces
  still gets a clear nudge to create one.
- Confirm the "workspace ⇒ many chats" / chat-before-workspace guarantee
  (plan 009) still holds — none of this should make it possible to
  create a chat with no workspace.
- `pnpm build && vue-tsc -p .nuxt/tsconfig.json --noEmit && pnpm run lint && pnpm audit` clean.

## Out of scope

- Not changing how `activeWorkspaceId` persistence/restore works (plan
  015) — this plan only adds an explicit override at chat-creation time,
  doesn't touch how the "current" workspace is remembered.
- Not revisiting the URL structure — still no `/workspace/:id/chat`,
  per [[007-workspace-client-routing]].
