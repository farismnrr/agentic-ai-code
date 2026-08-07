# ChatGPT-like AI chat UI — frontend only

## Context

The repo is currently the untouched Nuxt UI starter (one landing page). We want a ChatGPT-style app: conversation sidebar, streaming chat, settings, and MCP server management — **UI only, no backend this iteration**.

The point of doing UI first is to lock the interaction design and component structure while it's cheap to change. So the constraint that matters isn't "make it look right", it's **make the data shapes real** — if the mock layer invents its own message format, every screen gets rewritten when a backend lands.

Decisions already made: full scope including settings, all four MCP surfaces, and its own visual identity using Nuxt UI's semantic tokens (no ChatGPT pixel-chasing).

## Key decision: mock the transport, not the UI

Nuxt UI's chat components are built for the Vercel AI SDK. `UChatPromptSubmit` consumes `ChatStatus` (`ready` / `submitted` / `streaming` / `error`) and drives its own send/stop/reload affordances from it. Reimplementing that state machine by hand means rebuilding it wrong.

So: install the real `ai` + `@ai-sdk/vue`, use the real `useChat()`, and swap only the **transport** — the one seam designed for this.

```
useChat({ transport: new MockChatTransport() })   ← now
useChat({ transport: new DefaultChatTransport({ api: '/api/chat' }) })   ← later
```

`MockChatTransport` returns a `ReadableStream` of UI message chunks, emitted on a timer, so streaming, reasoning blocks, tool calls, stop, and regenerate all exercise real code paths. Adding a backend later is a one-line change in one composable, and no component changes at all.

**First implementation step is to read the `ChatTransport` interface from the installed `ai@7` types** and match it exactly — do not write the mock from memory.

## Dependencies

```sh
pnpm add ai @ai-sdk/vue @comark/nuxt
pnpm dlx nuxi module add @comark/nuxt   # if the bare add doesn't register it
```

`@comark/nuxt` renders assistant markdown incrementally as tokens arrive; plain `v-html` flickers and breaks mid-token. Versions confirmed available: `ai@7.0.56`, `@ai-sdk/vue@4.0.56`, `@comark/nuxt@0.6.1`.

Also add the Shiki dark-mode CSS block from `.agents/skills/nuxt-ui/references/layouts/chat.md` to `app/assets/css/main.css`.

## Build order

Each phase should end green (`pnpm lint && pnpm typecheck`) and visibly working at http://100.99.88.53:3333.

### 1. Data layer — types and fixtures ✅ done

- `app/types/chat.ts` — re-exports the SDK's `UIMessage`; defines `Conversation`, `McpServer`, `McpTool`, `ApprovalDecision`, `ChatModel`.
- `app/utils/fixtures/` — `models.ts`, `mcp-servers.ts` (4 servers, 10 tools, one in an error state), `conversations.ts` (3 seeds across time buckets), `replies.ts` (5 scenarios: default, reasoning, code, error, tool).
- `app/utils/mock-transport.ts` — the `ChatTransport` implementation.
- `@comark/nuxt` registered in `nuxt.config.ts`.

**Two things found by reading `ai@7`'s types that change later phases:**

1. **`simulateReadableStream` is exported from `ai`** — the SDK's own chunk-emitter with `initialDelayInMs` / `chunkDelayInMs`. No hand-rolled `setTimeout` loop. It's pull-based, so cancelling the reader (what `stop()` does) ends the stream on its own; `abortSignal` never needs threading through.
2. **Tool approval is native.** The chunk union has `tool-approval-request` / `tool-approval-response`, tool parts have `approval-requested` and `approval-responded` states, and `useChat()` exposes `addToolApprovalResponse({ id, approved, reason })`. **Phase 4's approval dialog is a view over SDK state, not a bespoke state machine** — do not invent a parallel one. `Conversation.approvals` only remembers the "always" answers so the dialog can be skipped next time.

### 2. Shell — layout and navigation

- `app/layouts/default.vue` → `UDashboardGroup` + `UDashboardSidebar` (collapsible, resizable), per the pattern in `references/layouts/dashboard.md`.
- Sidebar: "New chat" button, conversation list via `UNavigationMenu` (grouped Today / Previous 7 days), `UDashboardSearchButton`, user menu in `#footer`.
- `UDashboardSearch` wired to conversations for ⌘K.
- Keep `<UApp>` as the outermost element in `app.vue` — overlays and toasts depend on it.

