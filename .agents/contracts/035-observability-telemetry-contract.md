# Plan 035 Phase 0 — Observability & Telemetry Contract (Frozen)

**Status:** Frozen for Plan 035 Phase 1+. Documentation-only; no runtime behavior changed by this file.
**Baseline commit:** `0d3b1cc701e71c61775376c4dcdb8cd74619ab73` (branch `feat/035-p0-observability-contract`, based on `dev`).

This document is the deterministic matrix required by Plan 035 Phase 0 exit criteria: what is traced, what is logged, what is never logged, and where trace propagation is allowed. Every claim below was verified by reading current source at the baseline commit, not paraphrased from the plan text.

---

## 1. Baseline audit summary (current state, with citations)

### 1.1 Frontend telemetry composable — `app/composables/useTelemetry.ts`

- Lines 2-3: module-level singleton `batch` ref and `isInitialized` flag.
- Lines 5-20 (`flush`): serializes the entire batch to JSON and POSTs to `/api/telemetry` via `navigator.sendBeacon` (preferred) or `fetch(..., keepalive: true)`. No batch size cap, no record count cap, no byte-size cap.
- Lines 22-29 (`logEvent`): accepts a free-form `attributes?: Record<string, unknown>` with **no schema/allowlist** and pushes it straight into the batch.
- Lines 31-39 (`logError`): forwards `error.message` as the log message and the **raw browser stack trace** (`error.stack`) as an attribute, unredacted.
- Line 18: transport failure falls back to `console.error`, which Plan 035 already flags as a recursive/noisy-observability risk (`console.error` itself is not intercepted by this composable, so it is bounded today, but the plan's stated gap is confirmed: no dedicated failure-handling policy exists).
- Lines 44-50: flush is scheduled every 5s (`setInterval`) plus on `visibilitychange`/`beforeunload`. No rate limiting.

### 1.2 Telemetry ingestion — `server/api/telemetry.post.ts`

- Lines 3-8: `logEventSchema` validates only `{ level: string, message: string, attributes?: record<string, any>, timestamp?: number }` — `attributes` accepts **arbitrary keys and arbitrary value types**, unbounded.
- Line 10: `telemetrySchema` is `v.array(logEventSchema)` — **no max array length**, so an authenticated caller can submit an unbounded batch in one request.
- Line 13: `requireUserSession(event)` — authentication is mandatory (confirmed).
- Lines 19-51: for every record, severity is mapped and the record is emitted via `event.context.application.observability.logger.emit(...)`.
- Lines 45-49: attributes are spread verbatim (`...log.attributes`) into the emitted log record **and** `'userId': session.user?.id` is added — i.e. the **raw internal user ID** is written into telemetry data, confirming the plan's stated gap.
- No rate limiting, no attribute-key allowlist, no string-length caps, no rejection of unknown/forbidden keys (e.g. nothing stops a client submitting `attributes: { authorization: "...", password: "..." }`, since the schema is `v.record(v.string(), v.any())`).

### 1.3 Nuxt OTel/tracing bootstrap — `otel-preload.mjs`

- Lines 13-46: gated by `NUXT_OTEL_ENABLED === 'true'`. Loaded via Node `--import` preload (see Dockerfile `CMD`) so `HttpInstrumentation` patches `node:http`/`node:https` before Nitro's own `import 'node:http'`.
- Line 45: `registerInstrumentations({ instrumentations: [new HttpInstrumentation()] })` — this is **generic automatic outgoing HTTP instrumentation with no destination allowlist**. It will create spans (and, per default OTel HTTP instrumentation behavior, propagate `traceparent`/W3C headers) for **every** outbound `node:http`/`node:https` call the process makes, including model-provider calls, MCP remote server calls, OAuth provider calls, and SMTP — i.e. exactly the "outgoing auto-instrumentation/trace-header propagation is not yet governed by an explicit first-party trust policy" gap the plan names. No per-destination filtering config (`ignoreOutgoingRequestHook`, `requireParentforOutgoingSpans`, etc.) is present in this file.
- Lines 38-41: `SimpleSpanProcessor` (not batching) is used deliberately per the inline comment (Nitro-externalized `BatchSpanProcessor` reportedly never exports there).

### 1.4 Server structured logging — `server/infrastructure/observability/logger.ts`

- Lines 33-54: `logger.error/warn/info/forwardOnly` all print via `consola` (stdout) and also call `emit()` (line 24-31) which forwards to `getLogger('ai-code-server').emit(...)` (OTel Logs API, no-ops when OTel disabled per `otel.ts` line 6-20).
- Lines 18-22 (`errorAttributes`): when `err` is an `Error`, attributes include `{ error: err.message, stack: err.stack }` **unconditionally** — no environment/production gating, no redaction. Any caller passing an `Error` to `logger.error/warn` leaks the raw message and full stack into the OTel/Loki pipeline. This is the server-log-side twin of the `http.ts` client-facing leak (see §1.5).
- `server/infrastructure/observability/otel.ts` lines 44-99 (`LokiLogExporter.export`): Loki stream labels are `{ job: serviceName, level }` only (line 50-53) — confirmed **low-cardinality by construction today**. But `linePayload.attributes = log.attributes` (line 65) puts the **entire unredacted attributes object** (including whatever `logger.ts`/callers passed, with no allowlist) into the Loki JSON body. `trace_id`/`span_id` are correctly placed in the body, not labels (lines 68-73), matching the plan's cardinality rule for that one piece.

### 1.5 5xx behavior — `server/core/errors/http.ts` and `server/core/errors/index.ts`

- `http.ts` lines 19-40 (`problem()`): every `ProblemInit.detail` and `.extra` is placed **directly into the client-visible response body** (`data: { ..., detail: init.detail, ...init.extra }`), regardless of status code. There is no `status >= 500` branch that strips `detail`/`extra` before serialization.
- `http.ts` lines 62-65 (`internal(cause)`): `detail = cause instanceof Error ? cause.message : String(cause)` and `extra = { stack: cause.stack }` when present — **both flow into the public 500 response via `problem()`**. This is a confirmed, current P0 violation of Plan 035 requirement #3 (raw `cause.message` and stack trace are client-visible on every unhandled `internal(err)` call site).
- `http.ts` line 48 (`badGateway(detail)`): callers pass free-text `detail`, e.g. `server/infrastructure/composition/application.ts:113`: ``throw badGateway(`Could not reach ${provider.name}: ${(error as Error).message}`)`` — this puts the **raw provider error message** into the public 502 body.
- `server/core/errors/index.ts` (Nitro global error handler, wired at `nuxt.config.ts:99` `errorHandler: '~~/server/core/errors/index'`): lines 15-50. For `isProblem` responses (anything thrown via `problem()`/its helpers) the handler passes `data.detail` and spread `extra` straight through to the client body (line 37) — i.e. **the global handler does not re-sanitize `problem()` output**; it trusts `http.ts` already produced a safe body, which per the finding above it does not for `internal`/`badGateway`. For non-problem, non-trusted-title errors (raw exceptions/`fatal`/`unhandled`), the handler correctly falls back to a generic `{ type: 'about:blank', title: 'Internal Server Error', status: 500 }` body (lines 29, 40) and logs full detail server-side only (line 43) — that specific path is already compliant.
- Confirmed direct `internal(...)`/`badGateway(...)` call sites (5xx-producing, `grep` over `server/api`, `server/infrastructure`, `server/core`):
  - `server/api/auth/register.post.ts:48`
  - `server/api/conversations/index.post.ts:34`
  - `server/infrastructure/database/models.ts:39`
  - `server/infrastructure/database/settings.ts:52`
  - `server/infrastructure/database/mcp-servers.ts:57`
  - `server/infrastructure/database/chat.ts:42`, `:62`
  - `server/infrastructure/database/workspaces.ts:69`
  - `server/infrastructure/database/providers.ts:65`
  - `server/infrastructure/composition/application.ts:113` (`badGateway`, embeds provider error message)
  - `server/infrastructure/filesystem/browse.ts:24` (`createError({ statusCode: 500, ... })`, static message, not `internal()` — safe today but bypasses the `problem()` request-ID/telemetry path entirely)

### 1.6 Rust observability — `packages/rust-tools/infrastructure/src/observability.rs`

- Lines 6-7: `MAX_LOG_FIELD = 128`, correlation header name `x-correlation-id`.
- Lines 28-43 (`CorrelationId::from_headers`): accepts a **client-supplied `x-correlation-id`** if it is ≤128 chars and ASCII-graphic-without-quotes; otherwise generates a UUID. This is exactly the plan's named gap — client-controlled correlation IDs remain part of operator telemetry with only light validation, not a server-only-authoritative request ID (Plan 035 Phase 9 requires this to become server-generated, with any client hint treated as untrusted optional metadata).
- Lines 56-70 (`audit(...)`): writes one JSON line to stderr per request via `eprintln!`. Fields: `correlation_id` (sanitized via `safe_log_field`, truncated/control-char-stripped, but **not otherwise rate-limited or size-capped beyond 128 chars**), `method`, `tool` (privacy-reduced to `present`/`absent` via `privacy_id`, line 17-23), `outcome`, `status`, `latency_ms`, `subject` (also privacy-reduced). No OpenTelemetry exporter exists in this file — confirmed, matches plan's stated gap ("no OpenTelemetry trace/log exporter or W3C trace-context extraction").
- No connection between this stderr audit stream and the Nuxt trace — confirmed gap.

### 1.7 Rust relay transport 5xx behavior — `packages/rust-tools/infrastructure/src/transport.rs`

- Lines 299-325: when `oauth_issuer`/`oauth_audience` config is missing, returns `StatusCode::INTERNAL_SERVER_ERROR` with `McpError::Internal("oauth_issuer is required for Remote mode")` / `"oauth_audience is required for Remote mode"` — **static text, not attacker/upstream-controlled, but still an internal configuration detail exposed on the wire**.
- Lines 393-428: on OIDC discovery/JWKS failure, returns `StatusCode::INTERNAL_SERVER_ERROR` with **`McpError::Internal(format!("OIDC discovery unavailable: {msg}"))`** (line 401) where `msg` is the raw error string from `auth::fetch_discovery(...)` — this directly embeds upstream/network/OIDC failure detail (potentially including hostnames, TLS errors, HTTP status text) into the **client-visible JSON-RPC/HTTP error body**. Same pattern at lines 410-416 (`"OIDC discovery returned an invalid jwks_uri"`, static) and 420-426 (`"OIDC discovery returned an unsafe jwks_uri"`, static). The `format!("OIDC discovery unavailable: {msg}")` case is the confirmed P0 raw-upstream-detail leak; the other two are static strings (lower severity but still not the RFC-9457-style generic body the plan requires).
- Line 200-209 (`err_response`/`json_error_response`): all error paths funnel through `ErrorResponse::new(id, &McpError)`, i.e. **there is exactly one serialization chokepoint** — good for a future fix, but today `McpError::Internal(String)` carries arbitrary free text straight into that serialization with no separation between "public-safe" and "operator-diagnostic" content.
- Confirmed additional `StatusCode::INTERNAL_SERVER_ERROR` sites: lines 303, 317, 398, 410, 421, 441 (six total in this file as of baseline).
- Non-5xx error paths (401 `oauth_error_response` at lines 346, 366, 455, 482, 495, 515, 525, 535, 571; 400 `err_response`/`ParseError`/`InvalidRequest` at lines 627, 659, 685; 404 `MethodNotFound` at line 699-702) already use structured `McpError` variants without embedding raw upstream text — these are lower risk but still worth re-auditing in Phase 9 for consistency.
- `correlation_middleware` (lines 192-198) inserts the correlation ID into every response header (`insert_response_header`) regardless of status — this is the existing (client-influenceable) analog of the plan's `x-request-id` requirement; Phase 9 must replace this with a server-generated request ID while optionally retaining the client hint as untrusted metadata only.

---

## 2. Event/attribute vocabulary

### 2.1 Allowed stable attributes (adopted verbatim from Plan 035)

```
service.name
deployment.environment
component
layer
event.name
operation
outcome
request.id
http.request.method
route (low-cardinality template, e.g. "/api/conversations/:id", never the raw resolved path with IDs)
http.response.status_code
duration_ms
error.type
error.code
provider.type        (low-cardinality: e.g. "openai-compatible", "anthropic-compatible", "vertex-ai", "langgraph")
tool.name             (low-cardinality: registered tool identifier, e.g. "terminal", "curl", "searxng-search", MCP tool name — never tool arguments)
mcp.method             (low-cardinality JSON-RPC method: "initialize", "tools/list", "tools/call")
attempt / retry_count
auth.present           (boolean presence, not the credential)
```

Trace ID / span ID are correlation fields inside structured log bodies (already true for `LokiLogExporter`, §1.4), never Loki labels.

### 2.2 Stable low-cardinality operation/event names for this codebase

Derived from actual route/use-case boundaries found in `server/api/**`, `server/application/**`, `server/infrastructure/**`, and the Rust dispatcher (`packages/rust-tools`, per `Dispatch::{Discover,ToolsList,ToolsCall,Unknown}` in `transport.rs:692-704`):

**Chat / streaming** (`server/api/chat.post.ts`, `server/application/chat/execute-chat-turn.ts`, `server/infrastructure/ai/*`):
- `chat.execute`
- `chat.stream.start` / `chat.stream.chunk_error` / `chat.stream.abort` / `chat.stream.persist`
- `chat.tool.local_terminal.dispatch` (local relay-agent tool path, `server/infrastructure/ai/local-terminal-tool.ts`)
- `chat.tool.mcp.dispatch`

**Provider CRUD** (`server/api/providers/**`, `server/infrastructure/database/providers.ts`, `server/infrastructure/ai/providers/*`):
- `provider.create`, `provider.update`, `provider.delete`, `provider.list`
- `provider.discover_models` (`server/api/providers/[id]/models.get.ts`)
- `provider.reachability_check` (used by `application.ts:113`'s `badGateway`)

**Model CRUD** (`server/api/models/**`):
- `model.create`, `model.update`, `model.delete`, `model.list`

**MCP server management + calls** (`server/api/mcp-servers/**`, `server/api/mcp/index.ts`, `server/infrastructure/mcp/*`):
- `mcp_server.create`, `mcp_server.update`, `mcp_server.delete`, `mcp_server.test`
- `mcp.tools_list`, `mcp.tools_call`
- `mcp.inbound.dispatch` (server acting as an MCP endpoint, `server/api/mcp/index.ts`)

**Workspace ops** (`server/api/workspaces/**`, `server/infrastructure/database/workspaces.ts`, `server/infrastructure/filesystem/browse.ts`):
- `workspace.create`, `workspace.update`, `workspace.delete`, `workspace.set_active`, `workspace.list`
- `workspace.fs.browse`

**Conversations** (`server/api/conversations/**`):
- `conversation.create`, `conversation.update`, `conversation.delete`, `conversation.get`, `conversation.list`

**Auth / identity** (`server/api/auth/**`, `server/api/api-keys/**`, `server/api/devices/**`, `server/routes/auth/{google,github}.get.ts`):
- `auth.login`, `auth.register`, `auth.logout`, `auth.forgot_password`, `auth.reset_password`, `auth.verify_email`
- `auth.oauth.google`, `auth.oauth.github`
- `api_key.create`, `api_key.delete`, `api_key.list`
- `device.revoke`, `device.list`

**Settings** (`server/api/settings*.ts`):
- `settings.get`, `settings.update`

**Telemetry ingestion itself** (`server/api/telemetry.post.ts`) — must be excluded from self-instrumentation per Plan 035 Phase 4 item 7:
- `telemetry.ingest` (server-side only; never causes `/api/telemetry` to call itself)

**Rust relay** (`packages/rust-tools/infrastructure/src/transport.rs`, `observability.rs`):
- `relay.auth.validate` (bearer presence/structural JWT check, lines 339-375)
- `relay.auth.discovery` (OIDC discovery/JWKS refresh, lines 393-450)
- `relay.mcp.initialize`, `relay.mcp.tools_list`, `relay.mcp.tools_call`, `relay.mcp.discover` (matches `Dispatch` variants)
- `relay.tool.dispatch` (per-tool execution boundary inside `handle_tools_call`)
- `relay.request` (the existing `audit()` event name at `observability.rs:67`, keep as the top-level per-request event)

---

## 3. Forbidden-data matrix

| Forbidden field (Plan 035) | Current status in this codebase | Evidence |
|---|---|---|
| `Authorization` header value | **NOT logged** today — `transport.rs` reads it (line 329) only to check `Bearer ` prefix / extract token, never logs the raw header. `server/api/mcp/index.ts:14` only checks `startsWith('Bearer ')`. Verified via grep for `Authorization`/`authHeader` logging call sites — none found. | grep, `transport.rs:327-356`, `server/api/mcp/index.ts:14` |
| `Cookie` / `Set-Cookie` | **NOT logged** — no grep hits combining cookie values with `logger.*`/`console.*`/`audit(`. | grep |
| `x-api-key`, provider custom-header values | **NOT logged directly**, but `server/infrastructure/auth/api-key.ts:29` throws a generic `createError` on invalid key with no key material in the message — safe today. Provider API keys stored via `server/infrastructure/database/providers.ts` are not currently observed being logged. | `server/infrastructure/auth/api-key.ts:29` |
| Session IDs, bearer/access/refresh tokens, OAuth codes, PKCE verifier/state | **NOT logged** in `nuxt-auth-utils` session flows or `server/routes/auth/{google,github}.get.ts` `onError` handlers (not yet read in full, but no logger call sites take token values as arguments per grep). Flag for explicit re-verification in Phase 3/7. | grep (partial) |
| Provider/API keys, passwords, encryption keys, DB URLs/connection strings | **NOT logged** — `server/infrastructure/mail/mailer.ts` logs `{ to, subject }` only (line 19), never SMTP credentials. | `mailer.ts:19` |
| Raw request/response bodies | **AT RISK** — `server/api/telemetry.post.ts` has no attribute-value redaction, so a malicious authenticated client could submit body-shaped content inside `attributes` and it would be emitted to Loki verbatim (line 45-49). This is the confirmed Phase 5 target. | `telemetry.post.ts:45-49` |
| Prompts, chat messages, reasoning text, model output | **NOT currently logged** in `server/infrastructure/ai/*` per grep (no `logger.*(message.content` or similar found), but `logger.ts`'s `errorAttributes` (line 18-22) would capture **any** `Error.message` passed to it, and AI SDK/provider errors sometimes embed request/response fragments in `.message` — this is an indirect risk surface, not confirmed leaking today but not structurally prevented either. | `logger.ts:18-22` |
| MCP/tool arguments or outputs | **NOT logged** server-side (no call sites found passing tool args to `logger.*`). Rust `observability.rs:17-23` `privacy_id()` deliberately reduces `tool`/`subject` to `present`/`absent` rather than raw values — this is a good existing pattern to keep. | `observability.rs:17-23` |
| Terminal commands, shell input, file contents | **NOT logged** — `local-terminal-tool.ts` only builds a tool *definition* (description/schema), no execution/logging of command content in this repo (execution happens client-side in the paired relay-agent CLI). | `local-terminal-tool.ts` |
| Raw workspace/cwd/filesystem paths | **AT RISK, confirmed leaking today** — `server/infrastructure/filesystem/browse.ts` and `server/infrastructure/database/workspaces.ts` throw `createError({ statusCode: 4xx, statusMessage: 'Path is not a directory' / 'Path traversal detected' })` with **static** messages (safe), but underlying path values are not currently added as log/span attributes anywhere found — flag for explicit denylist enforcement once span attributes are added in Phase 6, since it would be easy to accidentally add `{ path }` as a "helpful" attribute later. | `filesystem/browse.ts:24,40,53`; `database/workspaces.ts` |
| Arbitrary full URLs or query strings | **AT RISK** — `badGateway` at `application.ts:113` embeds `provider.name` (not a full URL, low risk) plus `error.message`, which for network errors frequently **does** contain the target URL (e.g. Node `fetch failed` / `ECONNREFUSED <host>:<port>` messages). This is part of the same confirmed `internal`/`badGateway` leak in §1.5. | `application.ts:113` |
| Email/name or other direct PII | **AT RISK** — `mailer.ts:19` logs `{ to, subject }` on `logger.warn` when SMTP is unconfigured — `to` is a raw email address in structured log attributes today. This must move to a redacted/pseudonymous form or be dropped under the Phase 3 sanitizer. | `mailer.ts:19` |
| Raw user/tenant identifiers by default | **CONFIRMED leaking** — `server/api/telemetry.post.ts:47` writes `'userId': session.user?.id` (raw internal user ID) into every ingested telemetry record's attributes, which flow to Loki via `LokiLogExporter` body (`otel.ts:65`). This is the exact violation named in the plan and the clearest Phase 5 fix target. | `telemetry.post.ts:47`; `otel.ts:65` |
| JWKS documents or token claims | **AT RISK, partially confirmed** — Rust `transport.rs:401` embeds raw OIDC discovery error text (which can include response bodies/claims-adjacent detail depending on the upstream failure) into the **client-visible** 500 body, i.e. worse than a private-log-only leak. Token claims themselves (`auth_ctx.claims`) are only passed to `audit()` as `subject` via `privacy_id()` (present/absent), which is correctly redacted. | `transport.rs:394-406`; `observability.rs:63,69` |

**Summary:** Two confirmed P0 client-visible leaks exist today (Nuxt `internal()`/`badGateway()` in `http.ts`, Rust OIDC-discovery-error `format!` in `transport.rs:401`); one confirmed private-log PII/identifier leak (`telemetry.post.ts` raw `userId` + unredacted `attributes`); one lower-risk private-log PII leak (`mailer.ts` raw email in log attributes); remainder are currently clean but structurally unprotected against future regressions (no allowlist/sanitizer exists yet — that is the Phase 1-3/5 deliverable).

---

## 4. First-party vs third-party trace-propagation destinations

All outbound HTTP/fetch call sites found in `server/**` (excluding the Loki/OTLP exporters themselves, which are operator-facing infrastructure, not user-data destinations):

| Destination | Call site | Classification | Propagation policy |
|---|---|---|---|
| OpenAI-compatible provider APIs (user-configured `baseUrl`) | `server/infrastructure/ai/providers/openai-compatible.ts:8,27` | **Third-party** (user-configured arbitrary base URL) | No `traceparent`/`tracestate`/auth metadata; local CLIENT span only |
| Anthropic-compatible provider APIs (user-configured `baseUrl`) | `server/infrastructure/ai/providers/anthropic-compatible.ts:7,24` | **Third-party** | Same as above |
| Vertex AI | `server/infrastructure/ai/providers/vertex-ai.ts` | **Third-party** | Same as above |
| LangGraph-wrapped model provider | `server/infrastructure/ai/providers/langgraph-model.ts:19` (`baseURL: provider.baseUrl`) | **Third-party** (delegates to user-configured provider base URL) | Same as above |
| Remote MCP servers (user-configured `serverConfig.url`) | `server/infrastructure/mcp/client.ts:41` | **Third-party** (arbitrary remote MCP endpoint, SSRF-guarded via `ssrf-guard.ts`) | No trace headers; local CLIENT span with method/status/latency only |
| SMTP (nodemailer transport) | `server/infrastructure/mail/mailer.ts:7-14` | **Third-party** (not HTTP/traceparent-carrying protocol, but still an external boundary) | No trace context applies (SMTP, not HTTP); local CLIENT span for send success/failure/latency |
| Google OAuth | `server/routes/auth/google.get.ts` | **Third-party** | No trace headers to Google; local CLIENT span |
| GitHub OAuth | `server/routes/auth/github.get.ts` | **Third-party** | No trace headers to GitHub; local CLIENT span |
| Loki push (`LokiLogExporter`) | `server/infrastructure/observability/otel.ts:85-99` | **First-party/operator infrastructure** (not user data destination — the observability backend itself) | Not a propagation-policy target; this *is* the export sink |
| OTLP/Jaeger gRPC trace export | `otel-preload.mjs:30-32` | **First-party/operator infrastructure** | Not a propagation-policy target |
| Rust relay-agent (paired local terminal execution boundary) | Referenced from `server/application/chat/local-terminal-policy.ts`, `server/infrastructure/database/devices.ts`, `server/application/chat/execute-chat-turn.ts` — **no direct outbound HTTP call from Nuxt server to the relay found in `server/infrastructure/**`**; the relay is reached from the user's own paired local CLI, not server-initiated HTTP, per `local-terminal-tool.ts:4` docstring ("a loopback bridge — this server never runs the command itself") | **First-party** (same operator/security boundary), but **not a Nuxt-server-initiated outbound HTTP call today** — trace propagation to it would need a different mechanism than HTTP header injection (needs design work in Phase 7/9, flagged as open item) | No W3C header propagation exists today because no direct HTTP call exists; do not assume the plan's "Nuxt -> first-party Rust relay endpoint" language maps to an existing call site — confirm actual transport in Phase 7 before implementing |

**Important correction to plan assumption:** Plan 035's target model describes "Nuxt -> explicitly identified first-party/local Rust relay endpoints" as an allowed first-party propagation destination. Grep across `server/infrastructure/**` and `server/application/**` for `relay`/`RELAY_URL`/outbound `fetch`/`ws://`/`wss://` found **no current server-initiated network call to a Rust relay endpoint** — the relay-agent is paired and driven from the user's own machine, and the AI SDK tool definition in `local-terminal-tool.ts` only declares a tool schema for the model to call; execution happens outside this server's outbound network boundary. Phase 7/9 workers must re-verify this before assuming an existing first-party HTTP hop needs propagation wiring; if none exists, the "first-party Rust relay" propagation requirement may not apply to the Nuxt server's outbound calls at all in the current architecture, only to inbound requests reaching the relay directly from a paired client.

**Generic auto-instrumentation gap (§1.3):** `otel-preload.mjs:45`'s `HttpInstrumentation()` has no destination allowlist today, so if left unconfigured it will attempt to propagate trace headers to *all* of the above third-party destinations. Phase 7 must add explicit `ignoreOutgoingRequestHook`/manual instrumentation to enforce the first-party/third-party split.

---

## 5. Every current 5xx-producing helper/path (Nuxt + Rust)

### Nuxt (`server/core/errors/http.ts` + call sites + global handler)

| Helper/path | Status | Public detail leak? |
|---|---|---|
| `problem()` — `http.ts:19-40` | Generic factory, all statuses | **Yes for any caller passing `detail`/`extra` on status ≥500** — no status-based stripping |
| `internal(cause)` — `http.ts:62-65` | 500 | **Confirmed leak** — `cause.message` and `cause.stack` both public |
| `badGateway(detail)` — `http.ts:48` | 502 | **Confirmed leak at call site** `application.ts:113` (embeds `error.message`) |
| Direct `internal(...)` call sites | 500 | See list in §1.5 (`register.post.ts:48`, `conversations/index.post.ts:34`, `models.ts:39`, `settings.ts:52`, `mcp-servers.ts:57`, `chat.ts:42,62`, `workspaces.ts:69`, `providers.ts:65`) — all pass a **static string literal** today (e.g. `'Failed to create conversation'`), so **no current leak from these specific call sites**, but they all route through the leaking `internal()` implementation, so any future call site passing a real `Error`/`cause` will leak immediately without another review gate |
| `createError({ statusCode: 500, ... })` direct (bypasses `problem()`) — `server/infrastructure/filesystem/browse.ts:24` | 500 | Static message `'NUXT_WORKSPACES_ROOT is not configured'` — no leak, but bypasses request-ID/telemetry path since it skips `problem()` |
| Global Nitro error handler — `server/core/errors/index.ts:15-50`, wired at `nuxt.config.ts:99` | Catches everything | For `isProblem` (i.e. anything from `problem()`/helpers): **passes through whatever `http.ts` already produced**, including the `internal()`/`badGateway()` leaks above (line 37). For raw/unhandled exceptions not going through `problem()`: **compliant today** — generic body, full detail server-log-only (lines 29,40,43) |

### Rust (`packages/rust-tools/infrastructure/src/transport.rs`)

| Line(s) | Status | Public detail leak? |
|---|---|---|
| 299-311 | 500 `INTERNAL_SERVER_ERROR` | Static text `"oauth_issuer is required for Remote mode"` — no dynamic leak, but exposes internal config-state |
| 313-325 | 500 | Static text `"oauth_audience is required for Remote mode"` — same |
| 393-406 | 500 | **Confirmed leak** — `format!("OIDC discovery unavailable: {msg}")` embeds raw upstream/network error text |
| 407-419 | 500 | Static text `"OIDC discovery returned an invalid jwks_uri"` — no dynamic leak |
| 420-428 | 500 | Static text `"OIDC discovery returned an unsafe jwks_uri"` — no dynamic leak |
| (JWKS fetch/signature validation continuing past line 429, not fully enumerated in this pass — flag for Phase 9 full re-audit) | likely 500 on fetch/parse failure | Not yet verified line-by-line past 429; Phase 9 must re-audit the remainder of `auth.rs`/`transport.rs` for the same `format!(...{msg})` pattern |
| `err_response`/`json_error_response` chokepoint (200-209) | all JSON-RPC errors | Single serialization point — good target for a future public/private split, not itself a leak |

---

## 6. Maximum telemetry sizes/rates (proposed, for later phases)

These are proposed concrete numbers consistent with Plan 035's "bounded" requirement; not yet implemented (Phase 4/5 work).

**Frontend (`useTelemetry.ts` / `/api/telemetry`):**
- Max attribute string length: **256 characters** (truncate, do not reject the whole record)
- Max attribute key count per record: **16 keys**, allowlist-only (unknown keys rejected, not silently dropped-and-accepted)
- Max event/message string length: **512 characters**
- Max records per batch: **50**
- Max total batch payload size: **64 KB** (`sendBeacon`'s own practical ~64KB ceiling makes this a natural cap; enforce explicitly server-side too)
- Max flush frequency: **1 flush per 5s per client** (already the interval; add a server-side rate limit of e.g. **20 requests/minute per authenticated user** via the existing `rate-limit.ts` primitive)
- Reject (400, not silently truncate-and-accept) any batch exceeding count/byte caps

**Server-side structured logs (`logger.ts` / OTel):**
- Max attribute value string length: **512 characters** (longer than frontend since server-authored messages are trusted, but still bounded against accidental huge payload/stack dumps)
- Max attribute count per log record: **20**
- Stack trace inclusion: development only by default (`NODE_ENV !== 'production'`); production requires explicit opt-in env flag, and even then stack goes to structured body, never public response

**Rust (`observability.rs` / future OTel exporter):**
- Existing `MAX_LOG_FIELD = 128` (line 6) stays for stderr audit fields; align future OTel span/log attribute cap to the same **128 characters** for consistency, or raise to **256** to match frontend if operator readability requires more — final number decided in Phase 8, but must not exceed frontend's 256 without explicit reason
- Batch export size: use OTel SDK default batch processor limits (max 512 spans/batch, 2048 queue size) — do not hand-roll a custom batcher per the plan's "do not add a new observability vendor" anti-overengineering rule
- Exporter timeout: **5 seconds**, non-blocking relative to the auth/admission/tool critical path (per Plan 035 Phase 8 item 4)

---

## Open items for later phases (not blocking Phase 0 closure)

1. Confirm whether any Nuxt-server-initiated HTTP call to the Rust relay exists anywhere outside `server/infrastructure/**`/`server/application/**` before Phase 7/9 implement first-party propagation to it (see §4 correction).
2. Full line-by-line re-audit of `transport.rs` past line 429 (JWKS fetch/parse/signature validation) and of `auth.rs` for the same raw-upstream-message-in-500-body pattern.
3. Re-verify `server/routes/auth/{google,github}.get.ts` `onError` handlers do not log/return token/code values (not fully read in this pass).
4. Decide final Rust telemetry attribute size cap (128 vs 256) in Phase 8.
