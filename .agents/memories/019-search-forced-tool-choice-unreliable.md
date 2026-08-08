---
name: 019-search-forced-tool-choice-unreliable
description: ChatOpenAI.withConfig({ tool_choice }) doesn't reliably force a tool call through createAgent() in the installed langchain/@langchain/openai versions — @search now calls the tool directly instead of relying on provider-level forcing.
metadata:
  type: project
---

Plan [[019-search-mention-trigger]]'s original design forced the `searxng_search` tool call via `baseModel.withConfig({ tool_choice: { type: 'function', function: { name: 'searxng_search' } } })`, passed into `createAgent({ model, tools })`. This avoided `createAgent`'s `MultipleToolsBoundError` (which fires when the model handed to it already has a `tools` array bound — confirmed by reading `validateLLMHasNoBoundTools` in `node_modules/langchain/dist/agents/utils.cjs`, which only checks for a bound `tools` array, not `tool_choice`).

**Live-verified finding: it doesn't actually force anything.** Runtime introspection (`model.constructor.name`) showed the `.withConfig()` result stayed a plain `ChatOpenAI` instance rather than the `RunnableBinding` wrapper `Runnable.withConfig()`'s source implies it should produce. Sending `@search cari tentang bnsp` against the real 9Router-backed model answered from the model's own pretrained knowledge — no tool card rendered, and `docker logs shared-searxng` showed zero traffic for that turn. Whatever `.withConfig()` sets doesn't survive into the actual provider request in this codepath.

**The fix that actually works:** when `@search` is detected, call `searxngSearchTool.invoke({ query })` **directly** in `server/utils/langgraph-chat.ts` — no model involved in deciding whether to search — then hand-write the `tool-input-available`/`tool-output-available` UI-stream chunks in the exact same shape the normal `on_tool_start`/`on_tool_end` path produces, then call `baseModel.stream(...)` (a plain model call, not the `createAgent` ReAct loop) with the real search results appended as context so the model summarizes grounded, real data. This sidesteps `createAgent`/`tool_choice` entirely for the forced path; the non-forced (natural-language) path is untouched and still goes through the normal `createAgent` ReAct loop.

**How to apply:** don't reach for provider-level `tool_choice` forcing again in this codebase without a fresh, live (not just source-read) verification against the exact installed `langchain`/`@langchain/openai`/model-router combination — the library's own source comments and type signatures did not predict this behavior. If a future feature needs "guarantee tool X runs this turn," default to the direct-call-plus-manual-UI-chunks pattern established here rather than re-attempting `.bindTools()`/`.withConfig()` forcing.
