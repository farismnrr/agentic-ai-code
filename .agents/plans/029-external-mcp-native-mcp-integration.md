# Plan 029 — external MCP client / external MCP client Native MCP Connector Integration

**Status: IN FLIGHT**

## Goal

Turn the existing Plan 028 Rust Relay Agent into a production-grade **remote MCP app/connector endpoint** that works natively with external MCP client and external MCP client without duplicating the MCP core, OAuth resource-server logic, or execution sandbox already implemented.

Plan 029 is explicitly a **delta/integration plan**, not a second MCP implementation. Existing Plan 028 behavior remains authoritative unless this plan identifies a concrete compatibility gap.

The primary target is the current MCP `2026-07-28` protocol and external MCP client's current custom MCP app flow. external MCP client/external MCP client compatibility is validated at the client boundary; client-specific code must not fork tool execution, authorization, or sandbox logic.

## Current implementation baseline — DO NOT REIMPLEMENT

The following already exists in `dev` and must be reused rather than rebuilt:

- [x] Rust `relay-agent` binary and transport-independent MCP core.
- [x] Canonical `POST /mcp` JSON-RPC endpoint.
- [x] MCP `2026-07-28` protocol version constant.
- [x] Stateless request model; no legacy `Mcp-Session-Id` authorization boundary.
- [x] `MCP-Protocol-Version`, `Mcp-Method`, and `Mcp-Name` request validation.
- [x] Request `_meta` parsing/validation.
- [x] `server/discover`.
- [x] `tools/list`.
- [x] `tools/call` dispatch.
- [x] JSON Schema 2020-12-compatible tool input validation before execution.
- [x] Existing tools: `terminal_exec`, `http_fetch`, `web_search`.
- [x] Local-mode fail-closed `Origin` + `Host` policy.
- [x] Explicit local vs remote security mode.
- [x] OAuth Resource Server boundary in remote mode.
- [x] Bearer-token validation.
- [x] Asymmetric JWKS validation.
- [x] JWKS TTL cache.
- [x] Refresh-on-unknown-`kid`.
- [x] JWKS fetch timeout/fail-closed behavior.
- [x] issuer + audience/resource validation foundation.
- [x] Plan 028 execution sandbox/resource limits remain authoritative.
- [x] Zero-warning Rust CI/release quality gates from Plan 028 remain authoritative.

If implementation review proves any item above is incomplete, reopen only that concrete gap; do not duplicate the subsystem in Plan 029.

---

## Non-goals

- Do not build a bespoke OAuth Authorization Server in Rust.
- Do not create a external MCP client-only REST API or legacy external MCP client Plugin API.
- Do not create a second external MCP client-specific MCP server.
- Do not replace or weaken the Plan 028 filesystem/process/container sandbox.
- Do not duplicate existing JWKS/token validation just to satisfy external MCP client setup UI.
- Do not make external MCP client confirmation prompts a security boundary.
- Do not expose local no-auth mode to the public internet.
- Do not reintroduce deprecated HTTP+SSE as the canonical protocol.
- Do not require new unit tests for this deadline; use strict compile/lint/audit, protocol checks, targeted integration/E2E, and manual security review.

---

# Architecture

```text
external MCP client / external MCP client
        |
        | MCP over HTTPS
        v
Canonical /mcp endpoint
        |
        +--> OAuth Protected Resource discovery
        |       |
        |       v
        |   External IdP / Authorization Server
        |       |
        |       +--> Authorization Code + PKCE S256
        |       +--> refresh/offline access
        |       +--> OIDC optional
        |
        +--> Access token
                |
                v
        Rust Relay Agent
        OAuth Resource Server
                |
                +--> issuer/resource/audience validation
                +--> subject policy
                +--> scope -> tool authorization
                |
                v
        Existing MCP tool core
                |
                v
        Plan 028 sandbox/execution boundary
```

## Security invariant

A external MCP client/external MCP client connection may change **transport/authentication context**, but it must never create a weaker execution path.

