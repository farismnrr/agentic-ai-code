# 022 — OpenTelemetry: FE→BE logging + tracing to Jaeger/Loki

## Context

Debugging the terminal-tool incidents this session (stuck requests, empty responses, hallucinated actions) relied entirely on manually grepping `console.error` calls and querying the DB by hand — there's no structured, queryable record of what actually happened, and nothing from the frontend is captured at all. `.agents/plans/006-error-handling.md` explicitly deferred this ("cukup `console.error` server-side untuk sekarang").

The user wants a real fix: an industry-standard OpenTelemetry pipeline — traces (spans) and logs, both frontend and backend — landing in the observability stack already running on this machine:

- **Traces → `masih-awam-jaeger`** (`docker ps` confirms a Jaeger container already dedicated to this project, on its own `masihawam-net` docker network — OTLP ports 4317/4318 not published to the host).
- **Logs → `sensio-loki`** (the user explicitly chose to reuse the existing shared Loki from a different project — accepting that this app's logging becomes dependent on that project's infra staying up — on `sensio-network`, port 3100 not published to host either).

Confirmed with the user: since neither docker network is reachable from a bare `pnpm dev` host process, this app gets **containerized for local dev** (new `Dockerfile` + `docker-compose.yml`), joined to both `masihawam-net` and `sensio-network`. Nothing in this repo touches Docker or OpenTelemetry today (verified: no `Dockerfile`, no `@opentelemetry/*` deps, no otel config anywhere) — this is greenfield.

**Backward compatibility is non-negotiable**: today's `pnpm dev` (host process, no Docker) must keep working exactly as-is. OTel is gated behind `NUXT_OTEL_ENABLED` (default `false`) so the existing flow never attempts to reach an unreachable Jaeger/Loki.

## Decisions

