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

## Status: Phase 1 complete

Shipped on `feat/019-p1-search-trigger`, 4 commits (`8b6417f` initial impl, `72eedcc` fix, `38bf090` tool-file split, `1c2e97e` unrelated sidebar-watcher fix).

**Deviation from the original design**, live-verified and documented in `.agents/memories/019-search-forced-tool-choice-unreliable.md`: forcing via `ChatOpenAI.withConfig({ tool_choice })` (the plan's Decision #3) does not survive `createAgent()`'s internal handling in the installed `langchain`/`@langchain/openai` versions — it neither crashes (once `MultipleToolsBoundError` was separately fixed) nor actually forces the call; the model just answers from its own knowledge with zero tool traffic. `@search` now calls `searxng_search` directly instead of asking the model to decide, then hand-writes the same UI chunks the normal path produces, then does a plain (non-agent) model call to summarize the real results. All four verification scenarios above passed via a live Playwright run against a real logged-in session (tool card rendered, `docker logs shared-searxng` showed real traffic, bare `@search` handled gracefully, literal text survived reload).

---

## Phase 2 — `@` mention dropdown for chat-mode tool triggers

### Context

Phase 1 shipped `@search <query>` as a literal typed prefix, deliberately with **no autocomplete UI** ("per the request, `@search` is a literal typed prefix, detected as plain text"). Now explicitly requested: typing `@` in the chat input should open a dropdown so `@search` can be picked/tagged, not just remembered and typed blind. This stays in the same plan file rather than a new plan number — plan 020 (tools-as-local-packages) is unrelated and untouched.

**Nuxt UI 4 doesn't have a drop-in answer here.** Its one "type `@` to open a list" component, `UEditorMentionMenu`, only attaches to the Tiptap-based `UEditor` (contenteditable, ProseMirror suggestion plugin) — `UChatPrompt` (used in both `app/pages/chat/[id].vue` and `app/pages/chat/index.vue`) renders a plain `UTextarea` bound to a `ref<string>`. Swapping the textarea for a full rich-text editor just to get one mention popup is a much bigger change than this warrants. `UChatPrompt` also has no caret-position-aware overlay slot — its `#header` slot is the only one that renders inside `root` (which has `relative` positioning) above the textarea, everything else (`#footer`) renders below.

Given there's exactly **one** forceable tool today (`search`), this doesn't need real caret-tracking or a generic mention framework — a small, hand-built dropdown anchored above the textarea via `UChatPrompt`'s existing `#header` slot is enough, and cheap to extend later if more forceable tools are added (matches Phase 1's original "can follow the same pattern later if asked" for `curl`).

### Decisions

- **New shared component, not duplicated logic** — `app/components/ChatMentionMenu.vue`, used from both `[id].vue` and `index.vue` (both already compose `UChatPrompt` inline; `ChatToolPicker.vue` is the existing precedent for a small chat-input-adjacent component in this codebase).
- **Trigger detection**: watch the `input` ref for a trailing `@<partial>` token — `/(?:^|\s)@(\w*)$/` matched against the current value's end (simplification: good enough for a single-line-growing textarea where users aren't routinely editing mid-text before more typing). When matched, the menu opens with `partial` as the filter.
- **Tool list is a small local array**, extensible later — `[{ trigger: 'search', label: '@search', description: 'Force this turn to search the web', icon: 'i-lucide-search' }]` — filtered by `partial`. Only rendered/active when `mode === 'chat'` (agent mode has no `@search`-equivalent; matches Phase 1's chat-mode-only scoping).
- **Positioning**: rendered into `UChatPrompt`'s `#header` slot, `absolute bottom-full left-0` within `root`'s relative box — sits right above the textarea. Not true caret-tracking, but correct and simple for a single-line-growing input with one candidate item; revisit only if a future multi-line-editing use case demands real caret coordinates.
- **Selection replaces the trailing `@partial` token** with `@search ` (trailing space) in the `input` ref and refocuses the textarea — keyboard (↑/↓ to move, Enter/Tab to select, Escape to close) and mouse-click both supported, since "dropdown tools... biar bisa ke tag" implies real interactive selection, not just a visual hint.
- **No change to the backend.** `server/utils/langgraph-chat.ts`'s `@search` detection (already shipped in Phase 1) is unaffected — the dropdown only changes how the same literal `@search <query>` text gets typed.

### Changes

#### `app/components/ChatMentionMenu.vue` (new)
- Props: `open: boolean`, `filter: string`, `items` (the tool list, or hardcode it here since there's only one for now).
- Emits `select(trigger: string)`, `close()`.
- Keyboard nav (↑/↓/Enter/Tab/Escape) — needs `v-model:highlighted` or an internal `ref` index.
- Simple list markup (icon + label + description per row), similar visual weight to `ChatToolPicker.vue`'s existing popover rows but not wrapped in `UPopover` (it's driven by the trigger regex, not hover/click-to-open).

#### `app/pages/chat/[id].vue` and `app/pages/chat/index.vue`
1. Add `mentionOpen`/`mentionFilter` computed or refs derived from `input.value` via the trigger regex.
2. Add `<template #header>` to the existing `<UChatPrompt>` (currently unused in both files) rendering `<ChatMentionMenu :open="mentionOpen" :filter="mentionFilter" @select="onMentionSelect" @close="mentionOpen = false" v-if="mode === 'chat'" />`.
3. `onMentionSelect(trigger)`: replace the trailing `@partial` in `input.value` with `@${trigger} `, close the menu, refocus the textarea (`UChatPrompt` likely exposes the textarea ref, or focus via a template ref — confirm during implementation).
4. Existing `submit()`/`start()` handlers, model bindings, and `ChatToolPicker`/`mode` selects are untouched.

### Out of scope

- Real caret-position tracking (dropdown always anchors above the textarea, not at the cursor mid-text).
- Any tool besides `search` (list is trivially extensible later, not populated now).
- Agent-mode MCP tool mentions (different mechanism, different scope — not requested).
- Changing `UChatPrompt` to the rich-text `UEditor`/`UEditorMentionMenu` path.

### Verification

- `pnpm lint`, `pnpm typecheck`.
- Manual (or Playwright), in a **chat-mode** conversation: type `@` → dropdown appears with "search"; type `@se` → still matches (filter); type `@xyz` → no match, dropdown closes/empty; select via click and via Enter after arrowing → `@search ` inserted correctly, cursor after it, dropdown closes.
- In an **agent-mode** conversation: typing `@` does **not** open the dropdown (no forceable tools defined for that mode).
- Confirm the resulting `@search <query>` message still round-trips through the existing backend logic exactly as shipped in Phase 1 (no regression — this phase only changes how the text gets typed).