Every side effect must preserve:

```text
request
  -> MCP validation
  -> authentication
  -> subject authorization
  -> scope/tool authorization
  -> schema validation
  -> Plan 028 sandbox/resource policy
  -> side effect
```

Tool annotations, external MCP client confirmations, tool descriptions, and connector UI metadata are **advisory/client UX**, not security boundaries. They MUST NOT substitute for OAuth scope enforcement, subject checks, or the Plan 028 sandbox.

---

# external MCP client compatibility contract

external MCP client's current custom MCP app UI exposes these integration concepts and Plan 029 must explicitly support or intentionally document each one.

| Capability | Requirement | Plan 029 treatment |
|---|---|---|
| Remote MCP Server URL | MUST | Canonical HTTPS `/mcp` URL |
| Tool discovery/scan | MUST | Reuse `tools/list`, harden descriptors |
| Tool invocation | MUST | Reuse `tools/call` |
| User-defined OAuth client | MUST | Supported/config documented |
| Dynamic Client Registration (DCR) | SHOULD | Compatibility path only; advertise only when external IdP actually supports it |
| Client Identifier Metadata Document (CIMD) | SHOULD / PREFERRED | Preferred registration direction when supported by current MCP + external MCP client flow |
| Authorization endpoint | MUST for OAuth | Discover from Authorization Server metadata |
| Token endpoint | MUST for OAuth | Discover from Authorization Server metadata |
| Registration endpoint | CONDITIONAL | Required for DCR only |
| Authorization Server base/issuer | MUST | Stable trusted issuer |
| Resource identifier | MUST | Stable canonical MCP resource |
| Base scopes | MUST define | Small minimum scopes requested on every auth |
| Default scopes | MUST define | Fallback when action-level scopes are incomplete |
| Action/tool scopes | MUST | Tool descriptors map capabilities to scopes |
| PKCE S256 | MUST for public clients | External Authorization Server responsibility |
| Refresh tokens | MUST for durable external MCP client connection | IdP issues and rotates appropriately |
| `offline_access` or provider equivalent | SHOULD/MUST at Authorization Server/OIDC layer | Do **not** advertise as MCP resource permission |
| OIDC discovery | SHOULD | Optional identity enrichment/claiming, not core OAuth dependency |
| Userinfo/email | OPTIONAL/SHOULD | Useful for domain/user identity claiming |
| Write/modify tools | SUPPORTED | Server auth remains authoritative; external MCP client confirmation is UX only |
| Frozen tool snapshot compatibility | MUST account for | Treat tool schema changes as versioned public API |
| Tool annotations/risk hints | MUST | Expose current MCP annotations accurately; never use them as auth |

### external MCP client registration precedence

Plan 029 must implement/document this precedence explicitly:

1. **CIMD — preferred** when the current external MCP client + MCP registration path supports it.
2. **User-defined OAuth client — MUST work** as the deterministic/manual fallback.
3. **DCR — compatibility only**, advertised only when the IdP exposes a real secure registration endpoint.

Never silently fall back from a failed/invalid registration mode to a weaker or unvalidated registration flow.

### external MCP client availability/mode matrix

Acceptance testing must record the actual behavior available to the tested external MCP client plan/mode rather than assuming every plan supports every action. At minimum document:

- external MCP client Business / Enterprise / Edu: full custom MCP write/modify flow where currently supported.
- external MCP client Pro: verify current read/fetch/write limitations against the live product before declaring compatibility.
- Deep Research: treat as read/fetch-only unless current OpenAI docs explicitly expand support.
- Agent mode: do not assume custom apps are available; verify against current product behavior.

The plan is complete only against the explicitly tested external MCP client plan/mode matrix.

---

# Tool authorization model

Define stable scopes before client integration. Recommended initial scope model:

- `relay.tools.read` — discovery/listing and non-side-effect metadata.
- `relay.search` — `web_search`.
- `relay.http.fetch` — `http_fetch`.
- `relay.terminal.execute` — `terminal_exec`.

