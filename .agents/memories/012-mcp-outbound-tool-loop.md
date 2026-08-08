---
name: 012-mcp-outbound-tool-loop
description: chat.post.ts's outbound MCP tool-calling uses streamText + @ai-sdk/openai-compatible + toolApproval instead of hand-rolled SSE parsing — and denying a tool doesn't stop a model from retrying it.
metadata:
  type: project
---

Plan [[012-mcp-api-key]] Phase 2 wires stored `mcp_servers` rows into chat. The implementation goes through `ai@7`'s own tool-calling machinery end to end, not a hand-rolled OpenAI SSE parser:

- `server/utils/router-model.ts` addresses 9Router via `@ai-sdk/openai-compatible`'s `createOpenAICompatible(...).chatModel(id)`, giving `streamText` a real `LanguageModel`.
- `server/utils/mcp-tools.ts` turns a conversation's enabled MCP tools into an `ai@7` `ToolSet` (`tool({ inputSchema: jsonSchema(...), execute })`) plus a `toolApproval` map built from `conversations.approvals` (`'always'` → `'approved'`, `'never'` → `'denied'`, unset → `'user-approval'`).
- `chat.post.ts` calls `streamText({ model, tools, toolApproval, stopWhen: stepCountIs(5), ... })`, then the standalone `toUIMessageStream()` + `createUIMessageStreamResponse()` (not the deprecated `result.toUIMessageStreamResponse()` instance method — `ai@7.0.55` flags every instance-method stream helper as deprecated in favor of the standalone functions taking `result.stream`).

**Why:** this is what makes multi-tool-call handling, the pause-for-approval / resume-on-`addToolApprovalResponse` flow, and re-calling the model with tool results all come from the SDK for free. `app/components/ChatToolApproval.vue` was already built against the SDK's real `approval-requested` state before this Phase 2 work started — a hand-rolled `'tool-call'`/`'tool-result'` chunk protocol (an earlier, discarded draft of this work took that approach) would never have driven that component correctly. See [[ai-sdk-native-features]] for the standing warning against parallel state machines here.

**Live-verified finding — denial doesn't stop retries:** when a tool's `toolApproval` resolves to `'denied'`, the underlying MCP tool is genuinely never invoked (confirmed at the wire level: 0 calls reached the real test server). But the model (tested against 9Router's `vx/gemini-3-flash-preview`) doesn't reliably take the hint — it retried the identical denied call 5 times before `stopWhen: stepCountIs(5)` cut the turn off, leaving the user with no text reply at all for that turn. This is a model/prompting characteristic, not a bug in the wiring: the security property (denied tools cannot execute) holds regardless. `stepCountIs(5)` is what bounds the damage to a fixed number of wasted model calls instead of an unbounded loop.

**How to apply:** if this UX gap (silent empty turn after a denial) becomes a real complaint, the fix belongs in prompting/system-instructions (tell the model explicitly that a denial is final) or in a step-level check that short-circuits on a repeated identical denied call — not in loosening `toolApproval` or lowering the step cap further, which would trade a real security property for a UX patch.
