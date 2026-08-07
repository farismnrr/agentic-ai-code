# 008 — Remove dummy/seed data now that the backend is real

> **Status: complete.** Shipped to `dev` via PR #37 (squash-merged as `5f28475`). All six changes landed, plus a follow-up fix for a build-breaking import the first pass missed (`replies.ts` still importing from deleted `mcp-servers.ts`) and a lint fix for its replacement. `/security-review` found no HIGH/MEDIUM findings. Verified 2026-08-08.

## Context

Plans 001–003 built this app UI-first against fixtures (`shared/utils/fixtures/*`) with an explicit "no backend" disclaimer everywhere. Plans 005 and 007 then wired up a real Postgres-backed auth/data layer and a real model via 9Router. The fixtures and the "this is a prototype" copy never got cleaned up, so the app now has: a production endpoint that wipes a signed-in user's real data and replaces it with canned demo content, a chat component that resolves tool metadata from static fixture data instead of the user's real configured MCP servers, an account page whose "Save changes" button doesn't actually save, and landing/auth copy that tells users "any credentials work" and "nothing is stored" when both are false. This plan removes the dummy-data paths and corrects the copy and behavior to match what's actually real, with an eye on the security implications the user specifically flagged.

## Findings (what's dummy vs what's real, verified by reading the code)

- **`server/api/reseed.post.ts`** — a real, authenticated `POST` endpoint that deletes a signed-in user's `mcpServers`, `userSettings`, and `conversations` rows and replaces them with `shared/utils/fixtures/{conversations,mcp-servers}.ts` seed content. Reachable today from **Settings → General → "Reset demo data"** (`app/pages/settings/general.vue`). This is the security-relevant one the user called out: a destructive, unauthenticated-by-intent "demo reset" left live in a build with real persisted user data.
- **`app/components/ChatToolCall.vue`** — imports `mcpToolsById` from the fixture file to resolve which server a tool call belongs to, instead of the real `useMcpServers().toolsById` (already DB-backed per plan 005, already used correctly by `ChatToolPicker.vue` and `ChatToolApproval.vue`). A real tool call today would render against the wrong server list.
- **`shared/utils/fixtures/models.ts`** — **not dummy.** This is the real curated 9Router model list from plan 007 (`high-thinking-models`, `vx/gemini-3-flash-preview`, `free-models`). It's mislabeled by living in `fixtures/`, which is why it looked in-scope at first glance. Keep the data, move it out of `fixtures/`.
- **`shared/utils/fixtures/replies.ts`** (`pickScenario`) — used only by `LandingHeroDemo.vue`, a self-playing canned animation on the landing page hero. Per user decision: **keep** — normal marketing UX, not data presented to a real user as if it were theirs.
- **`app/pages/settings/account.vue`** — two real bugs found while reading it, both worth fixing alongside the copy pass since they're about this same "is this real or not" gap:
  - `onSubmit` only mutates local `settings.value`; it never calls `settings.update(...)` (the real `PUT /api/settings`), so "Save changes" silently doesn't persist.
  - `messageCount` sums `conversations.value[].messages.length`, but `GET /api/conversations` (list) intentionally omits message bodies (see its own comment) — so this count is always 0. The footer text under it ("Counts reset on reload — nothing is persisted in this build") is also just false now.
- **Landing/auth copy** — `app/pages/index.vue`, `app/layouts/landing.vue`, `app/layouts/auth.vue` all say things like "Prototype: no backend", "Any email and password gets you in. Nothing is stored", "the list is dummy data" for MCP servers, and an FAQ answer claiming replies "come from fixtures." All false post plan-005/007. Per user decision: fix this copy too.
- Confirmed via `find server/api server/routes` that `reseed.post.ts` is the **only** demo/debug-era endpoint left — nothing else in `server/` needs equivalent treatment.

## Changes

1. **Remove the destructive demo-reset path**
   - Delete `server/api/reseed.post.ts`.
   - Remove the "Reset demo data" `UCard` and `resetDemo()` handler from `app/pages/settings/general.vue`.
   - Delete `shared/utils/fixtures/conversations.ts` (only consumer was reseed).