`offline_access` is an Authorization Server/OIDC refresh-token concern, **not** an MCP resource permission and MUST NOT be mixed into `/.well-known/oauth-protected-resource` `scopes_supported` unless a future standard explicitly requires that behavior.

Rules:

- `terminal_exec` never inherits permission from a generic read scope.
- A token valid for one tool must not automatically authorize another tool.
- Default authorization is deny.
- Required scope is determined by server-owned tool metadata, never request arguments.
- Scope validation happens before execution or any side effect.
- external MCP client action confirmation does not grant scope and cannot override server denial.
- Tool annotations/risk hints do not grant, imply, or weaken authorization.

For single-developer/local-relay deployments, additionally bind `sub` (and optionally tenant/domain/client identity) to the configured owner identity.

---

# Phase 0 — Existing-Code & External-Contract Audit

### Objective

Freeze exactly what already exists and identify only real gaps against current external MCP client, MCP, and external MCP client behavior.

### Tasks

- [ ] Re-read `packages/rust-tools/src/relay_agent/transport.rs`, `mcp.rs`, `config.rs`, `security.rs`, and `execution.rs` from current `dev` before implementation.
- [ ] Record existing routes, protocol methods, OAuth behavior, scopes, subject checks, tool schemas, and sandbox entrypoints.
- [ ] Mark every existing capability in this plan as `EXISTING`, `PARTIAL`, or `MISSING` in `.agents/memories/029-external-mcp-mcp-integration-decisions.md`.
- [ ] Capture the actual external MCP client Custom Tool/App setup fields observed in the current UI.
- [ ] Verify current official OpenAI documentation for developer mode/custom MCP apps.
- [ ] Verify current official MCP transport + authorization + client-registration documents.
- [ ] Verify current external MCP client remote MCP requirements.
- [ ] Freeze a external MCP client/external MCP client compatibility matrix, including actual external MCP client plan/mode limitations.
- [ ] Do not implement anything in this phase.

### Exit criteria

- [ ] No Plan 028 functionality is scheduled for reimplementation without a documented gap.
- [ ] Every Phase 1+ task corresponds to a concrete missing/partial behavior.

---

# Phase 1 — Canonical Remote MCP Endpoint Hardening

### Objective

Reuse the existing `/mcp` implementation and fill only remote-client protocol gaps.

### Already implemented

- [x] `POST /mcp`.
- [x] Stateless MCP `2026-07-28` JSON-RPC core.
- [x] Protocol/method/name headers.
- [x] `server/discover`.
- [x] `tools/list` / `tools/call`.
- [x] body-size limit.

### Remaining tasks

- [ ] Verify external MCP client's current scanner against the live client/current MCP docs for required `Content-Type`, `Accept`, notification/status behavior, and transport semantics.
- [ ] Treat `POST /mcp` as MUST.
- [ ] Implement `GET /mcp` only when a **specific current MCP extension or tested client behavior requires it**; do not add GET/SSE just because older Streamable HTTP generations used it.
- [ ] If current POST responses require `text/event-stream` for a verified workflow, implement that behavior on the canonical `/mcp` path without reintroducing legacy session architecture.
- [ ] Do not add `/sse` + `/message?session_id=` unless a real supported client demonstrably requires legacy compatibility.
- [ ] If legacy SSE compatibility is required, route it into the exact same auth/tool-dispatch core and mark it compatibility-only/deprecated.
- [ ] Verify proxy/tunnel forwarding does not strip MCP metadata headers.
- [ ] Preserve local fail-closed Origin/Host semantics; remote mode must use its own explicit trusted-proxy/HTTPS policy rather than weakening local checks.

### Exit criteria

- [ ] external MCP client can scan the existing canonical MCP endpoint without introducing a second MCP protocol implementation.
- [ ] external MCP client can consume the same canonical endpoint.
- [ ] No unnecessary GET/SSE implementation exists without a documented current-client requirement.

---

