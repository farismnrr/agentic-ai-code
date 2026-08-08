# 019 — Explicit `@search` trigger for chat mode's web search tool

## Context

Chat mode (`server/utils/langgraph-chat.ts`, `server/utils/langgraph-tools.ts`) already has a `searxng_search` tool available to the model at all times — so "cari di internet dong" already works today, because the model can freely decide to call it. **That natural-language path is existing behavior and needs no changes.**

What's missing: an explicit trigger. Typing `@search <query>` should **guarantee** the search tool runs for that turn, rather than leaving it to the model's judgement (which is what natural language already relies on, and can occasionally decline to search). This is scoped to **chat mode only** — agent mode's tools are user-toggled MCP servers with no `searxng_search` equivalent, and forcing a specific MCP tool is a different, unrelated mechanism.

No autocomplete/mention-popup UI is being built — per the request, `@search` is a literal typed prefix, detected as plain text. This keeps the feature entirely server-side: no frontend changes, no new UIMessage part types, nothing for the client to shape or restructure (consistent with the standing rule in `.agents/knowledge/conventions.md` from the last fix — response/request shaping stays server-side).

## Decisions

- **Detection is server-side, on the raw message text** — `/^@search[:,]?\s+(.+)/is` matched against the last user message's text (trimmed), inside `server/utils/langgraph-chat.ts`. No new UIMessage part type, no frontend change. This keeps the feature reachable from any client (including a future CLI/API caller) without relying on UI-side parsing.
- **The persisted/displayed message is untouched.** The user's literal text (`"@search cari tentang bnsp"`) is what gets saved to `messages.parts` and rendered in the transcript — stripping only happens in the ephemeral conversion step that builds the LangChain input, so the model receives a clean instruction (`"cari tentang bnsp"`) without ever seeing the `@search` token itself.
- **Forcing the tool call goes through the model provider's `tool_choice`, not a hand-rolled pre-call.** When `@search` is detected, the turn's `ChatOpenAI` instance is invoked with `tool_choice` forced to `searxng_search` (OpenAI-compatible `tool_choice: { type: 'function', function: { name: 'searxng_search' } }`, via `.bindTools()`) instead of the default `auto`. This keeps the tool call flowing through LangGraph's own graph execution — `on_tool_start`/`on_tool_end` stream events fire exactly as they do for a model-initiated call today, so `runLanggraphChat`'s existing UI-writer code (tool card rendering, persistence shape) needs zero changes. A hand-rolled "call the tool ourselves and splice a synthetic tool-call/tool-result pair into history" approach was considered and rejected — LangGraph only emits `on_tool_start`/`on_tool_end` for tools *it* executes as part of the graph run, not for pre-resolved history, so that approach would silently skip the tool-card UI entirely.
- **Verify `createAgent`'s `model` param accepts a pre-bound model during implementation.** `createAgent({ model, tools })` (`langchain` package) may bind its own tools onto whatever `model` it's given — if passing an already-`.bindTools()`-bound model conflicts with that (double binding), the fallback is: build a **second, forced-choice-only agent instance** for `@search` turns via `createAgent({ model: boundModel, tools: langgraphTools })` and confirm empirically it still executes correctly end-to-end. This is a one-file spike (`langgraph-chat.ts`), not a design fork — resolve it inline while implementing, don't stall on it.
- **Out of scope:** agent mode, any `@mention` autocomplete/popup UI, forcing the `curl` tool (not requested), multi-tool `@` triggers.

## Changes

Single file, no schema/API/frontend changes:

### `server/utils/langgraph-chat.ts`
1. Add a small helper (e.g. `extractForcedSearch(uiMessages: UIMessage[])`) that looks at the **last** message only, checks `role === 'user'`, and matches the `@search` regex against its concatenated text parts. Returns `{ forced: boolean, cleanedText?: string }`.
2. `convertToLangchainMessages` (or a thin wrapper around it) uses `cleanedText` in place of the raw text for that one message when `forced` is true — every other message (history) is converted exactly as today.
3. `runLanggraphChat`: when `forced` is true, build the model for this call via the existing `getLanggraphModel(modelId)` instance's `.bindTools(langgraphTools, { tool_choice: { type: 'function', function: { name: 'searxng_search' } } })`, and pass that bound model into `createAgent({ model, tools: langgraphTools })` instead of the plain model. When `forced` is false, behavior is byte-for-byte what exists today.
4. Everything downstream (`agent.streamEvents(...)`, the `on_chat_model_stream`/`on_tool_start`/`on_tool_end` writer loop, persistence) is unchanged — the forced call is just a different model/agent instance feeding the exact same loop.

## Out of scope

- Agent mode.
- Any autocomplete/popup UI for `@search` or other mentions.
- Forcing `curl` or any other tool via a similar prefix (not requested — can follow the same pattern later if asked).
- Changing how natural-language search requests work (already functions via the model's own judgement).

## Verification

- `pnpm lint`, `pnpm typecheck`.
- Manual, in a chat-mode conversation:
  - Send `@search cari tentang bnsp` → confirm the `searxng_search` tool card renders (same as a model-initiated call) and the tool actually executes (check dev server logs / response content), even repeated a few times to confirm it's not just "the model happened to decide to search."
  - Send a plain natural-language search request ("cari di internet dong soal X") → confirm unchanged existing behavior (model decides, as today).
  - Send `@search` with no query after it → confirm it's treated as normal text (no forced call, no crash).
  - Reload the conversation → confirm the transcript shows the literal `"@search cari tentang bnsp"` text the user typed, not a stripped/cleaned version.

## Status: complete

Shipped on `feat/019-p1-search-trigger`, 4 commits (`8b6417f` initial impl, `72eedcc` fix, `38bf090` tool-file split, `1c2e97e` unrelated sidebar-watcher fix).

**Deviation from the original design**, live-verified and documented in `.agents/memories/019-search-forced-tool-choice-unreliable.md`: forcing via `ChatOpenAI.withConfig({ tool_choice })` (the plan's Decision #3) does not survive `createAgent()`'s internal handling in the installed `langchain`/`@langchain/openai` versions — it neither crashes (once `MultipleToolsBoundError` was separately fixed) nor actually forces the call; the model just answers from its own knowledge with zero tool traffic. `@search` now calls `searxng_search` directly instead of asking the model to decide, then hand-writes the same UI chunks the normal path produces, then does a plain (non-agent) model call to summarize the real results. All four verification scenarios above passed via a live Playwright run against a real logged-in session (tool card rendered, `docker logs shared-searxng` showed real traffic, bare `@search` handled gracefully, literal text survived reload).
