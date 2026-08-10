# 025 — Skeleton states + lazy data loading

## Status: CLOSED

Implemented and shipped to `dev` as commit `c089a1a` ("feat(ui): add
skeleton states and lazy data loading") — `useSidebarData`'s own
`pending`/`error` state, `layouts/default.vue`'s sidebar skeleton/error/
retry, `settings/models.vue` and `settings/api-keys.vue` converted to
`useLazyFetch`, `chat/[id].vue` kept its deliberate blocking load but
gained try/catch + retry, and a shared `DataLoadError.vue` component.
`pnpm lint`/`pnpm typecheck` clean.

Note for next time: this landed as a direct commit to `dev`, not through
a feature branch + PR — outside the workflow in
[`../knowledge/git.md`](../knowledge/git.md). No action needed now since
it's already merged and pushed (history isn't being rewritten after the
fact), but branch it next time.

## Goal

Right now most data-loading is a single blocking `await useFetch(...)` /
`await useAsyncData(...)` at the top of a page or layout `<script setup>`.
Two consequences of that shape, both visible today:

1. **No loading UI.** Nuxt blocks navigation until the fetch resolves, so
   there's nothing to skeleton *unless* we deliberately switch to the lazy
   variant. `chat/index.vue` already does this correctly (`USkeleton` while
   its own local `pending` is true) — everything else doesn't.
2. **A failed/slow API breaks the whole screen**, not just the widget that
   needed it. `app/layouts/default.vue`'s `useAsyncData('app-data', ...)`
   wraps sidebar + settings + MCP servers + active-conversation load in one
   blocking call with no `error` handling — if any of those endpoints 500s
   or times out, the *layout* fails, which takes the whole app down instead
   of degrading the one panel that needed that data. Same blocking-without-
   fallback shape in `settings/models.vue` and `settings/api-keys.vue`.

Fix: convert these fetches to Nuxt's lazy pattern (`lazy: true` /
`useLazyFetch` / `useLazyAsyncData`), render a `USkeleton` while `pending`
is true, and render a lightweight inline error/retry state when `error` is
set — instead of letting the fetch failure propagate up to Nuxt's global
error page.

## Non-goals

- No new dependencies — `useLazyFetch`/`useLazyAsyncData`/`error` are core
  Nuxt, `USkeleton` is already used in this codebase (`chat/index.vue`).
- Not touching the streaming chat fetch in `server/api/chat.post.ts` /
  `useConversationChat.ts` — that's a POST/stream, not a page-load GET, out
  of scope here.
- Not changing what the endpoints return, only how the frontend consumes
  them while in flight or on failure.

## Current blocking call sites (from grep across `app/`)

| File | Call | Used by |
| --- | --- | --- |
| `app/layouts/default.vue:14` | `await useAsyncData('app-data', ...)` | sidebar (workspaces + conversations), settings, MCP servers, deep-linked conversation |
| `app/pages/settings/models.vue:8` | `await useFetch('/api/settings/models-config')` | model/provider dropdowns |
| `app/pages/settings/api-keys.vue:8` | `await useFetch('/api/api-keys', ...)` | API key table |
| `app/components/WorkspaceFolderPicker.vue:32` | `await useFetch('/api/fs/browse', ...)` | already has a `pending` prop wired to `:loading`/`:disabled` on the picker button — leave as is, it's already non-blocking UX (a picker dialog, not page load) |

`chat/index.vue` is the reference pattern to copy from — keep it as-is.

## Plan

### Step 1 — Sidebar / layout (`app/layouts/default.vue`)

This is the highest-impact one: it gates the entire app shell.

- Switch `useAsyncData('app-data', ...)` to `{ lazy: true }` and capture
  `pending` / `error`.