# Phase 2 — external MCP client-Grade Tool Descriptors, Annotations & Schema Stability

### Objective

Make existing tools safe, understandable, scope-aware, risk-classified, and stable when external MCP client freezes their definitions.

### Existing baseline

- [x] Stable tool names exist.
- [x] JSON Schema input validation exists.
- [x] Tool dispatch exists.

### Tasks

- [ ] Audit `Tool` wire shape against current MCP + OpenAI Apps SDK/custom-app descriptor expectations.
- [ ] Add `title` where supported/valuable.
- [ ] Add output schemas where supported and where the result is structured enough to describe safely.
- [ ] Add current MCP tool annotations, including where applicable:
  - [ ] `readOnlyHint`,
  - [ ] `destructiveHint`,
  - [ ] `idempotentHint`,
  - [ ] `openWorldHint`.
- [ ] Treat annotations strictly as client UX/risk hints; add explicit code/docs stating they are **not authorization or sandbox controls**.
- [ ] Add external MCP client-compatible OAuth/security metadata or action-level scope tags only where the current OpenAI contract supports them.
- [ ] Ensure server-owned OAuth scope mapping remains authoritative even if a client ignores annotations or scope tags.
- [ ] Classify `terminal_exec` explicitly as write/destructive/high-risk/open-world and non-idempotent unless the final tool design narrows semantics.
- [ ] Do **not** incorrectly mark generic `http_fetch` read-only while it supports mutating methods. Either:
  - [ ] split read-only fetch from mutating HTTP actions, or
  - [ ] conservatively mark generic `http_fetch` as potentially mutating/open-world.
- [ ] Classify `web_search` as read-only/open-world and idempotent only if actual behavior supports that claim.
- [ ] Remove stale schema fields that execution ignores.
- [ ] Synchronize schema maxima with actual server-side limits (`timeout_ms`, args count, header count, etc.).
- [ ] Ensure descriptions explain user-visible behavior, risk, and scope needs rather than implementation internals.
- [ ] Freeze tool names once external MCP client app is published.
- [ ] Treat removing/renaming tools, required properties, scopes, or security semantics as breaking changes requiring external MCP client action refresh/review/republication.
- [ ] Prefer additive optional schema changes over breaking changes.
- [ ] Maintain a versioned tool-contract record in `.agents/memories/029-external-mcp-mcp-integration-decisions.md`.
- [ ] Ensure deterministic tool ordering and descriptor serialization.

### Exit criteria

- [ ] external MCP client Scan Tools presents clear, stable actions and accurate risk hints.
- [ ] Tool schemas match actual server behavior and scope requirements.
- [ ] Tool annotations cannot be mistaken for authorization controls.
- [ ] Future schema/security changes cannot silently break external MCP client's frozen snapshot without documented migration.

---

# Phase 3 — OAuth Discovery Bridge (External IdP, No Custom Auth Server)

### Objective

Make external MCP client's OAuth setup UI automatically discover real endpoints while the relay remains only an OAuth Resource Server.

### Existing baseline

- [x] `/.well-known/oauth-protected-resource` exists.
- [x] Resource-server JWKS validation exists.

### Tasks

- [ ] Select/freeze the production Authorization Server/IdP.
- [ ] Define one canonical resource identifier for the MCP server.
- [ ] Ensure `resource` in Protected Resource Metadata exactly matches the audience/resource policy enforced during token validation.
- [ ] Advertise real `authorization_servers` values.
- [ ] Ensure `/.well-known/oauth-protected-resource` contains resource-server scopes only; **do not include `offline_access`** as a resource permission.
- [ ] Populate/bridge current Authorization Server Metadata so external MCP client discovers:
  - [ ] authorization endpoint,
  - [ ] token endpoint,
  - [ ] issuer/authorization-server base,
  - [ ] supported authorization scopes,
  - [ ] PKCE S256 support,
  - [ ] refresh/offline capability at the AS/OIDC layer,
  - [ ] registration endpoint when DCR is supported.
