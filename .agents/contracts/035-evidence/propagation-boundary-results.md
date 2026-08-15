# Plan 035 Phase 11 — Case 6: Propagation boundary evidence

Method: source-level inspection of the actual OTel wiring (`otel-preload.mjs`), corroborated by
live Jaeger span attributes captured during this acceptance run. This is a fail-closed design, so
the correct behavior is a *global absence* of outbound `traceparent` injection, which cannot be
proven by a single positive HTTP capture alone — the code inspection is the primary evidence, and
the Jaeger captures for same-origin/first-party requests confirm span creation still happens.

## Same-origin / first-party inbound requests DO get instrumented spans

Every inbound Nuxt HTTP request is instrumented as an `IncomingMessage` server span by
`HttpInstrumentation` (registered unconditionally when `NUXT_OTEL_ENABLED=true`,
`otel-preload.mjs:97-116`), and every inbound Rust relay request creates a `relay.request` span
(`packages/rust-tools/infrastructure/src/transport.rs:673`). Confirmed live in this run: the
`relay.request` span for our `initialize` call carries `request_id=3910a88a-...` matching the
`x-request-id` header returned to the caller (see `rust-happy-initialize.md`).

## Outbound propagation is fail-closed for EVERYTHING, including the one first-party candidate

`otel-preload.mjs:69-101` (comment block, Plan 035 Phase 7) documents and implements this
precisely:

- `HttpInstrumentation`'s default behavior is to inject `traceparent`/`tracestate` into every
  outgoing `node:http`/`node:https` request via `propagation.inject()`.
- Because this OTel version has no per-instrumentation flag to disable injection while keeping
  span creation, the fix is one layer up: `tracerProvider.register({ propagator: noInjectPropagator })`
  installs a propagator whose `inject()` is an intentional no-op, while `extract()` (reading
  inbound `traceparent` off requests arriving at this server) still delegates to the real W3C
  implementation.
- Net effect, confirmed by reading the code: **no outbound HTTP call from the Nuxt server process
  ever gets a `traceparent`/`tracestate` header injected by default, for any destination** —
  model providers, remote MCP, OAuth, and even a hypothetical first-party Rust relay call.
- Per the Phase 0 contract (`§4`), there is currently **no Nuxt-server-initiated HTTP call to the
  Rust relay at all** — the relay is reached directly by the browser
  (`app/composables/useRelayAgent.ts`), not by the Nuxt server. So today's correct, honest state is
  "propagate to nothing," not an allowlist with one first-party entry — there is no real
  server-to-relay HTTP call site yet to allowlist. A future phase adding such a call must add a
  narrowly-scoped injection path for that one destination, not remove the no-op propagator.
- `W3CBaggagePropagator` is never registered — confirmed absent from the propagator wiring.

## Verification performed this run

1. Read `otel-preload.mjs` in full — confirmed the `noInjectPropagator` construction and the
   `tracerProvider.register({ propagator: noInjectPropagator })` call (no default
   `CompositePropagator` is ever installed).
2. Confirmed (Case 2/3 evidence) that a real outbound call to a third-party-shaped destination
   (`http://127.0.0.1:1`, treated as a user-configured provider `baseUrl`) produced a local
   CLIENT-side failure (SSRF-guard rejection) with no header-injection code path involved — the
   call never leaves the process, so this doesn't independently prove header omission, but is
   consistent with — and does not contradict — the code-level guarantee above.
3. Confirmed the Rust relay's own inbound span (`relay.request`) correlates via `request_id`
   attribute to the client-visible `x-request-id`, proving inbound span creation and correlation
   work correctly for the one real first-party endpoint that exists (browser → relay), even though
   that specific hop is not itself continued from a Nuxt server span today (documented as an
   honest limitation in the Phase 10 contract §"Operator lookup flow").

## Verdict

PASS by code inspection: the propagator-level no-op-inject design makes it structurally
impossible for any outbound call (first-party or third-party) to receive an injected
`traceparent`/`tracestate` today, which is the fail-closed default the plan requires. A live
packet-capture proving the *absence* of a header is lower-value than the propagator-level proof
above, since the header wiring is centralized in one place, not per-destination.
