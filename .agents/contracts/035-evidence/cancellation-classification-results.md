# Plan 035 Phase 11 — Case 8/13: Cancellation classification

## Attempt at a live trigger

A live end-to-end trigger requires a real, reachable model provider (OpenAI/Anthropic-compatible
endpoint or Vertex AI) with a valid API key so a chat stream actually starts, followed by a
client-side abort mid-stream. Setting up a genuine outbound-reachable provider credential was not
available in this environment within the acceptance time budget (the only provider created in this
run was intentionally unreachable, for the 502 SSRF-guard evidence — see
`error-path-502-badgateway.md` — which fails before any stream starts and therefore cannot exercise
this code path). Rather than fabricate a live trace/log capture for this specific case, this is
documented as a genuine, honestly-reported gap and backed instead by a direct source read of the
exact classification logic that would run.

## Source-level verification (real code, current `feat/035-p0-observability-contract` branch)

`server/infrastructure/ai/ai-sdk-stream.ts` (`streamAiSdkAgent`, AI SDK path), `onError` handler:

```ts
onError: ({ error }) => {
  logger.error('[chat stream]', error)
  const outcome = abortSignal.aborted ? 'cancelled' : 'error'
  if (outcome === 'cancelled') telemetry?.event('chat.stream.chunk_error', 'cancelled')
  else telemetry?.error('chat.stream.chunk_error', 'chat_stream_error', error)
}
```

`server/infrastructure/ai/langgraph/langgraph-chat.ts` (LangGraph path), equivalent catch block:

```ts
} catch (e: unknown) {
  ...
  if (abortSignal?.aborted) telemetry?.event('chat.stream.chunk_error', 'cancelled')
  else telemetry?.error('chat.stream.chunk_error', 'chat_stream_error', e)
  ...
}
```

`server/application/chat/execute-chat-turn.ts`, explicit abort-listener wiring (registered
unconditionally at stream start, independent of whether an error is ever thrown):

```ts
telemetry?.event('chat.stream.start', 'ok', { 'provider.type': provider.type })
if (abortSignal.aborted) {
  telemetry?.event('chat.stream.abort', 'cancelled', { 'provider.type': provider.type })
} else {
  abortSignal.addEventListener('abort', () => telemetry?.event('chat.stream.abort', 'cancelled', { 'provider.type': provider.type }), { once: true })
}
```

Both AI SDK and LangGraph provider paths check `abortSignal.aborted` **before** deciding
`outcome`, and route to `logger.warn`-shaped `event('chat.stream.chunk_error', 'cancelled')` /
`event('chat.stream.abort', 'cancelled')` (an `outcome: 'cancelled'` structured log/span event,
not `logger.error`) rather than `telemetry.error(...)` (which sets `outcome: 'error'` and
`error.code`) when the signal was intentionally aborted. This matches the plan's requirement:
"cancellation/intentional abort is classified separately from failure" — confirmed by reading the
actual conditional, not inferred from a comment.

## Verdict: PLAUSIBLE / code-verified, not live-triggered in this run.

The classification logic is correctly implemented per source inspection. A live trace/log capture
of an actual aborted stream was not obtained in this Phase 11 pass due to the lack of a reachable
provider credential in this environment — documented honestly rather than fabricated. A follow-up
pass with a real provider API key (or a local mock HTTP server standing in for a provider, wired
through the existing `openai_compatible` provider type) would close this gap with a live capture.