- [ ] Do not return fabricated placeholder endpoints.
- [ ] Implement/document registration precedence:
  - [ ] CIMD — preferred when supported by the current external MCP client/MCP path.
  - [ ] User-defined OAuth client — MUST work.
  - [ ] DCR — compatibility-only; advertise only when provider safely supports it.
- [ ] Never silently downgrade from failed CIMD/DCR to an unvalidated registration path.
- [ ] Do not implement a Rust registration database merely to make DCR available; use IdP capability.
- [ ] Configure exact external MCP client callback URL copied from the live setup UI.
- [ ] Document that callback URLs are connector-instance-specific and must not be guessed/hardcoded globally.
- [ ] Verify token endpoint auth method expected by chosen external MCP client client mode (`none`, client secret, etc.).
- [ ] Verify changing advertised metadata produces the expected available registration options in the live external MCP client OAuth UI.

### Exit criteria

- [ ] external MCP client's Advanced OAuth settings populate from discovery with no fake values.
- [ ] At least User-defined OAuth Client registration works end-to-end.
- [ ] CIMD/DCR availability shown by external MCP client matches what the server/IdP actually advertises.
- [ ] No insecure registration fallback exists.

---

# Phase 4 — Refresh, PKCE, and Optional OIDC

### Objective

Make authentication durable and compatible with external MCP client's production connector lifecycle without confusing OAuth core with optional OIDC identity enrichment.

### Tasks

- [ ] Authorization Code flow handled by the external Authorization Server.
- [ ] Require PKCE S256 for public-client flows.
- [ ] Reject PKCE downgrade where controlled by the IdP/configuration.
- [ ] Ensure refresh tokens are issued where required for external MCP client persistent connectivity.
- [ ] Request/advertise `offline_access` or provider-equivalent only through Authorization Server/OIDC configuration, not as MCP resource permission.
- [ ] Configure refresh-token rotation/reuse policy according to IdP best practice.
- [ ] Verify expired access token can be renewed without user re-login.
- [ ] Verify revoked refresh token forces reauthorization.
- [ ] Do not pass refresh tokens to the Relay Agent tool layer.
- [ ] Keep OAuth Resource Server operation independent of OIDC; OIDC is optional identity enrichment, not required for bearer-token validation.
- [ ] If OIDC is enabled, advertise real `/.well-known/openid-configuration` from the IdP.
- [ ] If OIDC is enabled, verify userinfo endpoint and supported OIDC scopes.
- [ ] Treat userinfo/email/domain as supplemental identity data; never replace stable `iss` + `sub` authorization anchors with an unverified email string.
- [ ] Do not implement local password/login/session UI in the relay.

### Exit criteria

- [ ] external MCP client reconnects after normal access-token expiry.
- [ ] OAuth works even when OIDC enrichment is disabled.
- [ ] No token/secret enters tool arguments, command lines, workspace files, or logs.

---

# Phase 5 — Per-Tool Scope, Subject Authorization & OAuth Challenge Semantics

### Objective

Convert existing authentication into explicit least-privilege tool authorization with OAuth challenge responses that external MCP client/external MCP client can recover from correctly.

### Existing baseline

- [x] Auth context and parsed token claims exist.
- [x] JWT issuer/audience/signature validation foundation exists.

### Tasks

