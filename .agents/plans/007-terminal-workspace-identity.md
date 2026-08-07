# 007 — Terminal/workspace identity + real model via 9Router

> **Status: complete.** Parts A, B, C shipped on `feat/007-terminal-workspace-identity`. Verified manually end-to-end (see Verification below) on 2026-08-08.

## Context

The app currently reads as a external MCP client clone: rounded chat bubbles, a soft
graphite/cyan "Instrument" theme (plan 003, complete), and replies generated
by `pickScenario()` — a canned fixture, not a model. The user wants two
independent but co-shipped changes:

1. **Visual/IA rebrand** toward opencode-web / OpenClaw: dark-by-default,
   dense/minimal sidebar, monospace-forward chat surface, command-palette-
   first navigation, and a "workspace" grouping concept (a workspace holds
   many chats; the user can have several workspaces). Confirmed direction
   from user answers: dense sidebar, mono typography in chat, ⌘K as primary
   nav, dark default — plus a `DESIGN.md` authored via the `design-md` skill,
   informed by `frontend-design` skill guidance, before any token changes.
2. **Real model answers** — no more mock scenarios. Wire `server/api/chat.post.ts`
   to the `9router` process already running under pm2, which exposes an
   OpenAI-compatible API at `http://localhost:20128/v1` (confirmed working:
   `curl .../v1/chat/completions` with the key from `~/.9router/db.json`
   returned a real completion from `minimax-m3`). Tool-calling stays out of
   scope — plain streaming text only, per the user's explicit instruction.

Both changes are scoped to keep `useChat()` / the AI SDK data-stream
transport untouched on the frontend — plan 001 already built that seam
correctly; this plan reuses it rather than re-deriving it.

## Part A — Real model wiring (do first; smallest, unblocks manual testing of everything else)

- **Runtime config**: add `NUXT_ROUTER_BASE_URL` (default `http://localhost:20128/v1`)
  and `NUXT_ROUTER_API_KEY` to `.env.example` / `nuxt.config.ts` runtimeConfig
  (server-only, not `public`). The real key (`sk-b779a94bf4382cee-...`, found
  in `~/.9router/db.json` under `apiKeys[0].key`) goes in the gitignored
  `.env` only — never hardcoded or committed.
- **`shared/utils/fixtures/models.ts`**: replace the three fake ids
  (`external-mcp-opus-5` etc.) with real ids 9Router actually serves — confirmed
  live via `GET /v1/models`: `high-thinking-models`, `free-models`,
  `vx/gemini-3-flash-preview` (or similar flash-tier id from that list).
  Keep the existing `label`/`description`/`icon` shape, just point `id` at
  something the router will accept. Update `defaultModelId` to match, and
  update `userSettings.defaultModelId` wherever it's seeded (register
  handler / DB default) to the new default.
- **`server/api/chat.post.ts`**: replace `pickScenario(prompt).build(...)`
  with a real call:
  - Map `messages: UIMessage[]` → OpenAI `messages` (role + flattened text
    from `parts`).
  - `POST ${baseUrl}/chat/completions` with `{ model: conv.modelId, messages, stream: true }`,
    `Authorization: Bearer ${routerApiKey}`.
  - Parse the upstream SSE (`data: {...}\n\n`, terminated by `data: [DONE]`),
    pull `choices[0].delta.content` off each event, and re-emit it through
    the **existing** data-stream protocol (`0:${JSON.stringify(delta)}\n`)
    so the frontend (`useConversationChat`, `useChat`) needs zero changes.
  - Keep the existing persistence (`messagesTable.insert` for both the user
    message already there and the assembled assistant reply) and the
    existing error chunk (`3:${JSON.stringify(...)}\n`) on upstream failure.
  - Drop the `reasoning-delta`/tool-related branches from the mock — not
    applicable to a plain-text real call, and tools are explicitly out of
    scope this round.
- Leave `MockChatTransport`, `ChatToolCall`, `ChatToolApproval`, MCP server
  settings UI in place but dormant (no code deleted) — future work can
  re-enable tool calls without re-deriving the plumbing.

## Part B — Workspace concept

Decision from user: **workspace = a per-user grouping of chats** (each user
has 1+ workspaces; each workspace holds many chats). To avoid a costly
routing rewrite (every `to="/chat/..."` link, breadcrumb, etc.), scope the
active workspace **client-side + server-enforced**, not via a nested URL
segment (`/w/:id/chat/:id`) — a workspace switcher in the sidebar header,
not a new route tree. Note this as the deliberate trade-off; nested routes
are a reasonable future step if workspaces need deep-linking.

- **DB** (`server/database/schema.ts`):
  - New `workspaces` table: `id`, `userId` (FK → `users`, cascade), `name`,
    `createdAt`, `updatedAt`.
  - `conversations.workspaceId` — new FK column, not null, references
    `workspaces.id` cascade.
  - Drizzle migration generated with the project's existing migration
    workflow (`server/database/migrations/`), including a data step: create
    one "Personal" workspace per existing user and backfill
    `conversations.workspaceId` before the `NOT NULL` constraint lands.
- **Register flow** (`server/api/auth/register.post.ts`): create a default
  "Personal" workspace for every new user, same transaction as the user row.
