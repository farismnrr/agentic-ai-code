# Plan: Context compaction for chat conversations

## Context

Every chat turn (`server/api/chat.post.ts`) sends the **entire** client-side
`messages: UIMessage[]` array straight to the model — via
`convertToModelMessages()` for agent mode, or `convertToLangchainMessages()`
inside `runLanggraphChat()` for chat mode. Nothing trims or summarizes this
history. Long conversations will eventually exceed the model's context
window and start failing.

The `models` table already tracks `contextWindow` and `maxOutputTokens` per
model (`server/database/schema.ts`), and `resolveModelConfig()`
(`server/utils/providers/index.ts`) already surfaces `contextWindow` — but
it is computed and never used. This plan wires it up: when the estimated
size of the conversation approaches the model's context window, summarize
the older portion of the conversation into a compact synthetic message and
send `[summary] + [recent messages]` to the model instead of the full
history. Full history stays in the DB/UI untouched — only what's sent to
the LLM is trimmed.

This mirrors the "auto-compact" pattern used by coding-agent tools (e.g.
external MCP client Code's own `/compact`): summarize once a threshold is crossed, keep
a pointer so it isn't redone from scratch every turn, and never split a
tool-call/tool-result pair (each `UIMessage` here already carries its tool
parts inline, so cutting on whole-message boundaries is always safe).

## Approach

### 1. Schema: track compaction state per conversation
Add to `conversations` (`server/database/schema.ts`):
- `contextSummary: text` (nullable) — the running summary text.
- `contextSummaryUpToMessageId: uuid` references `messages.id` (nullable) —
  last raw message folded into the summary; everything after this id is
  still sent verbatim.

Generate + apply migration with `npm run db:generate` / `db:migrate`
(existing convention, see `server/database/migrations/`).

### 2. Token estimation (no per-provider tokenizer)
No tokenizer exists anywhere in this codebase today for any provider
(OpenAI-compatible, Anthropic-compatible, Vertex). Adding one is out of
scope for a first cut — use a cheap, provider-agnostic heuristic:
`estimatedTokens = JSON.stringify(parts).length / 4`, summed across
messages. Good enough to trigger compaction well before a hard failure;
document it as an approximation inline.

### 3. New utility: `server/utils/context-compaction.ts`
Single function, shared by both chat and agent mode:

```ts
async function resolveMessagesForModel({
  messages,        // full UIMessage[] from the request
  conv,            // conversation row (has contextSummary / contextSummaryUpToMessageId)
  contextWindow,   // resolvedConfig.contextWindow (may be undefined -> no-op)
  maxOutputTokens,
  summarizerModel, // the same baseModel/langgraphModel already resolved for this turn
  db
}): Promise<UIMessage[]>
```

Logic:
1. If `contextWindow` is unset, return `messages` unchanged (feature is
   opt-in per model, since the field is optional today).
2. If `conv.contextSummaryUpToMessageId` is set, find that message's index
   in `messages` and build `candidate = [summaryMessage, ...messages.slice(idx+1)]`;
   otherwise `candidate = messages`.
3. Estimate tokens for `candidate`. Reserve headroom: `budget = contextWindow - maxOutputTokens - margin` (margin ~10%).
4. If under budget, return `candidate` as-is (no LLM call — this is the
   common case, must stay cheap).
5. If over budget: pick a new cutoff leaving the last ~6 messages (or
   enough to stay under a smaller "tail budget") untouched, call
   `generateText()` (from `ai`) against `summarizerModel` with a fixed
   system prompt ("Summarize the conversation so far, preserving key
   facts, decisions, file/code references, and open tasks. Be concise.")
   over `[existing summary message?, ...messages up to new cutoff]`,
   persist the new summary text + cutoff message id onto the `conversations`
   row, and return `[newSummaryMessage, ...tail]`.
6. The synthetic summary message is `{ role: 'system', parts: [{ type: 'text', text: summary }] }` — `role: 'system'` is already handled by both `convertToLangchainMessages` (→ `SystemMessage`) and `convertToModelMessages` (AI SDK's own system role).

### 4. Wire into `chat.post.ts`
After `resolvedConfig` is computed (line ~51) and before either mode branch,
call `resolveMessagesForModel(...)` once and use its result in place of the
raw `messages` in both:
- chat mode: `runLanggraphChat({ uiMessages: resolvedMessages, ... })`
- agent mode: `convertToModelMessages(resolvedMessages, { tools })`

Note: the *persisted* user/assistant messages (`messagesTable` inserts)
keep using the original, untouched `messages`/`parts` — compaction only
affects what's sent to the model this turn, never what's stored or
rendered.

### 5. Reuse the already-resolved model as summarizer
No new provider plumbing needed — `chat.post.ts` already resolves
`baseModel` (agent mode) / `langgraphModel` (chat mode) before the branch.
Pass that same model into `resolveMessagesForModel` for the summarization
call, so no extra provider config or API key handling is required.

## Files touched
- `server/database/schema.ts` — add `contextSummary`, `contextSummaryUpToMessageId` to `conversations`.
- `server/database/migrations/` — new generated migration.
- `server/utils/context-compaction.ts` — new file, the estimator + `resolveMessagesForModel`.
- `server/api/chat.post.ts` — call the new util before both mode branches.
- `shared/types/chat.ts` — extend `Conversation` type with the two new optional fields if it's used for API responses.

## Verification
- `npm run db:generate` then `db:migrate` to confirm the migration applies cleanly.
- Manual: open a conversation, keep chatting until estimated size crosses ~75% of the configured model's `contextWindow` (or temporarily set a small `contextWindow` on a test model to trigger compaction quickly), confirm:
  - A summarization call fires (add a `logger.info('[compaction] ...')` line to observe it in dev logs).
  - The conversation continues to respond coherently to something referenced only in the summarized (older) portion.
  - The DB `messages` table still has full untouched history (compaction never deletes rows).
  - Both `chat` mode and `agent` mode conversations trigger compaction correctly.