- [ ] Freeze scope names and server-owned `tool -> required scopes` mapping.
- [ ] Enforce `relay.terminal.execute` for `terminal_exec`.
- [ ] Enforce `relay.http.fetch` for `http_fetch`.
- [ ] Enforce `relay.search` for `web_search`.
- [ ] Keep discovery/listing policy explicit; do not accidentally grant execution through discovery scopes.
- [ ] Default deny missing scopes.
- [ ] Parse scope formats supported by the chosen IdP without accepting malformed/ambiguous values.
- [ ] Enforce subject ownership for single-user relay deployments.
- [ ] Optionally enforce tenant/domain/client ID where deployment requires it.
- [ ] Reject valid JWTs for the wrong developer/tenant.
- [ ] Authorization must run before schema-driven side effects and before tool dispatch.
- [ ] Return `401` for missing/invalid authentication and include standards-compatible `WWW-Authenticate` metadata.
- [ ] Where supported by current MCP/OAuth guidance, include `resource_metadata` in `WWW-Authenticate` so clients can discover the Protected Resource Metadata document.
- [ ] For invalid/expired tokens, expose `error="invalid_token"` without leaking token contents/internal validation details.
- [ ] For authenticated requests missing required scopes, return `403` and expose standards-compatible `error="insufficient_scope"` + required `scope` metadata where applicable.
- [ ] Ensure `WWW-Authenticate` responses never reveal secrets, internal paths, raw claims, or unnecessary issuer internals.
- [ ] Ensure local no-auth mode cannot accidentally enter remote mode through missing configuration.

### Exit criteria

- [ ] Valid token + right subject + right scope succeeds.
- [ ] Any wrong-subject/wrong-scope combination fails before execution.
- [ ] external MCP client/external MCP client can distinguish re-authentication from insufficient-scope failures using standards-compatible responses.

---

# Phase 6 — Remote Exposure / Secure MCP Tunnel / Proxy Boundary

### Objective

Expose a local developer relay to external MCP client without making the workstation a raw public command server.

### Tasks

- [ ] Verify the current supported OpenAI Secure MCP Tunnel workflow against live docs/product before implementation; do not hardcode obsolete tunnel assumptions.
- [ ] Keep relay loopback-only by default.
- [ ] For remote deployments, require explicit Remote mode and HTTPS.
- [ ] Define trusted proxy CIDRs/identity; never trust `X-Forwarded-*` from arbitrary peers.
- [ ] Define how effective scheme/host is derived behind the chosen proxy/tunnel.
- [ ] Preserve Authorization/MCP headers through the proxy.
- [ ] Add endpoint/request rate limiting.
- [ ] Add per-subject/per-token execution concurrency limits where useful.
- [ ] Add bounded request/stream lifetimes if streaming is actually enabled.
- [ ] Fail closed when proxy/tunnel security metadata is missing or contradictory.
- [ ] Ensure direct public no-auth mode cannot occur through config omission.

### Exit criteria

- [ ] external MCP client can reach the relay through the approved remote path.
- [ ] The public edge cannot bypass OAuth or Plan 028 execution security.

---

# Phase 7 — external MCP client Native Connector E2E

### Objective

Validate the actual current external MCP client Custom Tool/App UI rather than inferring compatibility from generic MCP tests.

### Tasks

- [ ] Create connector using the real MCP Server URL.
- [ ] Confirm `Scan Tools` succeeds.
- [ ] Record the external MCP client plan/mode used for testing and the capabilities actually available in that mode.
- [ ] Verify discovered tool names, descriptions, schemas, annotations/risk hints, and scopes.
- [ ] Open Advanced OAuth settings and verify discovered values:
  - [ ] Auth URL,
  - [ ] Token URL,
  - [ ] Registration URL when DCR applies,
  - [ ] Authorization Server base,
  - [ ] Resource,
  - [ ] OIDC fields where enabled.
- [ ] Verify available registration methods exactly match advertised capabilities.
- [ ] Verify User-defined OAuth Client flow.
- [ ] Verify CIMD if available/supported.
- [ ] Verify DCR only if advertised/supported.
- [ ] Verify exact callback URI.
- [ ] Verify PKCE S256.
- [ ] Verify base/default/action scopes requested by external MCP client match the intended tool selections.
- [ ] Verify access-token refresh without manual reconnect.
- [ ] Verify OIDC userinfo/email behavior if enabled.
- [ ] Execute `web_search` with search scope only.
- [ ] Execute `http_fetch` with fetch scope only.
- [ ] Verify those tokens cannot call `terminal_exec`.
- [ ] Execute a real coding workflow with terminal scope:
  - [ ] inspect repository,
  - [ ] create/edit file,
  - [ ] git operation,
  - [ ] package/build command,
  - [ ] shell chaining,
  - [ ] verify output.
