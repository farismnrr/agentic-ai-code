# 015 — Persist the active workspace server-side

## Why

User report: the active workspace resets and the workspace picker
reappears on refresh. Investigation confirmed the underlying architecture
is mostly right — `conversations.workspaceId` is a required, `NOT NULL`
FK (`server/database/schema.ts`), and both the API
(`server/api/conversations/index.post.ts`) and the client
(`useConversations().create()`, `app/pages/chat/index.vue`) already refuse
to create a chat without an active workspace. **"Workspace ⇒ many chats"
is already enforced end to end** — that part of the ask is done, not a bug.

The real gap: *which* workspace is "active" is tracked only in
`useCookie('workspace-id', ...)` (`app/composables/useWorkspaces.ts:7`),
with **no `maxAge` set** — a session cookie, cleared whenever the browser
session ends — and no DB-side record of it at all. The workspace *data*
(name, path) is properly persisted in Postgres and reloaded every page
load; only the "which one was I looking at" pointer is not. This is
narrower than "workspace isn't saved" — it's "the *pointer* to the
workspace isn't saved" — but the user-visible effect (picker reappears,
have to reselect) is the same complaint.

One open question from investigation, not yet resolved: the user describes
losing it on every refresh, but a session cookie should survive a plain
F5 (it only clears on closing the browser). That mismatch means either
the session-cookie explanation is incomplete, or something else drops the
cookie earlier than expected — Phase 1 exists to pin this down before
assuming the fix is "just persist server-side."

## Phase 1 — Confirm the actual failure mode

Don't skip this and jump straight to a DB column — the live symptom
("every refresh") doesn't fully match the diagnosed cause (session
cookie), so there may be a second, compounding bug.

1. Reproduce live: pick a workspace, do a plain page refresh (not close
   the tab/browser) in a real browser session, and check via devtools
   (or a Playwright `context.cookies()` check) whether the `workspace-id`
   cookie is actually present and unexpired immediately after refresh.
2. If the cookie **is** present after refresh but the picker still shows:
   the bug is in how `activeWorkspaceId` is read/hydrated on load (an SSR
   hydration mismatch, or `loadAll()`'s existing-workspace check at
   `useWorkspaces.ts:23-26` clearing it incorrectly), not cookie expiry.
   Root-cause that path specifically.
3. If the cookie is genuinely gone after a plain refresh: check for a
   `Set-Cookie` response on every request that might be re-issuing it
   with conflicting attributes (path/domain mismatch), comparing against
   how the auth session cookie is configured
   (`nuxt.config.ts`'s `session.cookie.secure` override, with its
   documented HTTP-vs-HTTPS trap) — the workspace cookie has no equivalent
   configuration at all right now.
4. Write down whatever the real mechanism turns out to be before Phase 2
   — Phase 2's server-side persistence fixes the *durability* problem
   (surviving a closed browser, a different device) but won't fix a
   distinct hydration/config bug if step 2 turns out to be the actual
   cause of "every refresh."

## Phase 2 — Durable server-side "active workspace"

Matches the Antigravity-style structure the user asked for: the active
workspace becomes a real, server-known fact about the user, not a
browser-only guess.

1. Add `lastActiveWorkspaceId` to the `users` table
   (`server/database/schema.ts`) — nullable FK to `workspaces.id`, likely
   `onDelete: 'set null'` (deleting a workspace shouldn't cascade-delete
   the user; it should just clear the pointer, and `useWorkspaces.ts`
   already has fallback logic for "active workspace no longer exists").
   Generate the migration the same way plan 014's `reasoning_effort`
   column was added.
2. Expose it through whatever already returns user state to the client on
   load (check `server/utils/settings.ts`/`GET /api/settings`, or add a
   dedicated small endpoint if that's the wrong place) — read once at
   app-data load time in `app/layouts/default.vue`'s existing
   `useAsyncData('app-data', ...)` block, which already has exactly this
   kind of "resolve workspace before the sidebar renders" logic for the
   deep-link case.
3. **Centralize the write path.** Right now `activeWorkspaceId.value = X`
   is set directly in five places (`useWorkspaces.ts` internally,
   `WorkspacePicker.vue`, three sites in `layouts/default.vue`). Add a
   `setActive(id: string | null)` function to `useWorkspaces()` that
   updates the cookie *and* fires a persist call to the server
   (fire-and-forget is fine — this is a preference, not data integrity —
   but log failures, per
   [[chat-onend-silent-persistence-failure]]'s lesson about not letting
   background writes fail invisibly). Update all five call sites to go
   through it instead of assigning `.value` directly.
4. On load: if the cookie is unset/stale but the server has a
   `lastActiveWorkspaceId` that still exists in the loaded workspace list,
   seed the cookie from that instead of showing the picker. If *neither*
   exists (new user, or their last workspace was deleted), the picker
   still correctly appears — this isn't removing the "must pick a
   workspace" requirement, just making a real prior pick survive.
5. Also give the cookie a real `maxAge` (defense in depth / avoids a
   round trip being the only path back) even after Phase 2 lands — the
   DB is the source of truth, the cookie is a fast-path cache of it.

## Verification

- Live test, real browser: pick a workspace, plain refresh — stays.
  Close the browser entirely (or clear cookies) and reopen — still
  restores from the server, not just "still works because the cookie
  survived."
- Confirm deleting the active workspace still falls back correctly (the
  existing fallback-to-first-remaining logic in `useWorkspaces.ts`'s
  `remove()` and `loadAll()`), now also clearing/updating the server-side
  pointer, not just the cookie.
- Confirm a user with zero workspaces still gets the picker, not an
  error — this must not regress the "no chat without a workspace"
  guarantee that's already correctly enforced.
- `pnpm build && vue-tsc -p .nuxt/tsconfig.json --noEmit && pnpm run lint && pnpm audit` clean.

## Out of scope

- Not changing the URL/routing structure (workspace stays out of the
  path) — already a deliberate, recorded decision in
  [[007-workspace-client-routing]] and plan 009; this plan doesn't revisit
  it, only fixes durability of the selection itself.
- Not adding multi-device "currently active" sync/realtime — last-write-wins
  on the new DB column is enough; no requirement for live cross-tab sync.
