# 018 — Chat mode via LangGraph (curl + SearxNG search)

## Context

Chat currently has one execution path: `server/api/chat.post.ts` runs `ai@7`'s `streamText` tool loop against whatever MCP servers/tools a conversation has enabled (`server/utils/mcp-tools.ts`). We're splitting this into two conversation **modes**:

- **`chat`** — a lightweight mode with exactly two always-on tools: `curl` (fetch a URL) and `searxng_search` (web search via the locally-running SearxNG container, `shared-searxng` on `127.0.0.1:8888`, confirmed reachable with `format=json` returning 200). No MCP server picker, no read/file access.
- **`agent`** — today's existing MCP-tool-loop behavior, unchanged. Full design deferred ("belakangan, satu satu").

Per explicit direction, `chat` mode's tool-calling is built on **LangGraph.js** rather than extending the existing `ai@7` loop — not because chat mode needs a graph's branching (it doesn't), but so the orchestration engine agent mode will need later already exists and is proven out now, instead of introducing it later as a second migration.

The frontend (`useChat` from `@ai-sdk/vue`, `app/composables/useConversationChat.ts`) only understands the AI SDK's UI-message-stream wire protocol, regardless of what runs it server-side — so the LangGraph path must still terminate in `createUIMessageStreamResponse`/`toUIMessageStream`-compatible output. `ai`'s `createUIMessageStream({ execute: ({ writer }) => ... })` is the documented bridge for driving that protocol from a non-`ai`-sdk source.

## Decisions

- **`mode` is a per-conversation column** (`conversations.mode`, `text`, `$type<'chat' | 'agent'>()`, `notNull`, DB default `'agent'`) — same shape as `modelId`/`reasoningEffort`. DB default is `'agent'` so existing rows (which may already have `enabledToolIds` configured) keep behaving exactly as today after migration. The **new-conversation UI form** independently defaults its `mode` ref to `'chat'` (mirroring how `reasoningEffort`'s UI default and DB default aren't the same concern).
- **Agent mode's code path is untouched.** `chat.post.ts` branches on `conv.mode`; the existing `streamText` + `buildMcpTools` block only moves, it doesn't change.
- **Tool schemas use `zod`**, not `valibot` — LangChain/LangGraph's `tool()` is built around zod schemas; this is the idiomatic way to define tools in that ecosystem, so we add `zod` as a new runtime dep rather than fighting the library. `valibot` stays the standard for all Nuxt-side (API route body) validation, unchanged.
- **SSRF guard gets extracted and shared**, not duplicated. `assertSafeMcpUrl`'s logic in `server/utils/mcp-client.ts:43-59` (DNS-resolved, re-checked per call, blocks loopback/RFC1918/link-local/metadata/IPv6 equivalents) is the same protection a `curl` tool needs, verbatim — a `curl` tool is a second "connect to a user/model-supplied URL" path, and `012-mcp-outbound-tool-loop.md` is explicit that anything reaching that class of code must go through the one guard, not a second copy. Pulled into `server/utils/ssrf-guard.ts` as an exported `assertSafeUrl(url: URL, context: string)`; `mcp-client.ts` is updated to import it (pure refactor, no behavior change).
- **New runtime config `searxngBaseUrl`** (`NUXT_SEARXNG_BASE_URL`, default `http://127.0.0.1:8888`) — not hardcoded, following `routerBaseUrl`'s pattern in `nuxt.config.ts`/`router-model.ts`.
- **Model access for LangGraph** goes through the same 9Router endpoint as today, via `@langchain/openai`'s `ChatOpenAI` pointed at `config.routerBaseUrl`/`routerApiKey` (9Router is OpenAI-compatible — same reason `router-model.ts` uses `@ai-sdk/openai-compatible` today) — not a second/different backend.
- **RFC 9457 error shape stays consistent**: any new validation error (e.g. invalid `mode` value) uses the existing `badRequest`/`unprocessable` helpers in `server/utils/http-errors.ts`, not raw `createError`.
- **Out of scope, flagged separately, not fixed here**: `server/utils/workspaces.ts`, `fs-browse.ts`, and `api-key.ts` throw raw `createError({ statusCode, statusMessage })` instead of the RFC 9457 `problem()` shape — a pre-existing inconsistency, unrelated to this feature, left alone.

## Changes

### Phase 1 — Schema & types
1. `server/database/schema.ts`: add `mode: text('mode').$type<'chat' | 'agent'>().notNull().default('agent')` to `conversations` (near `reasoningEffort`, `:107` area). Generate migration (`pnpm drizzle-kit generate` or project's existing migration command — check `package.json` scripts).
2. `shared/types/chat.ts`: add `mode: 'chat' | 'agent'` to the `Conversation` interface (`:58-71`), same shape as the existing `reasoningEffort?` field but required.

### Phase 2 — API routes
1. `server/api/conversations/index.post.ts`: add `mode: picklist(['chat', 'agent'])` (required, since the UI always sends one) to `createSchema` (`:4-9`); include in the `.values({...})` insert and the response-shaping object (`:34-44`).
2. `server/api/conversations/[id].put.ts`: add `mode: optional(picklist(['chat', 'agent']))` to `updateSchema` (`:5-11`); it flows through via the existing `...body` spread in `db.update(...).set({...body, updatedAt})` and needs adding to the response object (`:36-45`).

### Phase 3 — Frontend mode selector
Follow the `reasoningEffort` pattern exactly (`app/pages/chat/index.vue:35-46,146-152`, `app/pages/chat/[id].vue:74-78`, `app/composables/useConversations.ts:59,80`):
1. `app/pages/chat/index.vue`: `mode = ref<'chat' | 'agent'>('chat')`, a `USelect`/segmented-control (e.g. `UButtonGroup`) in the same footer row as the model/workspace pickers, passed into `create({ ..., mode })`.
2. `app/pages/chat/[id].vue`: `mode` as a computed get/set wired to `update(conversation.value.id, { mode: value })`, same as `modelId`/`reasoningEffort` (`:67-78`). When `mode === 'chat'`, hide `ChatToolPicker` (`:358`) — chat mode's tools are fixed (curl + search), not user-configurable, and MCP servers don't apply.
3. `app/composables/useConversations.ts`: `create()` sends `mode` in the POST body (`:59` area); `update()`'s `apiPatch` whitelist gets an `if (patch.mode !== undefined) apiPatch.mode = patch.mode` line (`:80` area).

### Phase 4 — LangGraph engine (chat mode only)
1. Add deps: `pnpm add @langchain/langgraph @langchain/core @langchain/openai zod`.
2. `server/utils/ssrf-guard.ts` (new): extract `assertSafeMcpUrl`'s body out of `mcp-client.ts:43-59` into an exported `assertSafeUrl(url: URL, context: string)`. Update `mcp-client.ts` to import and call it — no behavior change.
3. `server/utils/langgraph-model.ts` (new): builds a `ChatOpenAI` instance pointed at `config.routerBaseUrl` / `config.routerApiKey`, given a `modelId`, mirroring `router-model.ts`'s intent for the LangChain side.
4. `server/utils/langgraph-tools.ts` (new):
   - `curl` tool (zod schema: `url`, optional `method`/`headers`/`body`) — calls `assertSafeUrl()` before fetching, returns status/headers/body (capped size).
   - `searxng_search` tool (zod schema: `query`) — `GET ${config.searxngBaseUrl}/search?q=...&format=json`, returns top N results (title/url/snippet).
5. `server/utils/langgraph-chat.ts` (new): builds a LangGraph prebuilt ReAct agent (`createReactAgent` from `@langchain/langgraph/prebuilt`) with the model from (3) and tools from (4). Exposes a function that takes the conversation's `UIMessage[]`, runs the agent, and drives an `ai` `createUIMessageStream({ execute: ({ writer }) => ... })` writer from the agent's stream events (text deltas, tool-call-start/result) — the exact writer calls to confirm against the installed `ai@7` API during implementation, but the shape mirrors what `toUIMessageStream` already produces for the `agent`-mode path so `ChatMessages.vue`/`ChatToolApproval.vue` don't need frontend changes for tool-call rendering.
6. `server/api/chat.post.ts`: branch near `:45` — if `conv.mode === 'chat'`, skip `buildMcpTools`/`streamText` entirely and call the new LangGraph path from (5), reusing the existing user-message insert (`:31-37`) and the existing `onEnd`-equivalent persistence logic (insert/update assistant message, `:94-116`) unchanged in shape. If `conv.mode === 'agent'`, run exactly today's code, untouched.

## Out of scope

- Agent mode's own design (MCP tools staying on `ai@7` for now; whether/when they migrate to LangGraph is a separate future plan).
- Rate limiting SearxNG calls (confirmed: local-only container, not needed).
- The pre-existing raw-`createError` RFC 9457 inconsistency in `workspaces.ts`/`fs-browse.ts`/`api-key.ts` (flagged above, separate cleanup).

## Verification

- `pnpm lint`, `pnpm typecheck`, `pnpm audit` (zero) — gate as usual.
- `pnpm build && pnpm preview` — verify per `.agents/knowledge/project.md`'s note against relying on `pnpm dev`.
- Manual, in-browser:
  - Create a new conversation in `chat` mode, ask it to search something → confirm `searxng_search` fires and results come back, streamed and rendered like existing tool calls.
  - Ask it to fetch a URL → confirm `curl` tool works for a normal public URL.
  - Confirm the `curl` tool is blocked (via `assertSafeUrl`) against `http://169.254.169.254/`, `http://127.0.0.1/`, and a private RFC1918 address.
  - Create/open an `agent`-mode conversation with an MCP server enabled → confirm it behaves exactly as before (unchanged code path), including tool-approval flow.
  - Confirm assistant messages persist correctly for both modes after a page reload.
- Run `/security-review` before merge — this adds a second user/model-triggered outbound-URL-fetch path (`curl` tool), same risk class as the MCP SSRF finding in `012-mcp-outbound-tool-loop.md`.

## On completion
- [x] Update this plan file with final status per phase.
- [x] Write `.agents/memories/018-...md` for any live-verified finding (e.g. LangGraph→UI-stream bridging quirks, if any turn up), and add it to `memories/README.md`'s index.
- [x] Follow `.agents/knowledge/git.md` for branch/commit/PR — never commit to `main`/`dev` directly, never commit unasked.