- [ ] Verify external MCP client confirmation/write UX where the current product provides it.
- [ ] Verify server denies execution even if client UX would otherwise allow it when scope/subject is invalid.
- [ ] Verify wrong subject, wrong resource, wrong issuer, expired token, revoked token, wrong scope, malformed request all fail closed.
- [ ] Verify existing Plan 028 sandbox escape cases remain blocked remotely.

### Exit criteria

- [ ] Real external MCP client connection can perform the intended coding workflow.
- [ ] OAuth discovery/registration UI is accurate.
- [ ] Client-side approval UX and server-side authorization remain clearly separate.

---

# Phase 8 — external MCP client Compatibility

### Objective

Use the same protocol/auth/tool core for external MCP client without creating a second implementation.

### Tasks

- [ ] Connect current external MCP client Code / supported external MCP client MCP client to canonical `/mcp`.
- [ ] Verify current HTTP transport compatibility.
- [ ] Add SSE compatibility only if the actual tested client requires it.
- [ ] Reuse the same OAuth Resource Server validation.
- [ ] Reuse the same tool descriptors and scopes where the client supports them.
- [ ] Verify external MCP client cannot bypass scope/subject authorization.
- [ ] Verify realistic coding workflow.
- [ ] Record genuine client-specific differences in the decision memory; do not fork execution core.

### Exit criteria

- [ ] external MCP client and external MCP client use one MCP implementation and one security model.

---

# Phase 9 — Tool Contract Lifecycle & Published-App Change Management

### Objective

Treat the tool catalog as a public API because external MCP client can freeze/snapshot discovered actions and require explicit refresh/review for changes.

### Tasks

- [ ] Record a canonical version/hash of the published tool catalog.
- [ ] Treat tool names as stable API identifiers after publication.
- [ ] Require migration notes for tool removals/renames or required-field changes.
- [ ] Prefer additive optional properties over breaking input-schema changes.
- [ ] Treat changes to risk annotations, required scopes, destructive behavior, or write semantics as security-relevant changes requiring connector refresh/review.
- [ ] Document the operator process for external MCP client `Refresh` / action review after server changes.
- [ ] Verify newly added actions are not assumed enabled until current external MCP client admin/user workflow confirms them.
- [ ] Ensure old tool snapshots fail safely if they call removed/deprecated behavior.
- [ ] Store final published descriptor snapshot/evidence in `.agents/memories/029-external-mcp-mcp-integration-decisions.md`.

### Exit criteria

- [ ] Tool evolution cannot silently alter privileges or break a published external MCP client integration.

---

# Phase 10 — Observability & Remote Abuse Controls

### Objective

Make a remotely reachable command-capable MCP server diagnosable without leaking user code, credentials, or command secrets.

### Tasks

- [ ] Structured request/correlation ID.
- [ ] Log method/tool/outcome/latency/subject identifier safely.
- [ ] Never log Authorization headers or access/refresh tokens.
- [ ] Do not log full terminal commands/tool arguments by default; use redacted audit metadata.
- [ ] Add auth-failure, scope-denial, rate-limit, tool-error, timeout, and sandbox-failure metrics.
- [ ] Bound log sizes/cardinality.
- [ ] Ensure attacker-controlled strings cannot produce log injection or unbounded labels.
- [ ] Define retention/redaction policy.

### Exit criteria

- [ ] OAuth/MCP failures can be debugged without exposing secrets/source content.

---

# Phase 11 — Zero-Bypass CI / Conformance / Release Gate

### Objective

Prevent a connector-compatible artifact from shipping if MCP/OAuth/tool/security contracts regress.

### Tasks