- The sidebar's `NavigationMenu` (workspace groups + conversations) renders
  from `useSidebarData()`'s reactive state, which starts empty — replace
  the empty state with a `USkeleton`-based sidebar placeholder (a handful
  of `USkeleton` rows) while `pending` is true, driven by `useSidebarData()`
  exposing its own `pending`/`error` refs (mirror the existing `loaded`
  pattern already in `useWorkspaces()`/`useSidebarData()` — add `pending`
  and `error` `useState`s next to it) rather than the page-level
  `useAsyncData` pending flag, since `loadSidebar`/`loadSettings`/
  `loadMcpServers`/`loadOne` are invoked inside one non-awaited batch (see
  the SSR-context comment already in this file — don't break that).
- On `error`, show a small inline "Couldn't load your workspaces — retry"
  state in the sidebar instead of letting the error propagate to Nuxt's
  error page. Wire retry to re-call `loadSidebar()`.
- Keep the existing `Promise.allSettled` + non-awaited-invocation shape
  inside the loader — this step only changes how the *caller* (the
  template) reacts to pending/error, not the SSR-context-sensitive
  sequencing inside `useAsyncData`'s callback.

### Step 2 — `app/pages/settings/models.vue`

- Switch to `useLazyFetch('/api/settings/models-config')`, capture
  `pending`/`error`.
- Skeleton the model list / provider dropdowns while pending (`USkeleton`
  rows matching the real list-item height, same count as a typical
  provider list so there's no layout jump).
- On error, inline retry state instead of a blank/broken settings page.

### Step 3 — `app/pages/settings/api-keys.vue`

- Switch to `useLazyFetch('/api/api-keys', { default: () => [] })`, capture
  `pending`/`error`.
- Skeleton the key table rows while pending.
- On error, inline retry state; keep `default: () => []` so the "create
  key" form still renders even before the list resolves.

### Step 4 — `app/pages/chat/[id].vue`

This one is a *deliberate* blocking `await loadOne(conversationId.value)`
(and `await loadModels()` above it) — the code comment explains why: this
page's `useConversationChat()` seeds from `conversation.value.messages`,
and re-seeding after mount would reset in-progress chat state. This is not
a candidate for `lazy: true` — don't convert it to lazy just for
consistency, that would reintroduce the bug the comment is guarding
against.

What *is* still missing here, and is in scope:

- No error handling around either await. If `/api/conversations/:id` or
  `/api/models` fails, this throws inside the page's setup and hits Nuxt's
  global error page (`error.vue` / the crash overlay), same failure class
  as the rest of this plan, just via a different mechanism (a route-level
  `await` instead of `useAsyncData`).
- Wrap both in `try/catch`; on failure, render an inline "Couldn't load
  this conversation — retry" state (reuse the same small error component
  built in Step 1) instead of letting it propagate, and surface a toast for
  transient cases.
- The blocking wait itself already gets a "loading" affordance for free:
  Nuxt's route transition / `<NuxtPage>` shows nothing until this async
  setup resolves. That gap should be a `USkeleton`-based skeleton, not a
  blank screen — wrap `<NuxtPage>` in `app/app.vue` with `<Suspense>`
  (Nuxt does this internally per-page already via `definePageMeta` when
  needed) and provide a fallback, or — simpler and scoped to this one page
  — set `definePageMeta({ key: route => route.fullPath })` is unrelated;
  instead add a page-level pending flag: convert the two blocking awaits to
  fire in `onMounted`/a `pending` ref pattern only if profiling shows the
  blank gap is actually visible (SSR should already have the data by the
  time the client sees the page in the common case). Confirm the real gap
  before adding complexity here — this sub-item may turn out to be a no-op
  once Step 1–3's skeletons are in and the sidebar itself is no longer the
  bottleneck.

### Step 5 — Sweep for anything missed

- Re-grep `app/` for `useFetch`/`useAsyncData` after steps 1–4 to confirm
  no other blocking call sites were missed (the `WorkspaceFolderPicker.vue`
  one is intentionally left as-is, see table above).
- Confirm `settings/general.vue`, `settings/account.vue`,
  `settings/mcp.vue`, `settings/index.vue` don't have their own blocking
  fetches (they read from the already-loaded `useSettings()`/
  `useMcpServers()` state per the earlier grep — verify this still holds).
- `settings/index.vue` is just a `navigateTo` redirect — no data loading,
  nothing to do there.

## Shared pieces (build once, reuse everywhere)

To keep this consistent across Steps 1–4 rather than four one-off
implementations:

- **A small reusable inline error/retry block.** Nuxt UI has no built-in
  "retry" component; build one tiny local component
  (`app/components/DataLoadError.vue` — message + `UButton` "Retry" that
  emits `retry`) rather than repeating the same markup in four places.
- **Skeleton row counts should match real content shape** (sidebar rows,
  table rows, list items) so there's no layout jump when data arrives —
  this is the Nuxt UI `USkeleton` convention already established in
  `chat/index.vue`.
- Every conversion follows the same Nuxt-native shape: `useLazyFetch` /
  `useAsyncData(..., { lazy: true })` → destructure `{ data, pending,
  error, refresh }` → template branches on `pending` (skeleton) / `error`
  (`DataLoadError` wired to `refresh`) / else (real content). This is the
  documented Nuxt pattern for async data (nuxt.com/docs/getting-started/data-fetching),
  not a custom convention — matches the "Nuxt way" working agreement in
  `.agents/knowledge/nuxt-way.md`.

## Verification

- `pnpm lint` and `pnpm typecheck` pass.
- Manually: throttle/kill the relevant API route (or stop the DB) and load
  `/chat`, `/settings/models`, `/settings/api-keys` — each should show a
  skeleton then an inline retry state, not a blank page or Nuxt's crash
  overlay.
- Manually: normal path — skeletons appear briefly then resolve to real
  content, no layout shift between skeleton and loaded state.
- No regression in the deep-link-to-conversation SSR behavior described in
  `app/layouts/default.vue`'s existing comments — open `/chat/<id>` directly
  (not via sidebar click) and confirm the correct workspace still becomes
  active.
