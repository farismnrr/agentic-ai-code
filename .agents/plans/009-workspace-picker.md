# 009 — Require picking a workspace before the first chat, fix the race that throws

> **Status: complete.** Shipped on `feat/009-p1-workspace-picker` → `dev`. The original implementation (`0cbac67`) needed three follow-up fixes, found by actually running the app rather than trusting `pnpm lint`/`nuxt typecheck` alone — see Verification below and `.agents/memories/nuxt-ssr-fetch-cookies.md`.

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

- ✅ `pnpm lint`, a real `nuxt build` + `vue-tsc -p .nuxt/tsconfig.json` (not just `nuxt typecheck` — see `.agents/memories/007-typecheck-gate-was-silent.md`), and `pnpm audit` all clean on the final commit.
- ✅ Fresh registered user, real SSR requests (not just client-side clicking) against `/chat` — confirmed the picker renders reliably, not intermittently.
- ✅ Picking/creating a workspace sets the cookie and reloading `/chat` goes straight to the prompt.
- ✅ No uncaught console error on rapid-submit before data settles.

**Three real bugs were found only by actually running the app**, each fixed in its own follow-up commit rather than folded silently into the first one:

1. `0cbac67` (first pass) shipped `loaded` as a bare `ref(false)` inside `useWorkspaces()`. Since the composable is a factory function called independently in both `default.vue` and `chat/index.vue`, each call site got its *own* disconnected `loaded` — `default.vue` flipped its copy to `true`, `chat/index.vue`'s copy never moved, so the picker was **permanently unreachable** (stuck on the loading skeleton forever). Fixed in `2202a28` by wrapping it in `useState<boolean>('workspaces-loaded', ...)`, the same pattern already used for `workspaces` itself.
2. Even after that fix, the picker was still **intermittent** (~2/3 of fresh SSR requests stuck on the skeleton). Root cause: `default.vue`'s `Promise.all([loadSettings(), loadWorkspaces().then(loadConversations), loadMcpServers()])` — `Promise.all` rejects as soon as *any* promise rejects, without waiting for the others, so a failing `loadSettings()` could abandon `loadWorkspaces()` mid-flight before it set `loaded = true`. Fixed in `97cb168` by switching to `Promise.allSettled`.
3. That exposed the real, pre-existing root cause: `GET /api/settings` (and by the same code path, every other composable's `loadAll()`) was 401ing specifically on **SSR-internal** requests, while the exact same session cookie worked fine for external requests. Cause: `useSettings`/`useWorkspaces`/`useMcpServers`/`useConversations` all called the bare global `$fetch`, which does not carry the incoming request's cookies during SSR — `useRequestFetch()` is required for that. Fixed in `5036173`, applied to all four composables. Verified by SSR-curling `/chat` with a real session cookie and confirming the real user's data (not the composable's hardcoded fallback) renders on first paint.

Recorded as `.agents/memories/nuxt-ssr-fetch-cookies.md` — this is a trap that will recur the moment a new composable adds its own `$fetch('/api/...')` call.

## On completion

- [x] Plan file updated with status and real verification results.
- [x] Moved to the Completed list in `.agents/plans/README.md`.
- [x] `.agents/memories/nuxt-ssr-fetch-cookies.md` written and indexed.
- [x] Branch pushed, PR opened against `dev`, CI green, merged, branch/worktree cleaned up per `.agents/knowledge/git.md`.
