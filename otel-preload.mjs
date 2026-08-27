// Node --import preload (plan 022). HTTP auto-instrumentation must patch
// `node:http`/`node:https` before ANYTHING else imports them — Nitro's own
// entry (.output/server/index.mjs) does `import 'node:http'` at the very
// top, before any Nitro plugin (including server/plugins/otel.server.ts)
// ever runs, so registering HttpInstrumentation from inside the app itself
// is always too late. This file runs via `node --import ./otel-preload.mjs`
// (see Dockerfile CMD), executing before Nitro's module graph loads at all.
//
// Only the tracer + HTTP instrumentation live here, since those are the
// only pieces with this early-patch requirement. The logs pipeline
// (LoggerProvider + Loki export) has no such constraint and stays in
// server/plugins/otel.server.ts, which is the natural place for it.
if (process.env.NUXT_OTEL_ENABLED === 'true') {
  const { NodeTracerProvider, SimpleSpanProcessor } = await import('@opentelemetry/sdk-trace-node')
  const { OTLPTraceExporter } = await import('@opentelemetry/exporter-trace-otlp-grpc')
  const { resourceFromAttributes } = await import('@opentelemetry/resources')
  const { ATTR_SERVICE_NAME, ATTR_URL_FULL } = await import('@opentelemetry/semantic-conventions')
  const { HttpInstrumentation } = await import('@opentelemetry/instrumentation-http')
  const { registerInstrumentations } = await import('@opentelemetry/instrumentation')
  const { diag, DiagConsoleLogger, DiagLogLevel } = await import('@opentelemetry/api')
  const { W3CTraceContextPropagator } = await import('@opentelemetry/core')

  diag.setLogger(new DiagConsoleLogger(), DiagLogLevel.ERROR)

  const resource = resourceFromAttributes({
    [ATTR_SERVICE_NAME]: process.env.NUXT_OTEL_SERVICE_NAME || 'ai-code-server',
    'service.version': '1.0.0',
    'deployment.environment': process.env.NODE_ENV || 'development'
  })

  const configuredTraceEndpoint = process.env.NUXT_OTEL_JAEGER_ENDPOINT || 'http://localhost:4317'
  // Backward-compatible runtime migration: older deployed containers may
  // still carry the legacy Compose hostname `jaeger` in their immutable
  // environment. The shared observability stack now runs Tempo. Normalize
  // only that exact legacy hostname; arbitrary configured endpoints are
  // preserved unchanged.
  const traceEndpoint = configuredTraceEndpoint === 'http://jaeger:4317'
    ? 'http://shared-tempo:4317'
    : configuredTraceEndpoint
  const traceExporter = new OTLPTraceExporter({ url: traceEndpoint })

  // SimpleSpanProcessor, not Batch — see server/plugins/otel.server.ts's
  // comment on the same decision for the logs side; the same
  // Nitro-externalized BatchSpanProcessor never actually exports here
  // either, confirmed live, regardless of constructor argument shape.
  const tracerProvider = new NodeTracerProvider({
    resource,
    spanProcessors: [new SimpleSpanProcessor(traceExporter)]
  })

  // Plan 035 Phase 7 — fail-closed outbound trace propagation.
  //
  // `HttpInstrumentation` injects `traceparent`/`tracestate` into every
  // outgoing `node:http`/`node:https` request by calling
  // `propagation.inject(requestContext, optionsParsed.headers)` right before
  // the request is dispatched (see `_getPatchOutgoingRequestFunction` in
  // @opentelemetry/instrumentation-http@0.221.0's `http.js`) using whatever
  // propagator is registered globally via `@opentelemetry/api`. There is no
  // per-instrumentation config option in this version to disable injection
  // while keeping span creation (`disableOutgoingRequestInstrumentation`
  // and `ignoreOutgoingRequestHook` both suppress the span too, which the
  // plan requires we keep for latency/status visibility on ALL outbound
  // HTTP). So the fail-closed default is implemented one layer up, at the
  // propagator: instead of letting `tracerProvider.register()` install its
  // default `CompositePropagator([W3CTraceContextPropagator,
  // W3CBaggagePropagator])` (see `setupPropagator` in
  // @opentelemetry/sdk-trace-node's `NodeTracerProvider.js` — `register()`
  // always installs a propagator when `config.propagator` isn't supplied,
  // and a later, separate `propagation.setGlobalPropagator()` call would
  // silently no-op against the one already registered by `register()`, per
  // `registerGlobal`'s `allowOverride` guard in `@opentelemetry/api`), we
  // pass an explicit `propagator` into `register()` whose `inject()` is a
  // no-op while `extract()` (used to read incoming `traceparent` off
  // requests arriving at this server) still delegates to the real W3C
  // implementation. Net effect: NO outbound HTTP call from this Node
  // process ever gets a `traceparent`/`tracestate` header injected by
  // default, for any destination — model providers, remote MCP, OAuth,
  // everything — while inbound trace linkage and span creation for
  // outbound calls are both unaffected.
  //
  // Per the Phase 0 audit (see
  // .agents/contracts/035-observability-telemetry-contract.md §4), there is
  // currently NO Nuxt-server-initiated HTTP call to a first-party Rust relay
  // endpoint — the relay is reached from the user's own local machine, not
  // from this server process. So there is no real destination today that
  // should receive propagated trace headers, and this intentionally
  // implements "propagate to nothing" rather than inventing an allowlist
  // entry for a call site that doesn't exist. When a future phase adds a
  // real server-to-relay HTTP call, that is the place to add an explicit,
  // narrowly-scoped injection path (e.g. a dedicated propagator/hook that
  // only injects for that one first-party destination) — do not simply
  // remove this no-op propagator, or every third-party destination
  // regains automatic trace-header propagation.
  //
  // W3C `baggage` propagation is deliberately never registered here.
  const w3cTraceContext = new W3CTraceContextPropagator()
  const noInjectPropagator = {
    inject() {
      // No-op by design — see comment above. Nothing is written to the
      // outgoing carrier, so no `traceparent`/`tracestate` header is ever
      // attached to an outbound request via this mechanism.
    },
    extract(context, carrier, getter) {
      return w3cTraceContext.extract(context, carrier, getter)
    },
    fields() {
      return w3cTraceContext.fields()
    }
  }

  tracerProvider.register({ propagator: noInjectPropagator })

  registerInstrumentations({
    instrumentations: [
      new HttpInstrumentation({
        // Outgoing span attributes include `url.full` with only a fixed set
        // of known-sensitive query params redacted by default (sig,
        // Signature, AWSAccessKeyId, X-Goog-Signature — see
        // DEFAULT_QUERY_STRINGS_TO_REDACT in the installed instrumentation's
        // utils.js). Provider/MCP/OAuth URLs can carry other sensitive
        // values in their query string (e.g. API keys), so for outgoing
        // (ClientRequest) spans only, overwrite `url.full` with a
        // query-string-free `host + pathname` value. Incoming server spans
        // (IncomingMessage) are left untouched.
        requestHook: (span, request) => {
          if (typeof request.path !== 'string') return // IncomingMessage (server span) — not our concern here
          const pathname = request.path.split('?')[0]
          const host = typeof request.getHeader === 'function' ? request.getHeader('host') : undefined
          span.setAttribute(ATTR_URL_FULL, host ? `${host}${pathname}` : pathname)
        }
      })
    ]
  })
}
