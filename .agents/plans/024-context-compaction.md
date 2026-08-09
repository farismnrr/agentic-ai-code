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
Claude Code's own `/compact`): summarize once a threshold is crossed, keep
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

---

## Phase 2 (follow-up): replace the token heuristic with real usage accounting

**Status: Phase 1 and Phase 2 are implemented.** (`server/utils/context-compaction.ts`,
schema columns, wired into `chat.post.ts`). It uses
`estimateTokens = JSON.stringify(obj).length / 4` for every budget check —
a rough approximation (worse for non-English text and for dense
JSON/code, which is most of what tool calls carry). The AI SDK already
returns *real* token usage per turn; this phase swaps the heuristic for
that wherever it's available and keeps the heuristic only as a fallback
for the still-unmeasured tail.

### Why this is safe to bolt on, not a rewrite
`resolveMessagesForModel`'s structure doesn't change — only what
`estimateTokens(candidate)` is fed. The insight: we don't need to
re-measure the *whole* candidate array from scratch each turn. The
previous assistant turn's `streamText`/`generateText` call already told us,
exactly, how many tokens the model was given as input for that call. That
number is a real baseline; only messages appended *after* that turn (the
new user message, and anything else not yet sent to a model) are
unmeasured and still need the char/4 estimate.

### 1. Schema: persist real usage per message
Add to `messages` (`server/database/schema.ts`):
- `totalTokens: integer` (nullable) — `usage.totalTokens` from the AI SDK
  (agent mode) or the LangChain equivalent (chat mode), i.e. the exact
  input+output token count the provider billed for the turn that produced
  this assistant message. Null for user messages and for any assistant
  message where usage wasn't available (heuristic fallback still applies).

Migration via `npm run db:generate` / `db:migrate`, same as Phase 1.

### 2. Capture usage where it already exists
- **Agent mode** (`chat.post.ts`, `streamText` branch): `result.usage` is a
  promise on the `streamText` return value, resolving to
  `{ inputTokens, outputTokens, totalTokens }` once the stream finishes.
  `persistAssistantMessage` already runs at stream end (`onEnd` in
  `toUIMessageStream`) — thread `await result.usage` through to it and
  store `totalTokens` on the inserted/updated row.
- **Chat mode** (`server/utils/langgraph-chat.ts`): LangChain
  `AIMessageChunk`s carry `.usage_metadata` (`input_tokens`,
  `output_tokens`, `total_tokens`), and chunk `.concat()` merges it
  automatically — verify against the installed `@langchain/core` version,
  since API shape has moved between versions. Accumulate it the same way
  `currentText` is already accumulated in the streaming loop, and pass the
  final total through `onEnd` alongside `parts`. If a given call genuinely
  exposes no usage metadata, leave `totalTokens` null and let the fallback
  in step 3 cover it — don't block the feature on chat-mode parity.

### 3. Use it in `resolveMessagesForModel`
Replace the blanket `estimateTokens(candidate)` call with:
1. Walk `candidate` from the end backwards to find the most recent
   assistant message with a non-null `totalTokens` — call its value
   `measuredBaseline` and its index `measuredIdx`.
2. `projectedTokens = measuredBaseline + estimateTokens(candidate.slice(measuredIdx + 1))`
   — real count for everything up through the last completed turn, heuristic
   only for the handful of messages since (typically just the latest user
   message, sometimes an in-flight tool-approval continuation).
3. If no message in `candidate` has a measured `totalTokens` (fresh
   conversation, or usage capture failed), fall back to
   `estimateTokens(candidate)` exactly as today — Phase 1's behavior is the
   floor, never worse than before.
4. Compare `projectedTokens` (or the fallback) against `budget` as today.

This keeps the margin/budget math (`contextWindow - maxOutputTokens - 10%`)
untouched — only the numerator gets more accurate.

### Files touched (Phase 2)
- `server/database/schema.ts` — add `messages.totalTokens`.
- `server/database/migrations/` — new generated migration.
- `server/api/chat.post.ts` — capture `result.usage` and thread it into
  `persistAssistantMessage`.
- `server/utils/langgraph-chat.ts` — accumulate `usage_metadata` from the
  LangChain stream, pass through `onEnd`.
- `server/utils/context-compaction.ts` — swap flat heuristic for the
  measured-baseline + heuristic-delta calculation above.

