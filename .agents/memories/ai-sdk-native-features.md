# The AI SDK already provides streaming simulation and tool approval — don't rebuild them

Two things in `ai@7` that are easy to miss and expensive to reimplement:

**`simulateReadableStream({ chunks, initialDelayInMs, chunkDelayInMs })`** is exported from `ai`. It is what `app/utils/mock-transport.ts` uses instead of a hand-written `setTimeout` loop. It is pull-based, so cancelling the reader — which is what `useChat()`'s `stop()` does — ends it naturally. Don't add `abortSignal` plumbing to "make stop work"; it already does.

**Tool approval is a first-class SDK flow.** `UIMessageChunk` includes `tool-approval-request` / `tool-approval-response`, tool parts carry `approval-requested` and `approval-responded` states, and `useChat()` returns `addToolApprovalResponse({ id, approved, reason })`. The approval modal must render *that* state and answer through *that* method.

**The trap:** building a parallel approval state machine in our own store. It looks reasonable, and it works until the SDK's own tool part stays stuck in `approval-requested` forever because nothing ever answered it. Our `Conversation.approvals` map is deliberately narrow — it only remembers "always allow" / "always deny" so the dialog can be skipped, and is never the source of truth for whether a specific call was approved.

Read the installed `.d.ts` before writing against this SDK. Both of these were found that way, and neither is obvious from the Nuxt UI chat docs.
