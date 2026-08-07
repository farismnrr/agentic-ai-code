# 009 — Require picking a workspace before the first chat, fix the race that throws

## Context

Two related problems surfaced from a browser console dump on `/chat`:

1. **A real bug**: `useConversations().create()` (`app/composables/useConversations.ts:47`) throws `Error: No active workspace` synchronously if `useWorkspaces().activeWorkspaceId` is still `null` when the user submits their first message. `app/pages/chat/index.vue`'s `start()` calls `create()` with no guard or try/catch, so this becomes an uncaught promise rejection in the browser — the empty-state chat prompt accepted input and let the user submit before workspace data had necessarily settled.
2. **A UX gap**: right now the active workspace is picked *silently* — `useWorkspaces().loadAll()` (`app/composables/useWorkspaces.ts:16`) auto-selects the first workspace whenever none is active, and a cookie (`workspace-id`) remembers it after that. The user wants the opencode-web-style flow instead: landing on `/chat` for the first time in a session should present an explicit "pick or create a workspace" step — like choosing a project folder — before the chat prompt appears at all. Once picked, it's remembered (cookie, as today) and subsequent `/chat` visits go straight to the prompt.

Fixing the picker flow structurally closes off the main way the race in #1 was reachable (you can no longer see the chat input without an active workspace already set), and the remaining edge cases get defensive handling too.

## Decisions (confirmed)

- The picker appears on `/chat` (the empty-state "New chat" page) whenever no workspace is active for the session — not on every visit once one has been picked, and not as a blocking modal. It replaces the page body, consistent with how the empty state already works.
- Workspace list/creation reuses the existing `useWorkspaces()` composable (`app/composables/useWorkspaces.ts`) — no new API routes; `server/api/workspaces/*` already covers list/create.

## Changes

1. **`app/composables/useWorkspaces.ts`** — add a `loaded` ref (`ref(false)`), set to `true` at the end of `loadAll()` regardless of whether any workspaces came back. Export it. This is what lets the empty-state page distinguish "still fetching" from "genuinely zero workspaces" — right now both look identical (`workspaces.value` is `[]` either way), which is the root of the flash/race risk, not just a UI nicety.

2. **New `app/components/WorkspacePicker.vue`** — shown in place of the chat prompt. A `UPageGrid`/`UPageCard` grid (matches the visual language already used on the landing page, `app/pages/index.vue`) of the user's workspaces, each clickable to set `activeWorkspaceId.value = workspace.id`; plus a "Create workspace" card that opens the same small name-input `UModal` pattern already implemented in `app/layouts/default.vue`'s sidebar (`workspaceCreating`/`workspaceName` refs, `confirmCreateWorkspace()`) — lift that modal's logic into this new component rather than duplicating it, and simplify `default.vue`'s sidebar "New workspace" action to open the same shared piece if that's a clean fit, otherwise leave the sidebar's existing modal alone and only avoid duplicating the *business logic* (`useWorkspaces().create()` call + toast) — not necessarily the JSX.

3. **`app/pages/chat/index.vue`** — three states instead of one:
   - `!loaded` → a lightweight loading placeholder (a couple of `USkeleton` blocks is enough; no need for anything fancier).
   - `loaded && !activeWorkspaceId` → `<WorkspacePicker />`.
   - `loaded && activeWorkspaceId` → today's prompt UI, unchanged.
   Also wrap `start()`'s body in try/catch: on failure, `toast.add(...)` an error instead of letting it reach the console as an uncaught rejection — defense in depth for any future edge case (e.g. the active workspace gets deleted in another tab between page load and submit).

4. **`app/composables/useConversations.ts`** — `create()` keeps throwing when there's truly no active workspace (that's still a real error condition), but the call site now always handles it instead of letting it go uncaught.

## Out of scope

- No change to `/chat/[id]` (an existing conversation's URL already implies its workspace).
- No change to routing/URL structure — still the client-side-cookie approach from plan 007 (`.agents/memories/007-workspace-client-routing.md`).
- No change to the sidebar workspace switcher's own behavior, beyond not duplicating its create-modal logic.

## Verification

- `pnpm lint && pnpm typecheck && pnpm audit` green.
- Fresh browser session (clear the `workspace-id` cookie), sign in, land on `/chat`: see the picker, not the prompt.
- Pick a workspace → prompt appears, cookie is set, reloading `/chat` goes straight to the prompt.
- From the picker, create a new workspace → it becomes active immediately, same as picking an existing one.
- Throttle the network (or briefly delay `/api/workspaces` server-side) and confirm the loading placeholder shows instead of a flash of "no workspaces."
- Confirm no uncaught console error is reachable by rapid-submitting on `/chat` before data settles.

## On completion

Write this plan to `.agents/plans/009-workspace-picker.md`, tick items off as they land, move it to the Done list in `.agents/plans/README.md`, and note in `.agents/memories/` if the "loaded" flag pattern is worth remembering for other `useState`-backed composables that have the same empty-vs-loading ambiguity (`useConversations`, `useMcpServers` have the identical shape).
