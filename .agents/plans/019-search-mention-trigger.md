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

---

## Phase 3 — Rich-text `@search` tag via `UEditor` + `UEditorMentionMenu`

### Context

Phase 2 shipped a hand-rolled `@search` dropdown (`ChatMentionMenu.vue` + `useChatMention.ts`) against the plain `UTextarea` `UChatPrompt` uses by default. Feedback after using it:
1. Dropdown itself works now (Phase 2's regex fix confirmed).
2. Once `@search` is inserted, there's no visual marker that it's a "tag" (color/bold) — impossible with a plain `<textarea>`, which can't style substrings.
3. Cursor lands in the wrong place after inserting — a real bug in the hand-rolled `document.querySelector('textarea')`-based focus/positioning logic (grabs the *first* textarea in the whole document, and never explicitly moves the caret after `.focus()`).

Explicit direction this round: switch to Nuxt UI's rich-text `UEditor` (Tiptap-based, contenteditable) and its native `UEditorMentionMenu`, rather than patching the hand-rolled version further. Verified this is not just feasible but unusually cheap:

- **`UEditorMentionMenu`'s default mention rendering already produces exactly `@search`** as plain text — `Editor.vue`'s built-in `mention.renderText` is `` `${node.attrs.mentionSuggestionChar ?? '@'}${node.attrs.label ?? node.attrs.id}` ``, so `editor.getText()` on a doc containing a `search`-labeled mention node yields the literal string `"@search"`, byte-identical to what the backend's `extractForcedSearch()` (`server/utils/langgraph-chat.ts`) already regex-matches. **Zero backend changes, zero custom Tiptap extension needed.**
- **The colored/bold tag styling (feedback #2) already exists in Nuxt UI's own theme** — `.nuxt/ui/editor.ts` ships `[&_.mention]:text-primary [&_.mention]:font-medium` targeting the `.mention` class that `renderHTML` attaches to every mention span. This is free — no custom CSS to write.
- **The cursor-position bug (feedback #3) goes away structurally**, not by patching — Tiptap's own `insertContent`/selection handling places the cursor after inserted content natively; the entire class of "wrong textarea, wrong focus" bug is specific to the hand-rolled DOM-query approach being replaced.
- `@tiptap/extension-mention` is already a transitive dependency of `@nuxt/ui` (not in `package.json` directly, doesn't need to be added) — `Editor.vue` imports and configures it itself.

**What has to be hand-built, because `UEditor` doesn't provide it:** `UEditor` has no submit-on-enter / Shift+Enter-newline / IME-guard logic of its own (that lives entirely inside `ChatPrompt.vue`'s internals, not exported/reusable) — must be wired manually via `@keydown`. `UEditor` also has no `disabled` prop (use `editable: !disabled` instead) and its own content-model (`getText()`/`getHTML()`/`getJSON()` via the exposed Tiptap `editor` instance) is a *second*, separate model from `UChatPrompt`'s own top-level string `v-model` — the two must be bridged manually (derive plain text from the editor, assign into the same `input` ref `submit()`/`sendMessage()` already use, so nothing downstream changes).

### Decisions

- **`UChatPrompt`'s `#body` slot now renders `<UEditor>`** (via its `v-slot="{ editor }"` to get the live Tiptap instance) instead of the default `UTextarea`, with `<UEditorMentionMenu :editor="editor" :items="mentionItems" />` alongside it — no more manual `#header` overlay positioning; `UEditorMentionMenu` positions itself at the real cursor via Tiptap's suggestion plugin, which is strictly better than Phase 2's "always anchored above the textarea" approximation.
- **`mentionItems` stays a small local array** — `[{ label: 'search', description: 'Force this turn to search the web', icon: 'i-lucide-search' }]`, same one-tool-today, extensible-later shape as Phase 2, just moved to the shape `EditorMentionMenuItem` expects (`label`/`description`/`icon`, no `trigger` field needed anymore since the node's `renderText` already derives `@${label}`).
- **Shared composable, not duplicated logic across the two pages** — `useChatEditor(input: Ref<string>)` (or similar name), replacing `useChatMention.ts`, encapsulating: the exposed Tiptap `editor` ref, a watcher that derives plain text via `editor.getText()` into the passed-in `input` ref, the Enter-to-submit/Shift+Enter-newline keydown handler (respecting `settings.sendOnEnter`, mirroring `ChatPrompt.vue`'s own `handleEnter` semantics as closely as possible), and a `clearEditor()` used by `submit()`/`start()` after sending (must call the Tiptap instance's own clear command — resetting the bridged `input` ref alone isn't enough, since the next `getText()` tick would just repopulate it from the still-populated editor content). Used identically from both `[id].vue` and `index.vue`.
- **`ChatMentionMenu.vue` and `useChatMention.ts` are deleted**, not left dangling — fully superseded, not kept as a fallback.
- **No backend changes.** `server/utils/langgraph-chat.ts`'s `extractForcedSearch()` is untouched — verified the plain-text output is identical to what Phase 1 already handles.

### Changes

#### `app/composables/useChatEditor.ts` (new, replaces `useChatMention.ts`)
- Holds the exposed Tiptap `editor` ref (set from the `#body` slot's `v-slot="{ editor }"`).
- Watches editor content, writes `editor.getText()` into the caller's `input` ref.
- `handleKeydown(e)`: Enter submits (calls the `#body` slot's `submit` scoped prop) unless Shift is held or an IME composition is in progress; Shift+Enter inserts a newline (default Tiptap behavior, just don't `preventDefault`).
- `clearEditor()`: clears Tiptap content after a message is sent.
- `mentionItems`: the one-item array described above.

#### `app/pages/chat/[id].vue` and `app/pages/chat/index.vue`
1. Replace the current `#header` `ChatMentionMenu` block and `useChatMention(input)` call with `useChatEditor(input)`.
2. `<UChatPrompt>`'s default body replaced via `#body="{ submit, disabled }"` rendering `<UEditor v-slot="{ editor }" :placeholder="...' :editable="!disabled" @update:model-value="..." @keydown="handleKeydown($event, submit)"> <UEditorMentionMenu :editor="editor" :items="mentionItems" /> </UEditor>`.
3. `submit()`/`start()` call `clearEditor()` alongside the existing `input.value = ''` reset.
4. `mode === 'chat'` gating for the mention menu carries over from Phase 2 (agent mode still has no `@search`-equivalent) — likely via conditionally passing `mentionItems: []` or `v-if` around `UEditorMentionMenu` when `mode !== 'chat'`.

#### Deletions
- `app/components/ChatMentionMenu.vue`
- `app/composables/useChatMention.ts`

### Out of scope

- Any tool besides `search` in the mention list (same as Phase 2).
- Agent-mode MCP tool mentions.
- Changing the mention's rendered color/weight beyond what Nuxt UI's `.mention` theme class already provides by default.
- Rich-text formatting beyond what's needed for the mention node (no bold/italic/lists toolbar — `starterKit` stays effectively minimal/default, not expanded into a full WYSIWYG toolbar).

### Verification

- `pnpm lint`, `pnpm typecheck`.
- Manual (or Playwright), in a **chat-mode** conversation:
  - Type `@` → `UEditorMentionMenu` opens positioned at the cursor; type `se` → filters to "search"; select via click and via keyboard (Tiptap's suggestion plugin handles ↑/↓/Enter itself) → `@search` appears inline, styled distinctly (colored/bold via `.mention`), cursor lands immediately after it.
  - Continue typing a query after the tag, submit → confirm the message sent to the backend is the plain-text `"@search <query>"` string (check the persisted message / network payload) and that `extractForcedSearch()` still forces the real tool call exactly as Phase 1 verified (tool card renders, SearxNG receives traffic).
  - Shift+Enter inserts a newline instead of submitting; plain Enter submits (respecting `settings.sendOnEnter`).
  - After sending, the editor visually clears (not just the underlying `input` ref) — no leftover mention/text visible for the next message.
  - In an **agent-mode** conversation: typing `@` does not open the mention menu.

## Status: Phase 3 complete

Shipped on `feat/019-p3-rich-text-search-tag`, 2 commits (`7e22eaf` UEditor migration, `e6ded88` fix).

**Deviation from the plan's `#body` sketch**, live-verified: `@keydown` on `<UEditor>` does not register a real DOM listener — Nuxt UI's `Editor.vue` only declares `update:modelValue` as an emit, so any other listener attr falls through to ProseMirror's `editorProps.attributes` and gets stringified into an inert `onkeydown="..."` HTML attribute. Enter-to-submit silently did nothing until this was found and fixed by using Tiptap's real `editorProps.handleKeyDown(view, event)` hook instead (`:editor-props="{ handleKeyDown: (_view, event) => handleKeydown(event, promptSubmit) }"`). Also fixed along the way: `clearEditor()` in the new-chat page ran before the throwable `create()` call (typed text was lost on a failed request — now cleared only after success); autofocus was dropped when the textarea was swapped out (added `autofocus` to `<UEditor>`); and `UEditor`'s `mention` prop defaults to `true` regardless of chat mode, so Tiptap's default `@`-suggestion plugin kept tracking `@` in agent mode even with `UEditorMentionMenu` conditionally hidden — fixed with `:mention="mode === 'chat'"` to disable the extension outright outside chat mode.

All Verification scenarios above confirmed via live Playwright runs against a real logged-in session: autofocus on both pages, mention dropdown positioned at cursor with correct filtering, `@search` renders styled (`.mention` → `text-primary font-medium`, free from Nuxt UI's own theme), Enter submits and the tool card / SearxNG traffic fire exactly as Phase 1 verified, and typing `@` in an agent-mode conversation shows no dropdown (screenshot-confirmed, not just a text-match check — an earlier loose `page.locator('text=search')` assertion was a false positive from an unrelated "search" occurrence elsewhere on the page).
