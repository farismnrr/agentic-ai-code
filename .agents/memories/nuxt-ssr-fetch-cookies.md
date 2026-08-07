# Bare `$fetch('/api/...')` in a composable silently 401s during SSR

Any composable that fetches its own app's API routes (`useWorkspaces`, `useConversations`, `useSettings`, `useMcpServers`) must use `useRequestFetch()` on the server, not the plain auto-imported `$fetch`:

```ts
const fetch = import.meta.server ? useRequestFetch() : $fetch
const data = await fetch<T>('/api/whatever')
```

**Why**: the bare global `$fetch` does not carry the incoming request's cookies when Nuxt makes an internal SSR-side call to one of its own `requireUserSession()`-gated routes. The external request (browser → server) has the session cookie; the internal request (server → its own API route, made while rendering) does not, unless you explicitly use `useRequestFetch()` to forward it. This fails as a **401**, not a connection error, so it looks like an auth bug rather than a fetch-plumbing bug — easy to chase in the wrong direction.

**How it actually surfaced** (plan 009): `default.vue`'s `Promise.all([loadSettings(), loadWorkspaces()..., loadMcpServers()])` had one of these silently 401 during SSR, which — combined with `Promise.all`'s all-or-nothing rejection — starved the other calls before they could finish, breaking the then-new workspace picker intermittently (~2/3 of requests). The picker bug was two composable-level fixes away from its actual root cause; don't stop at the first fix that makes the symptom go away in one test if a build/`vue-tsc` run and a couple of manual `curl` checks against SSR HTML still show stale/fallback data. See `.agents/memories/verify-in-a-browser.md` for the general version of this lesson — SSR HTML rendering the "logged in" shell is not proof the SSR-internal fetches actually succeeded.

**Where to look if this repeats**: any *new* composable added under `app/composables/` that calls `$fetch` against a same-origin `/api/*` route needs this pattern from the start, not bolted on after someone notices stale data on first paint.
