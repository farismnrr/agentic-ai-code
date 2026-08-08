# 018 — LangGraph to AI SDK UI Message Stream Bridge

Bridging LangGraph's `createReactAgent` stream into the AI SDK's `createUIMessageStream({ execute: ({ writer }) => ... })` requires manually mapping `streamEvents` (with `version: 'v2'`) to writer calls and accumulating the result parts for persistence.

- `on_chat_model_stream`: map `chunk.content` to `writer.writeTextDelta` and append to a `currentText` string.
- `on_tool_start`: if `currentText` has accumulated, push a text part and clear it. Then map to `writer.writeCallTool` and push a `tool-invocation` part (with `state: 'call'`) to the parts array.
- `on_tool_end`: update the corresponding `tool-invocation` part's `state` to `'result'` and populate its `result` field. Then call `writer.writeToolResult`.
- At stream end, push any remaining `currentText`, call `writer.finish()`, and manually invoke the persistence callback (unlike `streamText` which wraps this in a `toUIMessageStream` `onEnd` handler).
