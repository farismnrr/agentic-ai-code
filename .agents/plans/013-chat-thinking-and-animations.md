# 013 — Real reasoning + chat animation polish

## Why

Chat currently has the plumbing for a ChatGPT-style "thinking" block
(`UChatReasoning` in `ChatMessageParts.vue`) and streaming markdown
(`Markdown :streaming`), but:

- No real reasoning ever reaches it — `server/api/chat.post.ts`'s
  `streamText()` call requests no reasoning output, so `UChatReasoning`
  is only ever exercised by the landing-page mock scenarios
  (`shared/utils/fixtures/replies.ts`).
- Message entry, the "Thinking…" indicator, and reasoning block open/close
  have no motion — everything pops in instantly.

Two independent halves: wire real reasoning through the router model, and
add the missing motion. Neither blocks the other; do reasoning first since
it's the riskier unknown.

## Phase 1 — Real reasoning from the model

**Unknown to resolve first:** whether 9Router (`server/utils/router-model.ts`,
an `openai-compatible` provider pointed at an internal proxy) forwards
reasoning content at all for `high-thinking-models`, and in what shape —
a structured `reasoning_content` field, or inline `<think>...</think>` tags
in the text (common for OpenAI-compatible reasoning-model proxies).

1. Add a temporary debug script (or a one-off `console.log` on
   `result.stream` chunks) hitting `high-thinking-models` directly, prompt
   with something that forces visible reasoning ("why does X work"), and
   inspect the raw stream chunks. Delete the script once done — this is a
   one-time investigation, not a lasting tool.
2. Branch on what's found:
   - **Structured reasoning** (provider emits its own `reasoning-delta`
     chunks) → nothing extra needed beyond making sure `streamText` doesn't
     suppress it; verify `toUIMessageStream` passes it through as-is.
   - **`<think>` tags in plain text** (the likely case for a proxy) → wrap
     the model with `wrapLanguageModel({ model, middleware:
     extractReasoningMiddleware({ tagName: 'think' }) })` from the `ai`
     package, which splits tagged text into real `reasoning-*` UI chunks.
   - **No reasoning available at all** → stop here, tell the user, fall
     back to Phase 2 only.
3. Add a `supportsReasoning?: boolean` flag to `ChatModel`
   (`shared/types/chat.ts`) and set it on the `high-thinking-models` entry
   in `shared/utils/models.ts` — this is what decides whether
   `chat.post.ts` wraps the model, so free/flash models aren't taxed with
   middleware they'll never use.
4. In `server/api/chat.post.ts`, look up the model's `supportsReasoning`
   flag and conditionally wrap `getRouterModel(...)` before passing it to
   `streamText`.
5. Manually verify in the running app: pick "High Thinking" as the model,
   ask a reasoning-shaped question, confirm the reasoning block streams in
   before the answer, and that switching to Flash/Free models shows no
   reasoning block (no regression there).

## Phase 2 — Motion pass (ChatGPT-style feel)

Scope: `app/pages/chat/[id].vue`, `app/components/ChatMessageParts.vue`,
and Nuxt UI's `:ui` slot-class overrides — no new dependencies, use Nuxt UI
4's existing transition/animation primitives and Tailwind's built-in
transition utilities per `.agents/knowledge/nuxt-way.md`.

1. **Message entry** — new messages (both the user's sent bubble and the
   assistant's first-token appearance) should fade/slide in rather than
   snap. Check what `UChatMessages` already exposes for this (`:ui` message
   wrapper class, or a built-in transition) before reaching for a custom
   `<TransitionGroup>` — prefer the component's own hook if one exists.
2. **Streaming text** — `Markdown :streaming` already renders incrementally
   as `text-delta` chunks arrive; check whether it needs a per-token fade-in
   (a subtle opacity transition on newly-appended text) to read as "typing"
   rather than "chunks appearing." Keep it cheap — no per-character DOM
   churn.
3. **Thinking indicator** — replace/extend the static `UChatShimmer`
   "Thinking…" with a small looping animation (pulsing dots or shimmer
   sweep) consistent with Nuxt UI's existing shimmer utility rather than a
   hand-rolled one.
4. **Reasoning block** — `UChatReasoning` likely already has open/close
   affordance; add a smooth height transition if it snaps open/closed, and
   make sure the streaming reasoning text animates in the same way as the
   final answer for consistency.
5. Verify at each step in the actual dev server (not just visually
   plausible code) — motion bugs (layout jump, jank on long messages) only
   show up when you watch it stream.

## Out of scope

- No new animation library — Nuxt UI/Tailwind only.
- No change to the mock/demo scenarios on the landing page
  (`shared/utils/fixtures/replies.ts`) — they already exercise reasoning
  UI and should keep working unmodified as a fallback demo path.
- No reasoning-effort user control (e.g. a slider) — out of scope unless
  requested later.