- **API**: `server/api/workspaces/index.{get,post}.ts`,
  `server/api/workspaces/[id].{put,delete}.ts` — same shape as the existing
  `conversations` routes (`server/api/conversations/*`), scoped to
  `session.user.id`. Deleting the last remaining workspace is rejected
  (mirror the pattern of other guarded deletes in this codebase, e.g.
  `server/utils/http-errors.ts`). `conversations/index.get.ts` /
  `index.post.ts` gain a required `workspaceId`.
- **Composable**: new `app/composables/useWorkspaces.ts`, following
  `useConversations.ts`'s pattern exactly (`useState`-backed, `loadAll`,
  `create`, `update`, `remove`), plus an `activeWorkspaceId` `useState` that
  defaults to the first workspace and persists via cookie
  (`useCookie('workspace-id')`) so it survives reload without needing a URL
  segment.
- **Sidebar** (`app/layouts/default.vue`): the current `#header` "New chat"
  button is replaced by a compact workspace switcher (current workspace name
  + chevron → `UDropdownMenu` listing workspaces, "New workspace", rename/
  delete per row) sitting above the existing time-bucketed chat list, which
  keeps its current grouping logic (`groupConversations`) but now filters by
  `activeWorkspaceId`. `useConversations().create()` passes the active
  workspace id through.
- ⌘K search (`UDashboardSearch`) gains a second group for workspaces
  alongside the existing conversations group, so palette-first navigation
  covers both.

## Part C — Visual identity (opencode-web / OpenClaw direction)

1. **Author `DESIGN.md`** at the repo root (or `.agents/knowledge/`, matching
   how plan 003 recorded "Instrument") using the `design-md` skill, informed
   by loading `frontend-design` skill guidance first for aesthetic direction.
   Cover: dark-as-default color mode, a denser type scale, mono-forward
   typography (the existing Geist Mono from plan 003 already covers labels/
   data — extend it to the chat message body itself, not just metadata),
   and the workspace-switcher + ⌘K-first interaction pattern. This
   supersedes/extends plan 003's tokens rather than starting from scratch —
   reuse the existing `signal`/`graphite` scale in `app/assets/css/main.css`
   unless the design pass has a concrete reason to change it.
2. **Dark by default**: set `colorMode: { preference: 'dark', fallback: 'dark' }`
   in `nuxt.config.ts` (currently unset, so it silently follows system
   preference — confirmed by grep, nothing in `nuxt.config.ts` sets this
   today).
3. **Chat surface typography**: switch message body font to `--font-mono`
   (or a dedicated `--font-chat` token) in the message content area
   (`ChatMessageParts.vue` / wherever `Comark` renders), keeping `--font-sans`
   for chrome (sidebar, settings, forms) — matching OpenClaw/opencode's
   terminal-flavored chat pane without going full-mono on the whole app.
4. **Sidebar density**: trim padding/row height in the conversation
   `UNavigationMenu` items and workspace switcher for a denser look;
   audit against `.agents/knowledge/conventions.md`'s semantic-color rule
   (`text-muted`, `bg-elevated`, never raw palette) — same rule plan 003
   already established, just applied to new density.
5. **Command-palette-first**: promote `UDashboardSearchButton`/⌘K as the
   primary way to switch workspaces/chats — de-emphasize (but don't remove)
   clicking through the sidebar list for a longer list of chats.
6. Re-run the plan 003 verification checklist (both themes, 375px width,
   keyboard focus visible, no raw-palette leaks) against the new surfaces.

## Verification

- `pnpm lint && pnpm typecheck && pnpm audit` — all green (typecheck script
  fixed to run `vue-tsc` directly; `nuxt typecheck` was silently exiting 0
  without surfacing real errors — see `.agents/memories/`).
- Verified via `curl` against a live `pnpm dev` instance (dev DB migrated
  with `pnpm db:migrate` — the `0002` migration existed but had not been
  applied, which 500'd registration until run):
  1. ✅ Registered a new user → `workspaces` row auto-created ("Personal").
  2. ✅ `GET /api/workspaces` returns it; `DELETE` on the sole remaining
     workspace returns `400 Cannot delete the last workspace`.
  3. ✅ Created a conversation, `POST /api/chat` with a real prompt →
     streamed back `0:"Namaku MiniMax M3"` from 9Router (model
     `free-models`) and persisted both the user and assistant messages.
  4. Model picker ids match the curated 9Router list from Part A.
  5. ✅ Server-rendered HTML on `/login` shows `preference: "dark", value: "dark"`
     with no `.env`/cookie override — dark is the true default.
  6. Chat body renders `font-mono` (`ChatMessageParts.vue`); chrome stays sans.
  7. ⌘K search groups cover workspaces and conversations (code review).
  8. ✅ Confirmed live per #2 above.

## On completion

Per `.agents/knowledge/self-improvement.md`: tick phases off here, move this
file to the Done list in `.agents/plans/README.md`, and record in
`.agents/memories/` anything a future agent could reasonably reverse — the
"workspace scoped client-side, not via URL segment" trade-off and the
9Router base URL/key convention are the two obvious candidates.
