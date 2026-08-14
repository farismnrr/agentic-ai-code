# Plan 035: End-to-End Observability and Secure Telemetry

**Status: OPEN / REMEDIATION ROUND 3 IN PROGRESS (Phase 0 only).** Implementation branch `feat/035-p0-observability-contract`. Round 1 and round 2 histories remain preserved below; no later phase is complete for round 3.

**Remediation round 3 reason:** the user's confirmed review blockers are: **P1-A** raw exceptions leak secrets/internal detail into Jaeger; **P1-B** LangGraph/tool failure paths stream raw internal errors; **P1-C** Rust telemetry exports dependency noise/filesystem paths; **P1-D** the actual Nuxt → Rust `ai-tools` subprocess lacks the same distributed trace. The evidence gaps are: **E1** the browser happy path is not real; **E2** the Rust internal proof is a 400, not a genuine 5xx; **E3** requestId → Loki → trace is not guaranteed; **E4** closure docs overstate claims. This is a documentation/contract reset only; no later phase is complete until these blockers and evidence gaps are independently re-proven.

**Round 3 Phase 0 inventory (must remain visible until each later phase is independently re-proven):**
- **P1-A — raw exceptions leak secrets/internal detail into Jaeger.**
- **P1-B — LangGraph/tool failure paths stream raw internal errors.**
- **P1-C — Rust telemetry exports dependency noise/filesystem paths.**
- **P1-D — the actual Nuxt → Rust `ai-tools` subprocess lacks the same distributed trace.**
- **E1 — browser happy path is not real.**
- **E2 — Rust internal proof is 400, not genuine 5xx.**
- **E3 — requestId → Loki → trace is not guaranteed.**
- **E4 — closure docs overstate claims.**

**Remediation round 3 checklist (Phase 0–12):**
- [x] Phase 0 — reopen this plan without deleting round histories; freeze P1-A–D and E1–E4, security rules, propagation rules, and trace/boundary distinctions.
- [ ] Phase 1 — centralized safe exception representation and real Jaeger redaction proof.
- [ ] Phase 2 — all LangGraph/tool/client error confidentiality.
- [ ] Phase 3 — Rust trace filtering/data hygiene.
- [ ] Phase 4 — explicit Node→`ai-tools` subprocess trace propagation and third-party fail-closed proof.
- [ ] Phase 5 — guaranteed requestId→Loki→trace lifecycle lookup.
- [ ] Phase 6 — real browser chat happy-path evidence.
- [ ] Phase 7 — genuine Rust internal/OIDC/JWKS 5xx evidence.
- [ ] Phase 8 — comprehensive runtime canary leakage falsification.
- [ ] Phase 9 — fresh source-level security/architecture falsification.
- [ ] Phase 10 — full release verification including build/audits/scripts/LSP.
- [ ] Phase 11 — fresh independent worker closure review.
- [ ] Phase 12 — truthful final documentation and closure.

**Remediation round 2 summary:** Reopened at commit `2f67f4dd85a8c5e9130b81a9157d49a93a1b8909` (2026-08-14) after the user's own fresh review found three confirmed P1 gaps missed by round 1's independent review: (1) `server/core/errors/http.ts` imported `server/infrastructure/observability/logger` directly, violating the Plan 031-034 dependency direction; (2) `server/api/mcp/index.ts:97` returned raw caught-error text inside a 200 JSON-RPC MCP tool-result, bypassing the 5xx sanitizer entirely; (3) chat traffic used AI SDK's `DefaultChatTransport`, which never went through the `globalThis.$fetch` override Phase 4/7 patched — the single most user-visible request path got no trace correlation at all. All three fixed and re-proven live. Full remediation lane list:

