# 002 — Landing → login → app, and make the prototype actually interactive

## Context

Plan 001 delivered the chat UI, but the app opened straight into a chat screen and, worse, **wasn't usable**: `Comark` never resolved, which broke hydration on every chat page and left nothing clickable. That's fixed (PR #12), but it exposed the real gap — 001 was verified by reading server-rendered HTML, which proves markup exists, not that the app runs.

This plan does two things:

1. **Add the front of the product** — a landing page, login and register, with a route guard, so the flow reads `landing → login → app` instead of dropping a visitor into a chat.
2. **Close the interaction gaps** left over from 001, so this behaves like a working prototype rather than a set of screens.

Still no backend. All data is dummy, in memory.

## Decisions already made

- **Fake auth with a real route guard.** Any credentials work; a guard bounces unauthenticated visitors to `/login`.
- **State stays in memory** and resets on reload — *except the session*. `localStorage` holds only the logged-in user, so a refresh doesn't kick you back to login. Conversations, settings and MCP servers all reset to seed data, so every demo starts clean.
- **Full landing**: hero, features, pricing, testimonials, FAQ.
- **No Playwright.** Verification is manual, by you. See Verification for how I'll compensate.

## Route restructure

Landing takes `/`, so the app moves under `/chat`:

| Route | Layout | Notes |
| --- | --- | --- |
| `/` | `landing` | hero, features, pricing, testimonials, FAQ |
| `/login`, `/register` | `auth` | centred card, any credentials accepted |
| `/chat` | `default` | the empty state currently at `/` |
| `/chat/[id]` | `default` | moved from `/c/[id]` |
| `/settings/*` | `default` | unchanged |

`/c/[id]` → `/chat/[id]` is a rename, not a rewrite. Every internal link lives in `app/layouts/default.vue`, `app/pages/index.vue` and `app/composables/usePendingPrompt.ts` consumers — grep for `` `/c/` `` before assuming a file is unaffected.

## Build order

Each phase ends green (`pnpm lint && pnpm typecheck && pnpm audit`) and gets its own PR into `dev`, per `.agents/knowledge/git.md`.

### 1. Auth foundation

- `app/composables/useAuth.ts` — `user` ref, `login()`, `register()`, `logout()`. Session in `localStorage` under one key; everything else stays `useState`. Read it in a `.client.ts` plugin, not at module scope — `localStorage` doesn't exist during SSR.
- `app/middleware/auth.global.ts` — unauthenticated visitors hitting an app route go to `/login?redirect=<path>`; authenticated visitors hitting `/login` go to `/chat`. Public routes: `/`, `/login`, `/register`.
- Guard runs client-side only for the `localStorage` read; make sure the server render doesn't flash the wrong screen.

### 2. Landing

- `app/layouts/landing.vue` — `UHeader` with nav + "Sign in", `UFooter`.
- `app/pages/index.vue` becomes the landing page using Nuxt UI's page components, all of which are already available: `UPageHero`, `UPageSection`, `UPageFeature`, `UPricingPlans`/`UPricingPlan`, `UPageColumns` for testimonials, `UAccordion` for FAQ, `UPageCTA`.
- Copy is invented but should describe *this* app — streaming chat, MCP tools, multi-model — not generic SaaS filler.

### 3. Auth pages

- `app/layouts/auth.vue` — centred card, logo, link back to `/`.
- `app/pages/login.vue`, `app/pages/register.vue` — `UForm` + valibot, matching `settings/account.vue`'s existing pattern. Any email/password combination succeeds; show the validation errors so the form feels real.
- Honour `?redirect=`. Fake OAuth buttons are **out of scope** — they were declined.

### 4. Move the app under `/chat`

- Move `pages/index.vue` (empty state) → `pages/chat/index.vue`, `pages/c/[id].vue` → `pages/chat/[id].vue`.
- Update every link and `router.push`.
- Wire the sidebar user menu's "Sign out" to `logout()` — it's currently a dead button.

### 5. Close the interaction gaps

The point of the plan: things that look interactive but aren't.

- **Rename a conversation** — inline edit or a menu item. Only delete exists today.
- **Sidebar row menu** — rename / delete in a `UDropdownMenu` instead of a hover-only trash icon, which is undiscoverable and unusable on touch.
- **Message actions** — thumbs up/down and an edit-and-resend on user messages; copy and regenerate already work.
- **New chat from the sidebar** currently routes to the empty state — confirm that still holds after the move.
- **Settings that claim to do something must do it**: `streaming` should actually toggle the mock transport's delay, `sendOnEnter` should change the prompt's submit behaviour, `defaultModelId` should be what `create()` uses. Right now they're stored and ignored.
- **Reset demo data** button in settings, since nothing persists and a wedged demo needs a way out.

### 6. Polish and consistency

- Landing and auth pages need the dark-mode pass; they're new surfaces.
- Mobile: landing sections stack, auth card doesn't overflow.
- `routeRules` — landing at `/` is static and *should* be prerendered again, which reverses the 001 change now that `/` is no longer stateful.

## Conventions to hold

From `.agents/knowledge/`:

- Semantic colours only. The raw-palette grep is the check.
- `nuxi module add` for any Nuxt module; `pnpm audit` must be zero before each merge.
- Read `.nuxt/ui/<component>.ts` or use the `nuxt-ui` MCP for real props — **the skill's reference is not always current.** That's exactly what caused the `Comark` bug: it documented a component name and prop that no longer exist.

## Verification

Per phase: `pnpm lint && pnpm typecheck && pnpm audit`, plus `pnpm build` before the final PR.

**Since there's no Playwright, I'll compensate with the check that would have caught the last bug:** Nuxt pipes the browser console into the dev-server log, so after loading each route I'll grep that log for `Failed to resolve component`, `Hydration`, and `[console.error]`. A page is only "done" when it renders *and* produces no client-side errors. That's how the `Comark` bug was eventually found — it had been shouting in the log for hours.

End to end, in your browser at http://100.99.88.53:3333:

1. `/` shows the landing page, not a chat.
2. "Get started" → `/login`. Submitting anything lands on `/chat`.
3. Hitting `/chat/seed-nuxt-ui` while logged out redirects to `/login`, and logging in returns you there.
4. Refresh anywhere in the app — still logged in, conversations back to seed data.
5. Send a message; the reply streams. Stop mid-stream. Regenerate.
6. Ask something that triggers a tool → approval dialog → allow → tool card renders with arguments and result.
7. Rename a conversation from the sidebar; the title updates in the list and the navbar.
8. Turn off `streaming` in settings; the next reply arrives without token delay.
9. Sign out → back to `/login`, and `/chat` is no longer reachable.

## Out of scope

No backend, no real auth, no persistence beyond the session key, no OAuth, no Playwright.

## On completion

Copy to `.agents/plans/002-landing-auth-interactive.md`, tick phases there, and record in `.agents/memories/` anything a future agent could get wrong — the `Comark`-class trap (skill references drifting from installed packages) is already worth its own note.