### Verification (Phase 2)
- After a normal turn, confirm the new assistant row has a non-null
  `totalTokens` matching the order of magnitude you'd expect for that
  conversation (cross-check against provider dashboard/logs if available).
- Force a conversation with heavy non-English or code-heavy content and
  confirm compaction now triggers at a materially different point than
  Phase 1's char/4 estimate would have (log both `projectedTokens` and
  what the old heuristic would've produced, temporarily, to compare).
- Confirm the no-usage-available fallback path still works (e.g. stub a
  provider response with `usage: undefined`) — compaction must not throw
  or silently stop triggering when usage is missing.

---

## Phase 2 review fixes (implemented)

Two gaps found reviewing the Phase 2 commit, both fixed:

1. **`@search`-forced replies in chat mode never captured usage.** The
   forced-search branch in `langgraph-chat.ts` streams via
   `baseModel.stream(inputMessages)` directly, separate from the
   `agent.streamEvents` loop where `usage_metadata` capture was added —
   so every `@search:`-triggered turn persisted `totalTokens: null` and
   compaction silently fell back to the heuristic for that turn. Fixed by
   reading `chunk.usage_metadata?.total_tokens` inside that branch's own
   streaming loop too.

2. **Unconditional extra DB query on every chat turn.** The original
   Phase 2 implementation queried `messages` via `inArray` on every call
   to `resolveMessagesForModel` (even when nowhere near budget) to look up
   `totalTokens` for the candidate messages. Replaced with a cache:
   `conversations` gained `lastMeasuredTokens` / `lastMeasuredMessageId`,
   written once per turn in `chat.post.ts`'s `cacheLastMeasuredTokens`
   (alongside the existing message insert/update), right after a real
   usage number is known. `resolveMessagesForModel` now reads the baseline
   straight off the already-loaded `conv` row — no query at all in the
   common case.

---

## Repeated-compaction hardening (implemented)

Two gaps specific to a conversation that compacts many times in a row:

1. **No verification a compaction pass actually lands under budget.**
   `resolveMessagesForModel` ran a single summarize pass and returned
   whatever tail size resulted, with no check that `[summary, ...tail]`
   was actually back under budget — a pathological case (one huge tool
   output sitting in the kept tail) could ship an over-budget request
   anyway. Added `truncatePartsIfNeeded()`: after summarizing, if
   `[newSummaryMessage, ...tail]` is still over budget, clip oversized
   `text`/`input`/`output` string content within tail parts (oldest tail
   message first, the single most recent message left untouched) until
   back under budget or all messages are checked. Logs a warning if still
   over budget after that — a deliberate best-effort last resort, not a
   hard guarantee.
