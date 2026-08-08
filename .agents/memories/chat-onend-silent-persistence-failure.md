---
name: chat-onend-silent-persistence-failure
description: server/api/chat.post.ts's onEnd callback can fail to persist the assistant message with zero error output anywhere, intermittently — found live-testing plan 014's reasoning effort feature
metadata:
  type: feedback
---

`toUIMessageStream`'s `onEnd` option (in `server/api/chat.post.ts`) is invoked from the underlying `TransformStream`'s `flush()`/`cancel()` hooks inside `ai@7`'s `handleUIMessageStreamFinish` (`node_modules/ai/dist/index.js`). Unlike the per-step `onStepFinish` callback, `ai` does **not** wrap this call in a try/catch. Before this was noticed, an exception thrown inside `onEnd` (from `close()`, or the `db.insert`/`db.update` calls) would error the response `ReadableStream` **after** every visible text/reasoning chunk had already been flushed to the browser — so the client renders a complete, correct-looking answer while the database write silently never happens, and nothing anywhere logs it (`streamText`'s own `onError` only covers generation errors, not `onEnd`).

**Why this matters:** confirmed live while testing plan [[014-reasoning-effort-and-model-cleanup]] — sending a message through "High Thinking" at `reasoningEffort: 'max'` sometimes fully renders in the browser but the `messages` table only ever gets the user row, never the assistant one, with **zero server-side error output**. Refreshing the page then loses the "sent" answer entirely, with no indication anything went wrong.

**Fix shipped:** `onEnd`'s body is now wrapped in try/catch with `console.error`, so a persistence failure is at least visible in server logs instead of invisible. This does not fix the underlying cause — it makes it debuggable.

**Root cause not fully pinned down — reproduction was intermittent, not deterministic**, with identical steps sometimes succeeding and sometimes failing. Leading theory, not yet confirmed: `event.node.req.on('close', () => abortController.abort())` (`chat.post.ts`) — `req`'s `'close'` event on Node's `IncomingMessage` is known to sometimes fire on normal, successful completion of a streamed response (not just client disconnects), and if it fires while the SSE body is still being flushed to the socket (plausible for slower, higher-token reasoning-effort responses), it can abort a request that both the model and the client already consider "done." Longer-running reasoning-effort responses (`'high'`/`'max'`) are more exposed to this timing window than short ones, which fits the observed pattern of it surfacing during plan 014 testing rather than plan 013's shorter responses.

**How to apply:** if this resurfaces, check for the new `console.error('[chat onEnd] failed to persist assistant message', err)` log line first — it will now show the real error instead of nothing. If the error trail points at the abort controller / a `AbortError`, investigate whether `req.on('close', ...)` needs to check `event.node.res.writableEnded` (or similar) before calling `abortController.abort()`, rather than aborting unconditionally on every `'close'` event.