- **Lane 1** (`cec664d`) — `server/core/errors/**` no longer imports infrastructure; `problem()` attaches a private `logPayload` on the thrown error's `data`, read back and logged by the Nitro global error handler via the already-injected `event.context.application.observability.logger`. `scripts/check-architecture.sh` extended to reject `server/core -> server/infrastructure` (value/type-only/facade/deep-relative forms) with negative/positive fixtures, both confirmed to fire correctly.
- **Lane 2** (`ded4c95`) — unified `createTracedFetch()` primitive (`app/utils/trace-context.ts`) now backs both `globalThis.$fetch` (via `ofetch.create({}, { fetch })`) and `DefaultChatTransport`'s explicit `fetch` option (`app/composables/chat/chat-transport.ts`) — no duplicated trace logic, same same-origin/no-baggage/no-third-party-leak guarantees for chat as every other request.
- **Lane 3** (`57df02b`, cleaned up at `dea57f5`) — deterministic value-level secret redaction (`redactSecrets()` in `server/infrastructure/observability/sanitize.ts`, `redact_secrets()` in Rust `observability.rs`) masks Bearer/Basic tokens, API keys, cookie/session/password/token/secret assignments, DB-URL userinfo, and JWT-like values inside otherwise-allowed attribute values (`error.message`, `stack`, free-form Rust diagnostics) — not just key allowlisting. A `#[cfg(test)]` module the implementing worker added (violating this repo's no-unit-test-suite policy) was removed and replaced with a black-box `examples/redact_check.rs` binary driven by `scripts/verify-value-level-secret-redaction.mjs`.
- **Lane 4** (`8418fe9`) — `server/api/mcp/index.ts` MCP tool-result failures now return a stable generic `"Tool execution failed"` message (still HTTP 200/`isError: true`, correct MCP semantics preserved); raw diagnostic goes only to `RequestTelemetryContext.error(...)`.
- **Lane 5** (`ee6ecae`) — fresh regression audit of lanes 1-4 found no new issues in the files they touched, but surfaced one more same-class pre-existing leak: `server/infrastructure/ai/langgraph/langgraph-chat.ts` wrote raw SearXNG tool-error text directly into user-visible chat stream output — fixed with the same generic-message/private-diagnostic split. Also fixed a broken relative import (doubled `infrastructure` path segment) found via LSP in the same file.
- **Lane 6** (`09003c4`, `5492f85`) — real, live E2E re-proof against a freshly rebuilt Docker image + standalone Jaeger + `sensio-loki`, this time with `NUXT_OTEL_ENABLED=true` (unlike round 1's evidence, which was code-only for the chat path due to OTel being disabled in that run). Genuine client-generated `trace_id`/`span_id` verified picked up server-side (`.agents/contracts/035-evidence/remediation-happy-path-chat.md` — the client's `traceparent` span ID appears as the exact parent reference of the server's `POST`->`chat.execute` span tree in a real Jaeger trace). All 6 required proofs (happy path A browser chat, happy path B Rust relay, error paths A/B/C/D) passed with live Loki+Jaeger evidence, not simulated. One new bug found honestly (not fabricated) and fixed: `server/infrastructure/ai/langgraph-stream.ts` called `runLanggraphChat()` without importing it (`ReferenceError` in production) — fixed at `5492f85`.
- **Lane 7** (`3a57581`) — second independent falsification review (deliberately more skeptical than round 1's, since round 1 missed real P1s) found one genuine new P1: `server/infrastructure/observability/logger.ts` printed raw, unredacted `err` objects to stdout/`docker compose logs` via `consola.error/warn(message, err)` — the `redactSecrets`/`sanitizeAttributes` pipeline only guarded the Loki `emit()` path, not the parallel console-output path. Fixed via a `consolaSafe()` helper that redacts `message`/`err.message`/`err.stack` before they ever reach `consola`. All other falsification categories (core/application->infrastructure leaks, OTel SDK leaking into core/application, MCP 200-body leakage, Rust stdout/tracing sites, Loki label cardinality, third-party propagation, request-ID trust, Rust admission ordering) checked clean.
- One deterministic-script false positive found and fixed (`6b820e9`): `scripts/verify-no-secret-leakage.sh`'s leak sweep flagged `value-level-secret-redaction-results.md` for containing the canary marker — that file's own before/after documentation table legitimately shows the raw canary next to its `[REDACTED]` result; added to the same intentional-documentation exclusion list as the other canary evidence files.
- Final verification, all passing on commit `6b820e9`: `pnpm verify:commit`, `pnpm build`, `pnpm audit` (no known vulnerabilities), `cargo audit` (no vulnerabilities, 297 crates), and all four deterministic acceptance scripts (`verify-value-level-secret-redaction.mjs`, `verify-mcp-tool-result-error-confidentiality.mjs`, `verify-no-secret-leakage.sh`, `verify-telemetry-endpoint-security.sh`) run live against the rebuilt stack with a real authenticated session.

**Baseline:** `dev` at `0134918100ddd2408c625ef6f96453edc11bd579`. Implementation baseline (post-sync): `origin/dev` at `0d3b1cc701e71c61775376c4dcdb8cd74619ab73`. Implementation branch: `feat/035-p0-observability-contract`.

**Execution notes:**
- Phase 0 (`.agents/contracts/035-observability-telemetry-contract.md`) complete at commit `e1479a3`. A pre-existing, unrelated lint failure on baseline `dev` (23 `@stylistic/quotes` errors) was fixed at commit `1b5875a` to unblock `pnpm verify:commit`.
- Phases 1-9 complete on `feat/035-p0-observability-contract` through commit `6722426`: application observability contract (Phase 1), Nuxt 5xx sanitization (Phase 2, P0), structured logging hardening (Phase 3), frontend telemetry envelope + trace continuity (Phase 4), `/api/telemetry` hardening (Phase 5), server journey spans (Phase 6), fail-closed outbound propagation (Phase 7), Rust OTel foundation (Phase 8), Rust request/auth/tool spans + Rust 5xx sanitization (Phase 9, P0).
- Recurring pattern found and fixed across nearly every worker phase: duplicate imports/identifiers and OTel-API type mismatches that pass `pnpm verify:commit`'s `vue-tsc -p .nuxt/tsconfig.json` check but are caught by the IDE/LSP, because that tsconfig's `include` list does not cover `server/**` (confirmed by running `vue-tsc -p .nuxt/tsconfig.server.json` directly, which is itself too noisy to use as a substitute — it lacks Nitro's auto-import resolution and reports many false positives like `Cannot find name 'useDb'`). This is a genuine gap in the repository's own mandated type-check gate for server-side code; every phase in this run was manually cross-checked against LSP diagnostics in addition to `pnpm verify:commit`, and real bugs found this way were fixed. Flagging for a future plan/phase — not fixed here as it's outside Plan 035's scope.
- Infra note: `docker ps` on this host shows `sensio-loki` (Loki) and `sensio-tempo` (Tempo) running, but no container named `jaeger`/`loki` matching `docker-compose.yml`'s expected service DNS names. This may block the mandatory real Jaeger/Loki evidence capture required to close Plan 035 (Phase 11/12) — to be re-verified when that phase is reached.
- Resolved: a standalone `jaegertracing/all-in-one` container (`masih-awam-jaeger`) was started on `masihawam-net`/`sensio-network` for evidence capture; `sensio-loki` (already running, aliased `loki` on `sensio-network`) was used as-is. Both are real, running backends, not simulated.
- Phase 11 (`.agents/contracts/035-evidence/`) complete at commit `8edc755`, with two follow-up fixes at `2aabaad`: real happy-path (API-key auth -> `x-request-id` -> Loki `request.id` -> `trace_id` -> Jaeger trace), controlled 500/502 error paths, Rust relay MCP happy/error paths, a canary-secret negative test (no leakage found), `/api/telemetry` abuse tests, propagation-boundary and backend-failure-resilience checks, and cancellation-classification notes (code-verified; not live-triggered — no reachable provider credential in this environment, disclosed honestly rather than fabricated). Two deterministic scripts added: `scripts/verify-telemetry-endpoint-security.sh`, `scripts/verify-no-secret-leakage.sh`.
- Phase 12 fresh independent security review (a separate worker, not reusing phase summaries) found **no P0/P1 issues**. Verdict: CLOSE. Informational-only follow-ups: `server/api/mcp/index.ts:96-97` returns raw upstream error text inside a 200 JSON-RPC tool-result body (outside the letter of the 5xx contract since it's a 200 response, but same failure class); logged error messages aren't scrubbed for embedded secret-shaped substrings (e.g. a DB URL with credentials) — private-log-only exposure, low risk, deferred to a future plan rather than blocking this one.
- Final verification, all passing on commit `2aabaad`: `pnpm verify:commit`, `pnpm build`, `pnpm audit` (no known vulnerabilities), `cargo audit` (no vulnerabilities found, 297 crates scanned).
- Non-blocking pre-existing bugs found and fixed while exercising real acceptance paths (predate Plan 035, unrelated to its scope, but blocked verification/evidence capture): see the canonical memory entry for the full list (broken relative imports, missing `useDb` imports breaking API-key auth, a missing `Bearer` prefix strip, telemetry status-code misclassification, and a `.dockerignore` gap).

## Objective

Implement one coherent observability contract from the browser/frontend through Nuxt/Nitro application and infrastructure boundaries into the Rust `ai-tools` / relay runtime.

The implementation must make successful and failed request journeys reconstructable without weakening any security boundary or leaking sensitive data.

Primary signals for this plan are **distributed traces + structured logs**. Metrics are not a closure requirement unless implementation proves they are necessary; do not grow a third telemetry pipeline merely for completeness.

## Non-negotiable requirements

1. A user-initiated request/action must be trace-correlatable from frontend -> Nitro -> application use case -> infrastructure/external boundary -> first-party Rust relay/tool execution where that path exists.
2. Happy paths and error paths must both leave sufficient structured telemetry to reconstruct the journey.
3. **No HTTP/JSON-RPC 5xx response may expose internal error details, stack traces, provider errors, database errors, filesystem errors, auth/JWKS details, or implementation messages to the client.**
4. Operators must be able to map a client-visible `requestId` to private logs/traces and see where the request went and where it failed.
5. Logging/telemetry must remain security-strict: no secret leakage, no arbitrary high-cardinality labels, no client-controlled opaque telemetry ingestion, no weakening of auth/admission/sandbox/SSRF boundaries, and no tracing headers sent to arbitrary third-party destinations.
6. Preserve repository policy: no CI, no unit-test suite, no git-hook bypass. Deterministic black-box/security acceptance scripts are allowed.
7. Preserve Layered Architecture from Plans 031-034. Application code consumes an application-owned observability contract; OpenTelemetry/Loki/Jaeger/Rust exporter details stay in infrastructure/composition.

---

## Current-state audit

### Frontend

Current `app/composables/useTelemetry.ts` batches free-form `{ level, message, attributes, timestamp }` records and sends them to `/api/telemetry` using `sendBeacon`/`fetch`.

Gaps:

- no distributed trace context is attached to frontend events;
- arbitrary `attributes` are accepted;
- `logError()` forwards raw error messages and browser stack traces;
- there is no stable event vocabulary or cardinality policy;
- telemetry transport failure can currently fall back to `console.error`, risking recursive/noisy observability behavior;
- no explicit per-batch/per-record size budget exists.

### Telemetry ingestion

Current `server/api/telemetry.post.ts` authenticates the caller but accepts free-form attributes and writes raw `session.user.id` into telemetry.

Gaps:

- client controls attribute keys and values;
- no strict allowlist/redaction boundary;
- no dedicated telemetry abuse/rate/size policy;
- raw user IDs become observability data;
- client data can create uncontrolled cardinality or inject secrets into Loki.

### Nuxt/Nitro server

Current tracing starts in `otel-preload.mjs` using Node HTTP instrumentation and OTLP trace export. Logs use `server/infrastructure/observability/logger.ts` -> OTel logs -> `LokiLogExporter`.

Gaps:

- no single request context exposing request ID / trace context to application code;
- log attributes are largely caller-defined;
- no central redaction/sanitization policy;
- route/use-case/infrastructure spans are incomplete;
- outbound auto-instrumentation/trace-header propagation is not yet governed by an explicit first-party trust policy;
- Loki JSON includes trace/span IDs when available, but code does not guarantee active-span context for all important log paths.

### 5xx handling

Current `server/core/errors/http.ts` exposes `detail` and `extra` in the RFC problem body for all statuses. `internal(cause)` derives public detail from `cause.message` and may include stack data. `badGateway(detail)` can expose upstream/provider failure text.

This violates Plan 035 requirement #3 and is a P0 closure blocker.

### Rust

Current Rust relay observability is `packages/rust-tools/infrastructure/src/observability.rs`:

- `x-correlation-id` is accepted from the request or generated locally;
- audit events are JSON written to stderr;
- some fields are privacy-reduced (`present`/`absent`);
- there is no OpenTelemetry trace/log exporter or W3C trace-context extraction;
- client-controlled correlation IDs remain part of operator telemetry;
- request path is not connected to the Nuxt trace.

Current relay transport also constructs some HTTP 500 / MCP internal responses containing detailed OIDC/JWKS/internal messages. Those must become private telemetry only.

---

## Target observability model

### Identity model

Use these concepts consistently:

- **trace ID** — W3C distributed trace identity. Generated/continued through standards-compliant `traceparent`.
- **span ID** — operation identity inside the trace.
- **request ID** — server-generated identifier for one inbound HTTP/MCP request. Safe to return to the client as `x-request-id` and in generic 5xx bodies.
- **operation/event name** — low-cardinality stable code such as `chat.execute`, `provider.discover`, `mcp.tools_call`, `relay.auth.validate`, not a raw URL or error message.

Do not overload these concepts. Do not use user IDs, conversation IDs, provider IDs, workspace IDs, paths, URLs, or tool arguments as span names or Loki labels.

### Frontend trace continuity

Frontend user actions and API requests must carry W3C trace context to **same-origin Nuxt requests only**.

Security/KISS decision:

- do **not** expose Jaeger/OTLP collectors directly to the browser;
- do **not** accept or proxy opaque browser OTLP payloads;
- frontend emits a strict, sanitized telemetry envelope to `/api/telemetry` and includes trace correlation fields;
- frontend request spans/events are represented as trace-correlated client telemetry, while server/Rust spans remain authoritative exported distributed spans;
- no W3C `baggage` propagation in this plan;
- never propagate tracing headers from the browser to arbitrary third-party origins.

### Server request context

Every inbound Nuxt API request must receive a fresh server-generated `requestId` and expose an application-safe request observability context through Nitro request context.

The context must provide only application-facing capabilities/types, for example:

```text
RequestTelemetryContext
  requestId
  traceId?            // read-only correlation value
  spanId?             // read-only current span value
  withSpan(operation, safeAttributes, fn)
  event(name, outcome, safeAttributes)
  error(name, errorCode, cause, safeAttributes)
```

Application modules must not import OpenTelemetry SDK types.

### First-party vs third-party propagation

Trace propagation is a trust-boundary decision.

Allowed by default:

- browser -> same-origin Nuxt;
- Nuxt -> explicitly identified first-party/local Rust relay endpoints;
- first-party internal HTTP hops reviewed by this plan.

Not allowed by default:

- model-provider endpoints;
- user-configured arbitrary provider base URLs;
- arbitrary remote MCP endpoints;
- email/OAuth/other third-party integrations.

Third-party calls still get **local CLIENT spans** with safe metadata and status/latency, but no internal `traceparent`, `tracestate`, `baggage`, auth metadata, or observability headers are injected unless a future reviewed policy explicitly opts a trusted destination in.

Audit `HttpInstrumentation` configuration so automatic outgoing propagation cannot silently bypass this policy. Prefer disabling generic outgoing auto-propagation and manually instrumenting reviewed outbound integration boundaries if that is the cleanest fail-closed design.

---

## Security logging contract

### Allowed stable attributes

Prefer a small controlled vocabulary, including only fields such as:

- `service.name`
- `deployment.environment`
- `component`
- `layer`
- `event.name`
- `operation`
- `outcome`
- `request.id`
- `http.request.method`
- low-cardinality route template / route operation
- `http.response.status_code`
- `duration_ms`
- `error.type`
- `error.code`
- low-cardinality `provider.type`
- low-cardinality `tool.name`
- low-cardinality `mcp.method`
- retry/attempt counts
- boolean/presence classifications such as `auth.present`

Trace/span IDs belong in structured log bodies/correlation fields, **not Loki labels**.

### Forbidden telemetry data

Never emit these into frontend telemetry, server logs, traces, Rust logs, Loki labels, or span attributes:

- `Authorization`, `Cookie`, `Set-Cookie`, `x-api-key`, provider custom-header values;
- session IDs, bearer/access/refresh tokens, OAuth codes, PKCE verifier/state values;
- provider/API keys, passwords, encryption keys, DB URLs/connection strings;
- raw request/response bodies;
- prompts, chat messages, reasoning text, model output;
- MCP/tool arguments or outputs;
- terminal commands, shell input, file contents;
- raw workspace/cwd/filesystem paths unless a future reviewed debugging mode explicitly redacts them;
- arbitrary full URLs or query strings;
- email/name or other direct PII;
- raw user/tenant identifiers by default;
- JWKS documents or token claims.

If stable user correlation is operationally required, introduce a dedicated server-only HMAC pseudonymization key and emit only a pseudonymous `user_ref`. Do not reuse session/encryption keys and do not emit raw user IDs.

### Sanitizer

Create one authoritative sanitization/redaction utility per runtime family (TypeScript and Rust) with the same policy vocabulary.

Requirements:

- allowlist beats denylist for structured attributes;
- cap string lengths and collection sizes;
- strip control characters for console/stderr safety;
- reject unknown frontend telemetry attributes rather than forwarding them;
- sanitize error class/code separately from raw messages;
- telemetry failures must never expose secrets while reporting telemetry failures.

### Cardinality

Loki labels stay static/low-cardinality: service/job, environment, level, component at most. Never label by trace ID, request ID, user ID, route parameter, URL, provider ID, conversation ID, workspace ID, error message, or tool input.

---

## 5xx public/private error split

### Nuxt

Refactor the error boundary so status `>= 500` always produces a generic public representation.

Expected public shape:

```json
{
  "problem": true,
  "type": "about:blank",
  "title": "Internal Server Error",
  "status": 500,
  "requestId": "..."
}
```

For 502/503/etc., the standard title/status may remain accurate, but internal/provider/database/network detail must not be included.

Private telemetry records:

- request ID;
- active trace/span ID;
- stable operation;
- error type/code;
- sanitized internal cause classification;
- stack trace only according to reviewed environment policy, never in public output.

Required API redesign:

- distinguish `publicDetail` from `cause` / operator diagnostic context;
- `internal(cause)` must never derive a client-visible detail from `cause.message`;
- `badGateway(...)` and all other 5xx helpers must use static public detail;
- add a global Nitro/unhandled error boundary so raw unexpected exceptions cannot bypass the rule.

4xx responses may preserve explicitly reviewed user-actionable detail, but must not accidentally include secrets.

### Rust

Apply the same rule to relay HTTP/JSON-RPC responses.

- any 5xx/internal MCP error body must be generic;
- OIDC discovery/JWKS/network/internal details remain private log/span events;
- every response receives the relay-generated request ID header;
- never use raw internal error text as JSON-RPC error `message`/`data` for 5xx.

---

## Execution phases

### Phase 0 — freeze contract and threat model

1. Record baseline files/behavior for frontend telemetry, server OTel/logging, error helpers, outbound integrations, Rust observability/transport, and environment config.
2. Define the event/attribute vocabulary and forbidden-data matrix in a tracked contract document under `.agents/contracts/035-*`.
3. Enumerate all first-party trace-propagation destinations and all third-party/no-propagation destinations.
4. Enumerate every 5xx-producing helper/path in Nuxt and Rust.
5. Define maximum telemetry sizes/rates before implementation.
6. Do not change runtime behavior until the contract is reviewed against current source.

**Exit criteria:** a deterministic written matrix identifies what is traced, what is logged, what is never logged, and where trace propagation is allowed.

### Phase 1 — application-owned observability contracts

1. Add application/core-safe observability interfaces/types with no OpenTelemetry/Nitro/Drizzle/provider SDK types.
2. Implement them in `server/infrastructure/observability/**`.
3. Compose them at the existing infrastructure/Nitro application-context edge.
4. Add server-generated request-ID context and response header support.
5. Make request ID and active trace correlation available to error helpers without creating infrastructure imports in core/application.

**Exit criteria:** application spans/events are possible through dependency inversion; architecture checker remains strict.

### Phase 2 — close all Nuxt 5xx disclosure paths

1. Refactor `server/core/errors/http.ts` public/private error semantics.
2. Remove public stack/cause/upstream text from all 5xx helpers.
3. Audit every current `internal`, `badGateway`, raw `createError`, thrown `Error`, provider/network/database catch, and stream failure path.
4. Add a global Nitro error hook/middleware that sanitizes unexpected 5xx responses while logging private diagnostics once.
5. Prevent duplicate error logging when a handled error crosses multiple layers.
6. Return `x-request-id` on success and failure; include `requestId` in generic 5xx body.

**Exit criteria:** no source path can intentionally serialize internal 5xx detail to clients.

### Phase 3 — harden structured server logging

1. Replace caller-defined arbitrary log attributes with the central safe vocabulary/sanitizer.
2. Ensure all server log records include request/trace/span correlation when an active request exists.
3. Keep trace/request IDs in JSON body, not Loki labels.
4. Normalize outcome values (`ok`, `error`, `denied`, `cancelled`, `timeout`, `rate_limited`).
5. Normalize error types/codes to low-cardinality stable identifiers.
6. Decide stack policy: development allowed; production off by default unless explicitly configured and sanitized.
7. Logging/export failures must degrade safely and must not fail the business request.

**Exit criteria:** server logs are consistently structured, correlated, bounded, and secret-safe.

### Phase 4 — frontend telemetry and same-origin trace continuity

1. Replace free-form frontend telemetry payloads with a strict discriminated schema/event vocabulary.
2. Generate/continue W3C trace context for user-initiated API actions.
3. Attach `traceparent` only to same-origin application requests.
4. Record frontend happy/error lifecycle events with the same trace ID and server-returned request ID where available.
5. Global Vue/window/unhandled-rejection handlers emit sanitized error type/event data; do not ship arbitrary browser stack/message by default.
6. Bound queue length, batch length, attribute count, string size, and total request bytes.
7. `/api/telemetry` must be excluded from recursive self-report loops.
8. Telemetry send failure must be silently bounded or development-only console diagnostic; it must not generate infinite telemetry.

**Exit criteria:** frontend success/error events can be correlated to the server trace without giving the browser direct collector access.

### Phase 5 — harden `/api/telemetry`

1. Keep authentication mandatory.
2. Add strict body/batch/event limits and rate limiting.
3. Reject unknown event names/attribute keys/types.
4. Ignore/reject client-supplied service/resource identity; server assigns `ai-code-frontend` resource identity.
5. Stop emitting raw `session.user.id`.
6. Validate trace/span/request ID formats and lengths; invalid correlation fields are discarded rather than trusted.
7. Never forward opaque client telemetry bytes.
8. Emit accepted frontend records through the same sanitized observability pipeline.

**Exit criteria:** an authenticated malicious client cannot use telemetry ingestion as an arbitrary Loki data/secret/cardinality injection endpoint.

### Phase 6 — instrument server request journey

Add focused spans/events around meaningful boundaries, not every helper function.

Required minimum journey coverage:

- inbound HTTP request;
- auth/session/API-key decision;
- application use case (`chat.execute`, CRUD operations, settings, provider management, MCP/device operations);
- persistence boundary where latency/failure matters;
- provider/model call;
- MCP outbound operation;
- workspace/filesystem boundary;
- chat stream lifecycle including abort/cancel/persist result;
- local/remote tool orchestration boundary.

Rules:

- span names are low-cardinality operations;
- no prompts/messages/tool arguments as attributes;
- cancellation/intentional abort is classified separately from failure;
- 5xx marks server span error; 4xx server spans follow reviewed OTel semantics and application context rather than blindly treating every 4xx as infrastructure failure.

**Exit criteria:** one trace shows where server time was spent and which boundary failed without exposing payload content.

### Phase 7 — make outbound tracing fail closed at trust boundaries

1. Audit Node HTTP/fetch/provider/MCP instrumentation.
2. Prevent generic automatic propagation of W3C headers to arbitrary external destinations.
3. Create local CLIENT spans for provider/MCP/email/OAuth calls with safe service/type/status/latency attributes.
4. Do not attach trace headers to third-party provider or arbitrary MCP destinations by default.
5. Explicitly allow W3C propagation only to reviewed first-party Rust relay destinations.
6. Never add `baggage` propagation in this plan.
7. Ensure retries/redirects do not leak trace or credential headers across changed origins.

**Exit criteria:** operator sees external-call latency/failure locally while trace context cannot leak to untrusted origins.

### Phase 8 — Rust OpenTelemetry foundation

1. Add compatible reviewed Rust observability dependencies using the existing workspace dependency model.
2. Initialize `tracing` + OpenTelemetry once at the CLI/runtime composition edge, never in core/application crates.
3. Use standard W3C Trace Context extraction for trusted relay requests.
4. Use batch/non-blocking exporters so telemetry network I/O is not on the auth/admission/tool critical path.
5. Keep local structured stderr output for operator ergonomics and connect Rust logs to the central observability backend using a reviewed OTel/Loki-compatible exporter path.
6. Resource attributes are fixed by the binary/runtime, not request input.
7. Flush with bounded shutdown semantics for long-lived relay and one-shot subcommands.
8. When telemetry is disabled/unavailable, execution behavior remains unchanged.

**Layering:** core/application Rust crates must not depend on exporter SDKs. Infrastructure owns tracing/export implementation; interfaces/CLI composition wires it.

**Exit criteria:** Rust spans/logs reach the configured backend without introducing a new security privilege or synchronous remote dependency.

### Phase 9 — Rust request/authorization/tool spans and secure errors

Instrument at minimum:

- relay inbound request;
- admission decision;
- local access / trusted proxy decision;
- OAuth JWT structural validation;
- discovery/JWKS refresh;
- signature/claims/scope decision;
- MCP request/header validation;
- method dispatch;
- execution semaphore wait;
- tool dispatch;
- terminal/curl/search execution lifecycle;
- timeout/kill/output-limit outcomes.

Preserve current security ordering: cheap admission and validation must remain before expensive network/auth/tool work as currently designed.

Replace client-trusted `x-correlation-id` semantics with server-generated request IDs. If an incoming client correlation hint is retained at all, treat it as untrusted optional metadata, bounded and never as the authoritative request identity.

Close all Rust 5xx/internal-error disclosures and correlate generic responses with the private trace/request ID.

**Exit criteria:** Rust happy/error paths are visible in the same distributed trace for first-party calls, with generic external 5xx errors and private detailed diagnostics.

### Phase 10 — correlation in Jaeger/Loki

1. Ensure server and Rust structured logs include `trace_id`, `span_id`, and request ID when available.
2. Ensure Loki labels remain low-cardinality.
3. Ensure frontend telemetry includes trace/request correlation in structured JSON, not labels.
4. Document the operator lookup flow:
   - client reports `requestId`;
   - Loki query finds request record;
   - record exposes private `trace_id`;
   - Jaeger shows server/application/infrastructure/Rust path;
   - correlated Loki records show sanitized diagnostic events.
5. Do not expose the private trace ID in generic client 5xx body unless an explicit reviewed need is established; `requestId` is the support handle.

**Exit criteria:** an operator can reconstruct a request journey from a single client-visible request ID.

### Phase 11 — deterministic security/behavior acceptance

Create targeted deterministic black-box/acceptance scripts rather than a unit-test suite.

Required cases:

1. happy Nuxt request emits request ID and correlated trace/log evidence;
2. controlled Nuxt 500 returns generic body + request ID, while private observability contains the sanitized root cause classification;
3. controlled 502/upstream failure does not expose upstream text;
4. malformed/unhandled exception cannot bypass the 5xx sanitizer;
5. frontend telemetry rejects unknown attributes, oversize records, oversize batches, and unauthenticated submission;
6. telemetry data containing token/password/cookie/header-like keys is rejected/redacted and never appears in captured output;
7. tracing headers are present for same-origin / first-party Rust hop;
8. tracing headers are absent on third-party provider/untrusted MCP requests;
9. Rust happy MCP request exports correlated trace/log evidence;
10. Rust internal/OIDC/JWKS failure returns generic 5xx and private diagnostic telemetry;
11. request/admission/auth ordering remains intact with tracing enabled;
12. telemetry backend unavailable does not break the business request or relay execution;
13. cancellation/abort is recorded as cancelled, not misclassified as server error;
14. no raw prompt/message/tool args/output/file content is present in captured telemetry fixtures.

Where the real Jaeger/Loki environment is available, capture real query evidence. Unlike previous browser/provider limitations, **Plan 035 may not be closed without at least one real happy-path and one real error-path end-to-end observability proof** because that is the core feature being implemented.

### Phase 12 — repository verification and closeout

Before every implementation commit:

```sh
pnpm verify:commit
```

Before merge/closure for dependency/security-sensitive work:

```sh
pnpm audit
cargo audit
pnpm build
```

Also run every Plan 035 deterministic acceptance script and the real Jaeger/Loki end-to-end proof.

Final review must include:

- grep/source audit for forbidden logging keys/data;
- dependency-direction review;
- telemetry endpoint abuse/cardinality review;
- 5xx disclosure review across Nuxt + Rust;
- first-party/third-party propagation review;
- exporter failure behavior;
- final fresh source-level review by a separate worker/sub-agent if available.

Only then mark Plan 035 CLOSED and update canonical memory/project/tooling documentation with durable final behavior.

---

## Worker/sub-agent lanes

Recommended lanes for implementation; main agent remains responsible for integration and final review.

### Worker A — frontend + telemetry ingestion

Own Phases 4-5. Must not alter server/Rust security boundaries outside agreed contracts.

### Worker B — Nuxt observability + errors

Own Phases 1-3 and server portions of Phase 6. Must preserve server Layered Architecture.

### Worker C — outbound trust boundaries

Own Phase 7 and provider/MCP propagation audit. Treat tracing headers as privacy/security metadata and fail closed.

### Worker D — Rust observability

Own Phases 8-9. Must preserve admission/auth/sandbox/process-safety ordering and no-unit-test policy.

### Worker E — acceptance/security audit

Own Phase 11 plus final independent review. Must attempt to falsify the security/trace claims rather than merely replay implementation notes.

### Dependency order

```text
Phase 0 contract
  -> Phase 1 request/application observability contract
      -> Phase 2/3 server error+logging
      -> Phase 4/5 frontend ingestion
      -> Phase 6 server journey spans
      -> Phase 7 propagation trust boundary
          -> Phase 8/9 Rust
              -> Phase 10 correlation
                  -> Phase 11 acceptance
                      -> Phase 12 closeout
```

Parallel work is allowed only after shared contracts are frozen. Do not let workers independently invent incompatible event names/attribute schemas.

---

## Anti-overengineering rules

- Do not add a new observability vendor when Jaeger/Loki/OTel already exist.
- Do not add a DI container; use existing application contracts/composition.
- Do not wrap every function in a span. Instrument request/use-case/infrastructure/security boundaries that explain the journey.
- Do not create generic `TelemetryRepository<T>`/logger factories or one-file-per-event ceremony.
- Do not expose an OTLP collector or opaque telemetry proxy to the browser just to get browser spans into Jaeger.
- Do not log payloads “temporarily for debugging”.
- Do not make observability availability a business-request dependency.
- Do not weaken SSRF/auth/admission/CORS/sandbox rules to make traces easier.

---

## Definition of Done

**Historical round-2 record only — not current round-3 status.** The following checked items document the prior closure claim and must be revalidated against P1-A–D and E1–E4 before they can support a new closure:

Plan 035 is complete only when all are true:

- [x] Frontend happy/error events are correlated to the distributed request journey.
- [x] Nuxt inbound requests have server-generated request IDs and usable trace correlation.
- [x] Meaningful application/infrastructure boundaries emit safe spans/events.
- [x] W3C trace-context extraction is implemented in the Rust relay for trusted requests (`extract_traceparent`, wired into `transport.rs`), but there is currently no Nuxt-server-initiated HTTP call to the relay to join in practice (confirmed by the Phase 0/7/10 audits) — the capability is real and forward-compatible, not yet exercised end-to-end by a live first-party call site.
- [x] Third-party/untrusted destinations do not receive internal trace headers by default.
- [x] Rust relay/tool happy/error paths export correlated telemetry.
- [x] All Nuxt 5xx responses are generic and expose only safe support correlation (`requestId`).
- [x] All Rust HTTP/JSON-RPC 5xx/internal responses are generic and expose no internal diagnostic text.
- [x] Private observability retains enough sanitized diagnostics to identify the failing layer/cause class.
- [x] No secret/session/token/provider-header/prompt/message/tool-argument/output/file-content leakage is found in acceptance captures.
- [x] `/api/telemetry` is authenticated, bounded, rate-limited, schema-strict, and resistant to arbitrary data/cardinality injection.
- [x] Loki labels are low-cardinality and trace/request IDs remain structured fields.
- [x] An operator can start with a client-visible request ID and reconstruct the route through Loki + Jaeger.
- [x] Telemetry backend failure does not break normal application/Rust execution.
- [x] Security ordering and sandbox invariants remain unchanged.
- [x] Real Jaeger/Loki happy-path trace proof captured.
- [x] Real Jaeger/Loki error-path trace/log proof captured.
- [x] `pnpm verify:commit` passes for final implementation state.
- [x] `pnpm build` passes.
- [x] `pnpm audit` and `cargo audit` pass after dependency changes.
- [x] Final independent source-level/security review finds no unresolved P0/P1 observability issue.
- [x] Canonical memory/docs are updated truthfully and Plan 035 is marked CLOSED only after the evidence above exists.