### 3. Chat — the core screen

- `app/composables/useConversations.ts` — `useState()`-backed store (SSR-safe; a module-scope `ref` leaks between requests). CRUD over conversations, all in memory.
- `app/pages/index.vue` — empty state: greeting, suggested prompts, centered prompt box. First submit creates a conversation and routes to it.
- `app/pages/c/[id].vue` — the chat itself, following the component tree in `references/layouts/chat.md`:
  ```
  UDashboardPanel
  ├── #header → UDashboardNavbar (title, model selector, share/rename menu)
  ├── #body   → UContainer → UChatMessages
  │                           └── #content → UChatReasoning | UChatTool | Comark
  └── #footer → UContainer → UChatPrompt → UChatPromptSubmit
  ```
- Message actions (copy, regenerate, thumbs) via the `#actions` slot.
- Model selector in `UChatPrompt`'s `#footer` slot.

### 4. MCP surfaces

- **Tool call in message** — `UChatTool` with `variant="card"` in the `#content` slot, showing tool name, server badge, arguments, and result. This is the surface users actually see; get it right first.
- **Tool picker in prompt** — popover in `UChatPrompt`'s footer listing servers and their tools with checkboxes; selection is per-conversation state.
- **Approval dialog** — `UModal` driven by the SDK's `approval-requested` part state (see phase 1 note 2): tool name, server, arguments, and Allow once / Always allow / Deny, answered with `addToolApprovalResponse()`. Only the "always" answers go into `Conversation.approvals`, so the dialog can be auto-answered next time.
- **Server manager** — `app/pages/settings/mcp.vue`: `UTable` of servers (name, transport, status, tool count), enable/disable `USwitch`, add-server `UModal` with a `UForm`, and an expandable tool list per server.

### 5. Settings

`app/pages/settings.vue` as a parent with a `UNavigationMenu`, children:

- `general.vue` — theme (`UColorModeSelect`), language, streaming toggle
- `models.vue` — default model, temperature, system prompt / custom instructions
- `mcp.vue` — from phase 4
- `account.vue` — profile fields, static usage display

Forms use `UForm` + a Standard Schema validator (Zod or Valibot — pick one and stay with it); see `references/guidelines/forms.md`.

### 6. Polish

- Loading skeletons, empty states, `UChatShimmer` while awaiting first token.
- Mobile: sidebar collapses to `UDashboardSidebarToggle`; verify the prompt box doesn't get eaten by the mobile keyboard.
- Keyboard: ⌘K search, ⌘⇧O new chat, Esc to stop streaming.

## Conventions to hold

From `.agents/knowledge/conventions.md` — these are the ones this work will repeatedly tempt us to break:

- Semantic colors only (`text-muted`, `bg-elevated`), never `text-gray-500`.
- Brand colors stay in `app/app.config.ts`.
- No manual imports for our own components/composables.
- Read `.nuxt/ui/<component>.ts` for real slot names instead of guessing; use the `nuxt-ui` MCP for props/events.
- `/` is currently prerendered in `nuxt.config.ts` — the new `/` is interactive, so that `routeRules` entry needs revisiting.

## Verification

After each phase:

```sh
pnpm lint && pnpm typecheck
```

End-to-end, in the browser at http://100.99.88.53:3333 (localhost is bound off — see `.agents/memories/port-3333.md`):

1. Empty state → type a prompt → conversation is created and routed to.
2. Assistant reply streams token by token; stop button halts it mid-stream; regenerate replays it.
3. A prompt that triggers a tool shows the approval dialog; denying skips the call, allowing renders a `UChatTool` card with arguments and result.
4. "Always allow" suppresses the dialog for that tool on the next call in the same conversation.
5. Tool picker changes which tools the mock will invoke.
6. `/settings/mcp` — add a server, toggle it off, confirm its tools disappear from the picker.
7. Toggle dark mode on every screen; check no raw palette color leaked.
8. Narrow to mobile width; sidebar collapses, chat stays usable.

## Out of scope

No server routes, no persistence across reload, no auth, no real model calls. State lives in memory and resets on refresh — that is the intended end state for this iteration.

## On completion

Per `.agents/knowledge/self-improvement.md`: tick phases off in this file as they land, move it to the Done list in `.agents/plans/README.md` when the work ships, and record in `.agents/memories/` any decision a future agent could reasonably reverse — the mock-transport seam being the obvious one.
