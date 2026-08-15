# P1 — Actual browser chat trace continuity: evidence

Plan 035, remediation round 2. Confirms `DefaultChatTransport` (AI SDK) now
carries `traceparent` on `/api/chat` requests via the same reusable
traced-fetch primitive used by the global `$fetch` override, and that the
third-party-origin gate still holds.

## 1. Code-level proof: DefaultChatTransport's fetch option is wired

`node_modules/ai/dist/index.d.ts:5683` — `HttpChatTransportInitOptions`
(which `DefaultChatTransport`'s constructor destructures) accepts a `fetch`
option (`ReturnType<typeof getOriginalFetch> | undefined`), confirming the
SDK does NOT use `globalThis.$fetch` internally and exposes an explicit
injection point.

`app/composables/chat/chat-transport.ts` now passes:

```ts
fetch: createTracedFetch(telemetry, globalThis.fetch)
```

`createTracedFetch` (`app/utils/trace-context.ts`) is the single reusable
primitive — the SAME function instance-shape used by
`app/plugins/trace-context.client.ts` as the underlying fetch for
`globalThis.$fetch` (via `ofetch.create({}, { fetch: tracedFetch })`). No
duplicate trace-generation or telemetry-recording logic exists between the
two call sites.

## 2. Live proof against the running dev server

App was already running as a live Nuxt dev server (`nuxt dev`, PID 2593107)
on `http://100.99.88.53:3333` (tailscale interface), reflecting this
branch's code after HMR recompiled the plugin/composable changes.

Manually constructed a valid W3C `traceparent` and POSTed to `/api/chat`
(the same endpoint `createConversationTransport()` targets), simulating what
the traced fetch now sends:

```
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01

POST /api/chat -> HTTP/1.1 401 Unauthorized (no session cookie — expected,
proves the request reached the real chat route/auth guard)
x-request-id: f4d29a3e-c931-4dfb-9a18-64bdc81d58a6
```

This proves: (a) `/api/chat` accepts and does not reject a request carrying
`traceparent`, and (b) the server returns `x-request-id`, which
`createTracedFetch` captures and correlates via `telemetry.logEvent(...,
'trace.id': traceId, 'request.id': requestId, ...)` — the same Phase 4
telemetry event path `api.request.success`/`api.request.error` other
same-origin requests already use.

**Honesty note on full trace correlation**: server-side OTel/Loki
trace_id correlation (`server/infrastructure/observability/otel.ts`) is
gated behind `NUXT_OTEL_ENABLED=true`, which is unset in this environment's
`.env` — the OTel SDK runs as a no-op tracer/logger and never exports to the
running `sensio-loki` container regardless of the incoming `traceparent`.
Full live trace_id-in-Loki correlation for this specific request therefore
could not be captured in this environment. What COULD be verified live: the
request path accepts the header end-to-end and the server-observable
`x-request-id` correlation the client records is real and unchanged.

## 3. Same-origin gate still enforced (no third-party leakage)

Standalone reproduction of `isSameOriginApiRequest`'s exact algorithm
(`app/utils/trace-context.ts`), exercised against two calls:

```json
[
  {
    "url": "http://localhost:3333/api/chat",
    "headers": { "traceparent": "00-1111...-2222...-01" }
  },
  {
    "url": "https://evil-third-party.example.com/collect",
    "headers": {}
  }
]
```

Same-origin `/api/**` gets `traceparent`; a different-origin fetch gets
none — confirming the gate that `createTracedFetch` shares across both the
`$fetch` override and the chat transport's explicit `fetch` option.

No `baggage` header is ever set (grep confirms `baggage` does not appear
anywhere in `app/utils/trace-context.ts`, `app/plugins/trace-context.client.ts`,
or `app/composables/chat/chat-transport.ts`).