2. **Lossy drift across many resummarization rounds.** Each compaction
   feeds the previous summary back into the next summarization call, so
   over many rounds specific facts can gradually erode through repeated
   compression. The summarization system prompt now explicitly instructs
   the model to carry forward every concrete fact/decision/file
   path/identifier/number already present in the existing summary
   verbatim, rather than just "be concise" — reduces (doesn't eliminate)
   drift. Full raw history in the DB remains the ultimate source of truth
   regardless.

---

## Phase 3 (implemented): stop sending full chat history on every request

### Why
Phases 1-2 fixed what's sent *to the LLM*. This phase fixes the other half:
what's sent *client → server*. `chat.post.ts` currently trusts the
client-supplied `messages: UIMessage[]` request body as the full
conversation history on every turn, and `useChat` (`useConversationChat.ts`)
has no `transport`/`prepareSendMessagesRequest` override, so it defaults to
sending its entire local `messages` state every time. Compaction never
touches this — it only trims the in-memory array built inside
`chat.post.ts` after the request already arrived. At high message counts
this means unbounded JSON payload growth, `readBody()` parse cost, and
`O(n)` scans (`messages.findIndex(...)`) on every single turn regardless of
how well-compacted the model-facing context is.

The `messages` table is already the durable, complete source of truth —
confirmed via `server/api/conversations/[id].get.ts` →
`listConversationMessages()` (`server/utils/messages.ts`), which already
loads a conversation's full history straight from the DB, not from any
client cache. The client array in `chat.post.ts` is redundant.

### Three ways `useChat` re-sends messages (verified against `node_modules/ai/dist/index.js`)
1. **`submit-message`** (normal new turn) — last client message is a new
   user message not yet in the DB.
2. **`regenerate-message`** (`regenerate()` in `app/pages/chat/[id].vue:291,403`,
   always called with no `messageId` in this UI → always targets the last
   assistant message) — `this.regenerate` trims the SDK's own
   `state.messages` client-side to end right after the preceding user
   message before sending. That last message is **already in the DB,
   unchanged** — today's code (`if (lastMsg.role === 'user') insert`)
   actually re-inserts it as a duplicate row every time regenerate is
   clicked, silently. This phase fixes that as a side effect. The model
   also must not see the stale assistant answer being replaced.
3. **Tool-approval resume** — the existing in-flight assistant message is
   re-sent with an appended approval part (`chat.post.ts:62-64`). Not a
   new message; its content differs from what's currently in the DB.

### Approach
**Client** (`app/composables/useConversationChat.ts`): replace the bare
`api: '/api/chat'` string with an explicit `DefaultChatTransport` (import
from `'ai'`) using `prepareSendMessagesRequest`:
```ts
transport: new DefaultChatTransport({
  api: '/api/chat',
  prepareSendMessagesRequest: ({ id, messages, trigger, messageId }) => ({
    body: { id, trigger, messageId, message: messages[messages.length - 1] }
  })
})
```
`trigger` is the SDK's own `'submit-message' | 'regenerate-message' | 'resume-stream'` — reuse verbatim, don't re-derive server-side.
`sendAutomaticallyWhen`/`onError` stay as-is.

**Server** (`server/api/chat.post.ts`): read `{ message, trigger, id: conversationId }` instead of `{ messages, id }`. After the existing conv/model/provider lookups, reconstruct `messages` from the DB (same select shape as `listConversationMessages`):
```ts
const dbRows = await db.select().from(messagesTable)
  .where(eq(messagesTable.conversationId, conv.id))
  .orderBy(asc(messagesTable.createdAt))
let messages: UIMessage[] = dbRows.map(r => ({ id: r.id, role: r.role as UIMessage['role'], parts: r.parts }))

if (trigger === 'submit-message' && message?.role === 'user') {
  const [inserted] = await db.insert(messagesTable)
    .values({ conversationId: conv.id, role: 'user', parts: message.parts })
    .returning({ id: messagesTable.id })
  messages.push({ ...message, id: inserted.id })
} else if (trigger === 'regenerate-message') {
  // drop the stale assistant answer being replaced — not history for this call
  if (messages.at(-1)?.role === 'assistant') messages = messages.slice(0, -1)
} else {
  // tool-approval resume (and resume-stream, safe no-op if `message` is undefined):
  // swap in the client's freshly-updated version of the in-flight assistant
  // message — DB still has the pre-approval parts.
  if (message && messages.length > 0) messages[messages.length - 1] = message
}
```
Every downstream use of `messages as UIMessage[]` (`resolveMessagesForModel`, `runLanggraphChat({ uiMessages })`, `convertToModelMessages(...)`, `toUIMessageStream({ originalMessages: messages, ... })`) switches to this reconstructed array — no call-site shape changes.

`isContinuation` detection (`toUIMessageStream`'s `state.message.id === lastMessage?.id`, `node_modules/ai/dist/index.js:7456`) still works unchanged: the approval-resume branch puts the client's message (same id as what's being resumed) at the end of the reconstructed array, same as today.

### Known accepted gap (pre-existing, unchanged by this phase)
The stale assistant row being regenerated away is never deleted from the DB — it's excluded from what's sent to the model this turn, but stays as an orphaned row. This is true of today's code too (nothing currently deletes it either); fixing it is a separate, explicit scope decision, not bundled into this payload-size fix.

### Files touched
- `app/composables/useConversationChat.ts` — `DefaultChatTransport` + `prepareSendMessagesRequest`.
- `server/api/chat.post.ts` — read `{ message, trigger, id }`, reconstruct `messages` from `messagesTable` keyed by `trigger`.

No schema/migration changes.

### Verification
- New turn: persists once (not duplicated), reply coherent.
- Regenerate: exactly one new assistant row, no duplicate user row, regenerated answer doesn't reference the discarded old answer.
- Tool approval: full approve flow still completes exactly as before (highest-regression-risk path — re-check against `chat.post.ts:57-64`'s documented behavior).
- Network tab: `/api/chat` request body is a single message, not the full array, even on a long conversation.
- `npx nuxi typecheck` clean.