- [ ] Preserve Plan 028 `cargo fmt --all -- --check`.
- [ ] Preserve workspace/all-target/all-feature/locked `cargo check`.
- [ ] Preserve workspace/all-target/all-feature/locked Clippy `-D warnings`.
- [ ] Preserve `cargo audit`.
- [ ] No warning/lint/security suppression or CI failure masking.
- [ ] Add deterministic checks for required remote routes/metadata.
- [ ] Add protocol conformance checks for canonical `/mcp`.
- [ ] Add metadata validation for Protected Resource/Authorization Server documents.
- [ ] Add checks for missing tool scope metadata / server-owned scope mapping.
- [ ] Add checks that `offline_access` is not accidentally advertised as an MCP resource permission.
- [ ] Add checks that annotations are present/valid but never consumed as authorization decisions.
- [ ] Add checks for `WWW-Authenticate` invalid-token / insufficient-scope behavior.
- [ ] Add checks against unauthenticated Remote mode.
- [ ] Add checks that no new external MCP client/external MCP client compatibility path bypasses the canonical auth/tool/sandbox core.
- [ ] Add tool-contract snapshot/change detection requiring explicit review for breaking/security-relevant changes.
- [ ] Release must depend on the complete quality/conformance/security gate.

### Exit criteria

- [ ] CI cannot go green by bypassing warnings, auth, scope, descriptor, or protocol checks.
- [ ] Release cannot publish when connector contract/security checks fail.

---

# Phase 12 — Final Production Readiness

Plan 029 can be marked `COMPLETED` only when:

- [ ] Current code was re-audited and no Plan 028 subsystem was unnecessarily duplicated.
- [ ] Current MCP canonical transport works with external MCP client.
- [ ] GET/SSE behavior exists only where justified by tested current-client requirements.
- [ ] external MCP client tool scan succeeds.
- [ ] Tool descriptors include accurate schemas, risk annotations, and scope metadata.
- [ ] Annotations are demonstrably advisory only and cannot grant authorization.
- [ ] OAuth Advanced settings show correct discovered endpoints/resource.
- [ ] CIMD/User-defined/DCR availability matches actual advertised support and follows the documented precedence.
- [ ] User-defined OAuth Client works.
- [ ] CIMD works if declared supported.
- [ ] DCR works if declared supported.
- [ ] PKCE S256 works.
- [ ] refresh-token flow works.
- [ ] `offline_access` remains confined to AS/OIDC semantics, not MCP resource permissions.
- [ ] OIDC works if enabled but OAuth still functions without it.
- [ ] exact callback configuration works.
- [ ] resource/audience/issuer/subject checks are strict.
- [ ] per-tool scopes are enforced server-side.
- [ ] `WWW-Authenticate` invalid-token / insufficient-scope challenges are standards-compatible.
- [ ] external MCP client read/write/destructive UX matches the actual tested plan/mode and is not treated as security enforcement.
- [ ] Plan 028 sandbox is unchanged as authoritative execution boundary.
- [ ] secure remote exposure works.
- [ ] external MCP client uses same core successfully.
- [ ] published tool snapshot/change-management workflow is documented and verified.
- [ ] no secrets leak through logs/tool results.
- [ ] CI zero-bypass gates pass.
- [ ] release gate passes.
- [ ] real external MCP client coding E2E passes.
- [ ] negative auth/scope/sandbox tests fail closed.
- [ ] `.agents/memories/029-external-mcp-mcp-integration-decisions.md` contains final observed client behavior, tool-contract snapshot, OAuth registration mode, IdP decisions, and any compatibility exceptions.

---

## Definition of Done

The same Rust Relay Agent is consumable by external MCP client and external MCP client as a standards-compliant remote MCP server. Plan 029 adds only the missing connector/OAuth/descriptor/remote-operation pieces on top of Plan 028, follows current MCP and external MCP client registration/discovery semantics, treats tool annotations and external MCP client confirmation as advisory UX only, keeps `offline_access` at the Authorization Server/OIDC layer, preserves strict subject/scope/sandbox enforcement, and ships only after real client E2E plus zero-bypass CI/release verification succeed.