2. **Fix `ChatToolCall.vue` to use real data**
   - Replace `import { mcpToolsById } from '#shared/utils/fixtures/mcp-servers'` with `const { toolsById } = useMcpServers()` (matching `ChatToolApproval.vue`'s existing pattern) and resolve `server` from that.
   - Once this and step 1 are done, `shared/utils/fixtures/mcp-servers.ts` has no consumers — delete it.

3. **Relocate the real model list out of `fixtures/`**
   - Move `shared/utils/fixtures/models.ts` → `shared/utils/models.ts` (content unchanged — it's real config, not a fixture).
   - Update the four import sites: `app/pages/chat/index.vue`, `app/pages/chat/[id].vue`, `app/pages/settings/models.vue`, `app/composables/useSettings.ts`.

4. **Fix `app/pages/settings/account.vue`**
   - `onSubmit` calls `await settings.update({ displayName: event.data.displayName, email: event.data.email })` (the real composable method, already used correctly elsewhere) instead of only touching local state.
   - Message count: since the list endpoint doesn't carry messages, either drop the "Messages" stat or source it from a real count. Simplest correct fix: keep "Conversations" (already accurate — `conversations.value.length`), drop the fabricated "Messages" stat rather than adding a new endpoint just for a settings-page number. Update the footer copy to reflect real persistence (no more "resets on reload / nothing is persisted").

5. **Copy pass — landing and auth**
   - `app/pages/index.vue`: SEO description, the "Nothing leaves the page" feature card, the FAQ answers ("Does this actually talk to a model?", "Is my data stored anywhere?", "Can I use my own MCP servers?"), the hero description, and the closing CTA ("Any email and password gets you in. Nothing is stored.") — rewrite each to state what's actually true (real auth, real Postgres persistence, real 9Router model, real per-user MCP config that a user must actually connect). Pricing/testimonials stay as explicitly-labeled illustrative content ("Illustrative. Nothing here charges anyone." / "Invented, like the pricing.") — those already disclose themselves honestly and aren't part of this cleanup.
   - `app/layouts/auth.vue` and `app/layouts/landing.vue`: drop "A prototype — no real accounts, any credentials work" / "A prototype. No backend, no real accounts." and replace with copy that doesn't undersell or misrepresent the real auth.

6. **Dead code**
   - `useConversations().reset()` (`app/composables/useConversations.ts`) has no remaining callers after step 1 — remove it.

## Security note (per user's explicit ask)

The core issue is #1: a live, reachable endpoint that destroys and overwrites a real user's persisted data under the label "demo reset," in a build where that data is no longer disposable. It's scoped to `session.user.id` (not a cross-user vuln), but it has no business existing now that data is real — removing it *is* the fix, not a follow-up. After the changes above, run the `security-review` skill against the diff before calling this done, specifically checking: no other mutating endpoint lacks proper `session.user.id` scoping, and nothing in the copy changes accidentally weakens or misdescribes the auth model (e.g. don't overclaim security properties that aren't actually true either).

## Verification

- ✅ `pnpm lint` and `pnpm audit` clean. `pnpm typecheck` (`nuxt typecheck`) reported clean, but per `.agents/memories/007-typecheck-gate-was-silent.md` that command is known to miss real errors — confirmed independently with a full `nuxt build` + direct `vue-tsc -p .nuxt/tsconfig.json`, which caught a real build-breaking regression the first commit introduced (`replies.ts` still importing the deleted `mcp-servers.ts`) and, after that fix, a lint-only issue (`any`-typed stub missing `McpTool` fields). Both fixed; final state passes build + vue-tsc + lint + audit all for real, not just via the unreliable script.
- ✅ `grep -rn "fixtures" app shared server` returns only `replies.ts`'s consumer (`LandingHeroDemo.vue`) — confirmed clean.
- ✅ `/api/reseed` route file deleted, "Reset demo data" UI removed, `grep -rn "reseed"` across the repo returns nothing.
- ✅ `ChatToolCall.vue` resolves tools via `useMcpServers().toolsById` (matches `ChatToolApproval.vue`'s pattern).
- ✅ `account.vue`'s "Save changes" now calls `settings.update(...)` for real.
- ✅ Landing/auth copy no longer claims "prototype"/"no backend"/"any credentials work"; the MCP FAQ answer was corrected twice — first to say tool calling is fully supported (overclaim, since execution isn't wired up per plan 007), then fixed to accurately say config is stored but not yet executed during chat.
- ✅ `/security-review`: one general-purpose agent pass over the full diff, no HIGH/MEDIUM findings. Noted (not a finding): `server/api/settings.put.ts` accepts an unvalidated `email` string server-side, but it's pre-existing, out of this diff's scope, and self-scoped to `session.user.id` — no cross-user impact.

## On completion

Per `.agents/knowledge/self-improvement.md`: write this plan to `.agents/plans/008-remove-dummy-data.md`, tick items off as they land, move it to the Done list in `.agents/plans/README.md`, and record in `.agents/memories/` if anything here is a trap worth remembering (e.g. "fixtures/ is not all disposable — models.ts was real config that had drifted into the wrong folder").