- **Two independent export legs, not one collector.** Traces go straight to Jaeger via OTLP/gRPC (Jaeger's OTLP receiver is standard, no intermediary needed). Logs go straight to Loki's native HTTP push API (`/loki/api/v1/push`). Deliberately **not** routing through the existing `sensio-alloy` collector — that's another project's shared agent/config, and pointing a third app's telemetry through it without touching its config is out of scope and riskier than two direct, this-repo-owned integrations.
- **Real OTel SDK on both legs**, not just "OTel-shaped JSON": `@opentelemetry/sdk-trace-node` + `@opentelemetry/exporter-trace-otlp-grpc` for traces; `@opentelemetry/sdk-logs` + `@opentelemetry/api-logs` for logs, with a small **custom LogRecordExporter** that translates OTel `LogRecord`s into Loki's push format (`{streams: [{stream: {labels}, values: [[ns, line]]}]}`) — this is the one bit of glue code needed since Loki isn't a native OTLP-logs receiver, but everything upstream of it (the SDK, the record shape, severity levels, trace/span correlation) is unmodified OTel.
- **One initialization point, server-side**: a Nitro plugin (`server/plugins/otel.server.ts`, loaded once at boot before any request) sets up the `NodeTracerProvider` + `LoggerProvider`, registers `@opentelemetry/instrumentation-http` for automatic request/response spans on every `/api/*` call "for free," and exports `getTracer()`/`getLogger()` helpers from a new `server/utils/otel.ts` for manual instrumentation elsewhere. No-ops entirely (returns no-op tracer/logger, does not attempt any network connection) when `NUXT_OTEL_ENABLED` is false — this is the mechanism that keeps host-mode `pnpm dev` untouched.
- **Frontend reports through the backend, not directly to Jaeger/Loki.** A browser can't reach `masihawam-net`/`sensio-network` and shouldn't need CORS-exposed OTLP endpoints. A small client-side composable (`app/composables/useTelemetry.ts`) batches structured log events (including uncaught errors, captured via a new Nuxt plugin wiring `vueApp.config.errorHandler` + `window.onerror`/`onunhandledrejection`) and flushes them — periodically and on page unload via `navigator.sendBeacon` — to a new `POST /api/telemetry` endpoint. That endpoint feeds the exact same server-side OTel Logs pipeline, tagged with `service.name: 'ai-code-frontend'` (vs `ai-code-server` for backend-originated logs), so both ends land in Loki queryable the same way.
- **Docker networking**: new `docker-compose.yml` at repo root declares `masihawam-net` and `sensio-network` as `external: true` (they already exist, this repo doesn't own their lifecycle) and attaches the app service to both, alongside its normal Nuxt port. Postgres isn't part of either network (today's dev Postgres is reached via `localhost`) — the compose service gets `extra_hosts: ["host.docker.internal:host-gateway"]` (Docker's standard host-reachability mechanism) and a compose-scoped `NUXT_DATABASE_URL` override pointing at `host.docker.internal` instead of `localhost`, so the containerized app can still reach the same Postgres without touching the host dev flow's own `.env`.
- **Scope of instrumentation**: automatic HTTP server spans (via `instrumentation-http`) cover every request for free in phase one. Deep manual spans around specific operations (the LLM call in `chat.post.ts`, terminal command execution) and migrating the ~20 existing ad-hoc `console.error`/`console.warn` call sites onto the new logger are real, valuable follow-ups but are **out of scope for this plan** — get the pipeline working end-to-end first; instrumenting individual call sites is mechanical once `getLogger()`/`getTracer()` exist.

## Changes

### New dependencies (`package.json`)
`@opentelemetry/api`, `@opentelemetry/api-logs`, `@opentelemetry/sdk-node`, `@opentelemetry/sdk-trace-node`, `@opentelemetry/sdk-logs`, `@opentelemetry/exporter-trace-otlp-grpc`, `@opentelemetry/instrumentation-http`, `@opentelemetry/resources`, `@opentelemetry/semantic-conventions`. No extra package for the Loki leg — a plain `fetch` POST is enough for the custom exporter.

### `server/utils/otel.ts` (new)
- `getTracer(name)` / `getLogger(name)` — thin wrappers around the OTel API's global providers. Return real instances when `NUXT_OTEL_ENABLED` is true (set up by the plugin below), no-op instances otherwise — callers never branch on the flag themselves.
- `LokiLogExporter` (implements OTel's `LogRecordExporter` interface) — batches `LogRecord`s, POSTs them to `NUXT_OTEL_LOKI_PUSH_URL`, mapping OTel severity → a `level` label and including `trace_id`/`span_id` as structured metadata when present (this is what makes a log line click-through-able to its trace in Grafana).

### `server/plugins/otel.server.ts` (new)
Runs once at Nitro startup. If `NUXT_OTEL_ENABLED`: builds a `Resource` (`service.name: 'ai-code-server'`, version, environment), registers `NodeTracerProvider` with `OTLPTraceExporter` pointed at `NUXT_OTEL_JAEGER_ENDPOINT`, registers `LoggerProvider` with the `LokiLogExporter`, and registers `HttpInstrumentation` for automatic request spans. If disabled, does nothing (no provider registration at all — `getTracer`/`getLogger` fall back to OTel's own built-in no-ops).

### `server/api/telemetry.post.ts` (new)
Accepts a batch of frontend-originated log events (`{ level, message, attributes, timestamp }[]`), validates with `valibot` (matching this repo's existing validation convention, e.g. `server/api/conversations/index.post.ts`), and emits each through `getLogger('ai-code-frontend')`. No auth requirement beyond the existing session (reuses `requireUserSession`) so events are attributable to a user, matching how every other API route in this repo already gates on session.

### `app/composables/useTelemetry.ts` (new)
- `logEvent(message, attributes?)` / `logError(error, context?)` — pushes onto an in-memory batch.
- Flushes the batch to `POST /api/telemetry` on a short interval and via `navigator.sendBeacon` on `visibilitychange`/`beforeunload` (beacon is fire-and-forget and survives page unload, unlike a normal `fetch`).

### `app/plugins/telemetry.client.ts` (new)
Wires `nuxtApp.vueApp.config.errorHandler`, `window.onerror`, and `window.onunhandledrejection` to `useTelemetry().logError(...)` — this is what actually captures the FE crashes that today just vanish into the browser console.

### `Dockerfile` (new, repo root)
Multi-stage: `pnpm install` + `pnpm build` in a build stage, then a slim runtime stage running `node .output/server/index.mjs` — the standard Nitro-output deployment shape, nothing app-specific.

### `docker-compose.yml` (new, repo root)
One `app` service: builds from the new `Dockerfile`, publishes the app's usual port, attaches to `masihawam-net` and `sensio-network` (both `external: true`), sets `extra_hosts` for Postgres reachability, and sets `NUXT_OTEL_ENABLED=true`, `NUXT_OTEL_JAEGER_ENDPOINT=http://jaeger:4317`, `NUXT_OTEL_LOKI_PUSH_URL=http://loki:3100/loki/api/v1/push` (internal docker DNS names — `jaeger` and `loki` are the network aliases `docker inspect` already showed on those containers).

### `.env.example` additions
```
# --- Telemetry (plan 022) ---
NUXT_OTEL_ENABLED=false
NUXT_OTEL_SERVICE_NAME=ai-code
NUXT_OTEL_JAEGER_ENDPOINT=http://localhost:4317
NUXT_OTEL_LOKI_PUSH_URL=http://localhost:3100/loki/api/v1/push
```
(The `docker-compose.yml` overrides the last two to the docker-internal service names; the `.env.example` defaults describe what a host process would need if the ports were ever published locally.)

## Out of scope

- Touching `sensio-alloy`'s configuration, or routing this app's telemetry through it.
- Migrating the ~20 existing ad-hoc `console.error`/`console.warn` call sites to the new logger — real follow-up, not required for the pipeline to exist and work.
- Manual spans around specific business logic (LLM calls, terminal exec) beyond the automatic HTTP-level spans `instrumentation-http` provides for free.
- Any change to how the non-Docker `pnpm dev` flow runs day-to-day — it keeps working exactly as today, OTel simply never activates for it.
- Grafana dashboards/alerting on top of the new data — that's a Grafana-side follow-up once data is actually flowing.

## Verification

- `docker compose up --build` — app container starts, joins both networks (`docker network inspect masihawam-net` / `sensio-network` show the new container attached), and can still reach Postgres (check `db:migrate` or a login round-trip works against the containerized app).
- With `NUXT_OTEL_ENABLED=true`: hit any `/api/*` route, confirm a trace appears in Jaeger's UI (`http://localhost:16686`, already published) for `service.name: ai-code-server`.
- Trigger a frontend error deliberately (e.g. temporarily throw in a component) and confirm a `ai-code-frontend`-tagged log line lands in Loki (query via Grafana at `http://localhost:3004`, already published, against the `sensio-loki` datasource).
- Confirm a log line emitted during a traced request carries a `trace_id` that matches a real trace in Jaeger for that request (the FE→BE→Loki→Jaeger correlation is the actual point of doing this with real OTel instead of plain `console.log`).
- With `NUXT_OTEL_ENABLED=false` (or unset): confirm `pnpm dev` on the host starts and behaves exactly as before — no connection attempts, no errors, no behavior change. This is the regression check that matters most.
