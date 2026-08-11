# Plan 029 — ChatGPT / Claude Native MCP Connector Integration

**Status: IN FLIGHT**

## Goal

Turn the existing Plan 028 Rust Relay Agent into a production-grade **remote MCP app/connector endpoint** that works natively with ChatGPT and Claude without duplicating the MCP core, OAuth resource-server logic, or execution sandbox already implemented.

Plan 029 is explicitly a **delta/integration plan**, not a second MCP implementation. Existing Plan 028 behavior remains authoritative unless this plan identifies a concrete compatibility gap.

The primary target is the current MCP `2026-07-28` protocol and ChatGPT's current custom MCP app flow. ChatGPT/Claude compatibility is validated at the client boundary; client-specific code must not fork tool execution, authorization, or sandbox logic.

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
- Do not create a ChatGPT-only REST API or legacy ChatGPT Plugin API.
- Do not create a second Claude-specific MCP server.
- Do not replace or weaken the Plan 028 filesystem/process/container sandbox.
- Do not duplicate existing JWKS/token validation just to satisfy ChatGPT setup UI.
- Do not make ChatGPT confirmation prompts a security boundary.
- Do not expose local no-auth mode to the public internet.
- Do not reintroduce deprecated HTTP+SSE as the canonical protocol.
- Do not require new unit tests for this deadline; use strict compile/lint/audit, protocol checks, targeted integration/E2E, and manual security review.

---

# Architecture

```text
ChatGPT / Claude
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

A ChatGPT/Claude connection may change **transport/authentication context**, but it must never create a weaker execution path.

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

---

# ChatGPT compatibility contract

ChatGPT's current custom MCP app UI exposes these integration concepts and Plan 029 must explicitly support or intentionally document each one.

| Capability | Requirement | Plan 029 treatment |
|---|---|---|
| Remote MCP Server URL | MUST | Canonical HTTPS `/mcp` URL |
| Tool discovery/scan | MUST | Reuse `tools/list`, harden descriptors |
| Tool invocation | MUST | Reuse `tools/call` |
| User-defined OAuth client | MUST | Supported/config documented |
| Dynamic Client Registration (DCR) | SHOULD | Support through external IdP when advertised; compatibility path |
| Client Identifier Metadata Document (CIMD) | SHOULD | Prefer when supported by current MCP/client registration flow |
| Authorization endpoint | MUST for OAuth | Discover from Authorization Server metadata |
| Token endpoint | MUST for OAuth | Discover from Authorization Server metadata |
| Registration endpoint | CONDITIONAL | Required for DCR only |
| Authorization Server base/issuer | MUST | Stable trusted issuer |
| Resource identifier | MUST | Stable canonical MCP resource |
| Base scopes | MUST define | Small minimum scopes requested on every auth |
| Default scopes | MUST define | Fallback when action-level scopes are incomplete |
| Action/tool scopes | MUST | Tool descriptors map capabilities to scopes |
| PKCE S256 | MUST for public clients | External Authorization Server responsibility |
| Refresh tokens | MUST for durable ChatGPT connection | IdP issues and rotates appropriately |
| `offline_access` or provider equivalent | SHOULD/MUST where IdP requires | Advertised/configured |
| OIDC discovery | SHOULD | Enable when provider supports it |
| Userinfo/email | OPTIONAL/SHOULD | Useful for domain/user identity claiming |
| Write/modify tools | SUPPORTED | Server auth remains authoritative; ChatGPT confirmation is UX only |
| Frozen tool snapshot compatibility | MUST account for | Treat tool schema changes as versioned public API |

---

# Tool authorization model

Define stable scopes before client integration. Recommended initial scope model:

- `relay.tools.read` — discovery/listing and non-side-effect metadata.
- `relay.search` — `web_search`.
- `relay.http.fetch` — `http_fetch`.
- `relay.terminal.execute` — `terminal_exec`.
- `offline_access` — provider-standard refresh-token request where applicable.

Rules:

- `terminal_exec` never inherits permission from a generic read scope.
- A token valid for one tool must not automatically authorize another tool.
- Default authorization is deny.
- Required scope is determined by server-owned tool metadata, never request arguments.
- Scope validation happens before execution or any side effect.
- ChatGPT action confirmation does not grant scope and cannot override server denial.

For single-developer/local-relay deployments, additionally bind `sub` (and optionally tenant/domain/client identity) to the configured owner identity.

---

# Phase 0 — Existing-Code & External-Contract Audit

### Objective

Freeze exactly what already exists and identify only real gaps against current ChatGPT, MCP, and Claude behavior.

### Tasks

- [ ] Re-read `packages/rust-tools/src/relay_agent/transport.rs`, `mcp.rs`, `config.rs`, `security.rs`, and `execution.rs` from current `dev` before implementation.
- [ ] Record existing routes, protocol methods, OAuth behavior, scopes, subject checks, tool schemas, and sandbox entrypoints.
- [ ] Mark every existing capability in this plan as `EXISTING`, `PARTIAL`, or `MISSING` in `.agents/memories/029-chatgpt-mcp-integration-decisions.md`.
- [ ] Capture the actual ChatGPT Custom Tool/App setup fields observed in the current UI.
- [ ] Verify current official OpenAI documentation for developer mode/custom MCP apps.
- [ ] Verify current official MCP transport + authorization + client-registration documents.
- [ ] Verify current Claude remote MCP requirements.
- [ ] Freeze a ChatGPT/Claude compatibility matrix.
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

- [ ] Verify whether ChatGPT's current scanner requires `GET /mcp`, POST SSE responses, or a specific `Accept` behavior; implement only requirements confirmed against the live client/current MCP docs.
- [ ] If GET SSE is needed by current Streamable HTTP semantics, implement it on `/mcp`, not a separate legacy core.
- [ ] Do not add `/sse` + `/message?session_id=` unless a real supported client demonstrably requires legacy compatibility.
- [ ] If legacy SSE compatibility is required, route it into the exact same auth/tool-dispatch core.
- [ ] Verify `Content-Type` and `Accept` negotiation against ChatGPT's real tool scanner.
- [ ] Verify notification behavior/status codes against the current MCP client.
- [ ] Verify proxy/tunnel forwarding does not strip MCP metadata headers.
- [ ] Preserve local fail-closed Origin/Host semantics; remote mode must use its own explicit trusted-proxy/HTTPS policy rather than weakening local checks.

### Exit criteria

- [ ] ChatGPT can scan the existing canonical MCP endpoint without introducing a second MCP protocol implementation.
- [ ] Claude can consume the same canonical endpoint.

---

# Phase 2 — ChatGPT-Grade Tool Descriptors & Schema Stability

### Objective

Make existing tools safe, understandable, scope-aware, and stable when ChatGPT freezes their definitions.

### Existing baseline

- [x] Stable tool names exist.
- [x] JSON Schema input validation exists.
- [x] Tool dispatch exists.

### Tasks

- [ ] Audit `Tool` wire shape against current MCP + OpenAI Apps SDK/custom-app descriptor expectations.
- [ ] Add `title` where supported/valuable.
- [ ] Add output schemas where supported and where the result is structured enough to describe safely.
- [ ] Add MCP annotations/tool metadata needed to distinguish read-only, write, destructive, idempotent, and open-world behavior where supported.
- [ ] Add ChatGPT-compatible OAuth/security metadata or scope tags at the tool/action descriptor layer where the current OpenAI contract supports them.
- [ ] Ensure `terminal_exec` is explicitly classified as write/destructive/high-risk capability.
- [ ] Classify `http_fetch` accurately; GET-like behavior is read/network access while arbitrary methods may mutate remote state.
- [ ] Classify `web_search` as read-only/network access.
- [ ] Remove stale schema fields that execution ignores.
- [ ] Synchronize schema maxima with actual server-side limits (`timeout_ms`, args count, header count, etc.).
- [ ] Ensure descriptions explain user-visible behavior, not internal implementation details like binary filenames unless useful.
- [ ] Freeze tool names once ChatGPT app is published.
- [ ] Treat removing/renaming required fields or tools as breaking changes requiring ChatGPT action refresh/republication.
- [ ] Document backward-compatible tool evolution rules: additive optional fields are preferred.
- [ ] Ensure deterministic tool ordering and descriptor serialization.

### Exit criteria

- [ ] ChatGPT Scan Tools presents clear, stable actions.
- [ ] Tool schemas match actual server behavior and scope requirements.
- [ ] Future schema changes cannot silently break ChatGPT's frozen snapshot without documented migration.

---

# Phase 3 — OAuth Discovery Bridge (External IdP, No Custom Auth Server)

### Objective

Make ChatGPT's OAuth setup UI automatically discover real endpoints while the relay remains only an OAuth Resource Server.

### Existing baseline

- [x] `/.well-known/oauth-protected-resource` exists.
- [x] Resource-server JWKS validation exists.

### Tasks

- [ ] Select/freeze the production Authorization Server/IdP.
- [ ] Define one canonical resource identifier for the MCP server.
- [ ] Ensure `resource` in Protected Resource Metadata exactly matches the audience/resource policy enforced during token validation.
- [ ] Advertise real `authorization_servers` values.
- [ ] Populate/bridge current Authorization Server Metadata so ChatGPT discovers:
  - [ ] authorization endpoint,
  - [ ] token endpoint,
  - [ ] issuer/authorization-server base,
  - [ ] supported scopes,
  - [ ] PKCE S256 support,
  - [ ] refresh/offline capability,
  - [ ] registration endpoint when DCR is supported.
- [ ] Do not return fabricated placeholder endpoints.
- [ ] Decide registration methods explicitly:
  - [ ] User-defined OAuth client — MUST work.
  - [ ] DCR — support only when provider advertises and is safely configured.
  - [ ] CIMD — advertise/support when current MCP + ChatGPT integration path supports it.
- [ ] Do not implement a Rust registration database merely to make DCR available; use IdP capability.
- [ ] Configure exact ChatGPT callback URL copied from the live setup UI.
- [ ] Document that callback URLs are connector-instance-specific and must not be guessed/hardcoded globally.
- [ ] Verify token endpoint auth method expected by chosen ChatGPT client mode (`none`, client secret, etc.).

### Exit criteria

- [ ] ChatGPT's Advanced OAuth settings populate from discovery with no fake values.
- [ ] At least User-defined OAuth Client registration works end-to-end.
- [ ] DCR/CIMD availability shown by ChatGPT matches what the server/IdP actually advertises.

---

# Phase 4 — Refresh, PKCE, and Optional OIDC

### Objective

Make authentication durable and compatible with ChatGPT's production connector lifecycle.

### Tasks

- [ ] Authorization Code flow handled by the external Authorization Server.
- [ ] Require PKCE S256 for public-client flows.
- [ ] Reject PKCE downgrade where controlled by the IdP/configuration.
- [ ] Ensure refresh tokens are issued where required for ChatGPT persistent connectivity.
- [ ] Request/advertise `offline_access` or provider-equivalent scope when required.
- [ ] Configure refresh-token rotation/reuse policy according to IdP best practice.
- [ ] Verify expired access token can be renewed without user re-login.
- [ ] Verify revoked refresh token forces reauthorization.
- [ ] Do not pass refresh tokens to the Relay Agent tool layer.
- [ ] If OIDC is enabled, advertise real `/.well-known/openid-configuration` from the IdP.
- [ ] If OIDC is enabled, verify userinfo endpoint and supported OIDC scopes.
- [ ] Use OIDC email/domain only as an additional identity claim; keep `sub`/issuer as stable identity anchors.
- [ ] Do not implement local password/login/session UI in the relay.

### Exit criteria

- [ ] ChatGPT reconnects after normal access-token expiry.
- [ ] No token/secret enters tool arguments, command lines, workspace files, or logs.

---

# Phase 5 — Per-Tool Scope & Subject Authorization

### Objective

Convert existing authentication into explicit least-privilege tool authorization.

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
- [ ] Use correct `401` for missing/invalid authentication and `403` for authenticated-but-insufficient authorization.
- [ ] `WWW-Authenticate` should expose standards-compatible error/scope information without leaking internals.
- [ ] Ensure local no-auth mode cannot accidentally enter remote mode through missing configuration.

### Exit criteria

- [ ] Valid token + right subject + right scope succeeds.
- [ ] Any wrong-subject/wrong-scope combination fails before execution.

---

# Phase 6 — Remote Exposure / Secure MCP Tunnel / Proxy Boundary

### Objective

Expose a local developer relay to ChatGPT without making the workstation a raw public command server.

### Tasks

- [ ] Use OpenAI Secure MCP Tunnel for supported local/private-network ChatGPT development when available/applicable.
- [ ] Keep relay loopback-only by default.
- [ ] Remote server mode requires explicit configuration.
- [ ] Remote mode requires HTTPS at the trusted ingress.
- [ ] Define exact trusted-proxy IP/network configuration.
- [ ] Never trust arbitrary `X-Forwarded-*` headers from direct clients.
- [ ] Ensure proxy preserves Authorization and required MCP headers.
- [ ] Define canonical public MCP URL and resource identifier relationship.
- [ ] Add/verify request rate limiting.
- [ ] Add per-subject/token tool-call rate limiting for dangerous actions.
- [ ] Keep execution concurrency limits.
- [ ] Bound stream/connection lifetime if GET/SSE is implemented.
- [ ] Prevent slow-client and connection-exhaustion DoS.
- [ ] Keep internal paths/errors redacted.

### Exit criteria

- [ ] ChatGPT can reach a developer-machine relay through the approved tunnel path.
- [ ] Closing the tunnel removes remote reachability.
- [ ] No unauthenticated remote fallback exists.

---

# Phase 7 — Real ChatGPT Custom MCP App E2E

### Objective

Validate the exact current ChatGPT UI and runtime, not only protocol-level mocks.

### Setup verification

- [ ] Create Custom Tool/App using the actual MCP server URL.
- [ ] Confirm OAuth discovery populates real endpoints.
- [ ] Verify User-defined OAuth Client flow.
- [ ] Verify DCR if advertised/supported.
- [ ] Verify CIMD if advertised/supported.
- [ ] Copy/register the exact ChatGPT callback URL.
- [ ] Verify Default scopes.
- [ ] Verify Base scopes.
- [ ] Verify action-level/tool-level scopes.
- [ ] Verify OIDC fields if enabled.

### Tool scan

- [ ] `Scan Tools` succeeds.
- [ ] Only intended tools appear.
- [ ] Names, descriptions, schemas, and write/read semantics are correct.
- [ ] Tool scope tags/security metadata appear as expected.

### Coding workflow

- [ ] Inspect repository/workspace.
- [ ] Read files.
- [ ] Create/edit files.
- [ ] Move/delete files inside the permitted workspace.
- [ ] Run shell/interpreter commands.
- [ ] Run package-manager/build commands.
- [ ] Run a realistic compile/build flow.
- [ ] Use Docker only within Plan 028 policy.
- [ ] Verify terminal output/errors are returned correctly.

### Negative security scenarios

- [ ] Missing token.
- [ ] Invalid signature.
- [ ] Wrong issuer.
- [ ] Wrong resource/audience.
- [ ] Wrong subject.
- [ ] Missing tool scope.
- [ ] Expired access token.
- [ ] Revoked/invalid refresh token.
- [ ] Unknown/rotated signing key.
- [ ] malformed MCP request.
- [ ] unauthorized write tool.
- [ ] Plan 028 filesystem escape attempt.
- [ ] privilege escalation attempt.
- [ ] Docker escape attempt.
- [ ] rate/concurrency abuse.

### Frozen snapshot behavior

- [ ] Publish/test a tool snapshot.
- [ ] Verify additive optional schema change behavior.
- [ ] Document that breaking action/tool changes require ChatGPT action refresh/republication.

### Exit criteria

- [ ] A real ChatGPT conversation can inspect, modify, build, and run code through the relay.
- [ ] No server-side security rule depends on ChatGPT confirmation UX.

---

# Phase 8 — Claude Compatibility E2E

### Objective

Prove vendor-neutral MCP behavior using the same implementation.

### Tasks

- [ ] Connect Claude remote MCP client to the canonical `/mcp` endpoint.
- [ ] Verify discovery/tools/call.
- [ ] Verify OAuth using the same Authorization Server/resource policy.
- [ ] Verify scope enforcement is identical.
- [ ] Verify real coding workflow.
- [ ] Add legacy SSE adapter only if an actual supported Claude target cannot use the canonical transport.
- [ ] If legacy adapter is required, no separate auth/tool/sandbox implementation is allowed.

### Exit criteria

- [ ] ChatGPT and Claude share exactly one MCP/tool/security core.

---

# Phase 9 — Observability & Audit

### Objective

Make remote tool use diagnosable without creating a new data-leak surface.

### Tasks

- [ ] Correlation/request IDs.
- [ ] Record client type when reliably known without trusting spoofable data for authorization.
- [ ] Record method/tool/outcome/latency/status/subject identifier.
- [ ] Never log bearer/refresh tokens.
- [ ] Redact command arguments likely to contain secrets.
- [ ] Avoid dumping source-file contents into logs.
- [ ] Metrics for auth failures, scope failures, tool calls, timeouts, rate limits, sandbox failures, JWKS refresh/failure.
- [ ] Bound metrics label cardinality.
- [ ] Audit events must not be writable to attacker-controlled workspace paths.

---

# Phase 10 — Zero-Bypass CI / Conformance / Release Gate

### Objective

Prevent ChatGPT compatibility work from weakening Plan 028 quality/security guarantees.

### Required gates

- [ ] `cargo fmt --all -- --check`.
- [ ] `cargo check --workspace --all-targets --all-features --locked` with warnings denied by repository policy.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- [ ] `cargo audit`.
- [ ] No blanket `allow`/`expect`/warning suppression to hide failures.
- [ ] No `continue-on-error`, `|| true`, swallowed failures, or equivalent lint/security bypasses.
- [ ] Protocol conformance check for current `/mcp` contract.
- [ ] OAuth metadata/discovery conformance check.
- [ ] Static check that every tool-call side-effect path passes authorization.
- [ ] Static check that remote mode cannot fall back to local no-auth access.
- [ ] Static check that no second MCP execution core was introduced.
- [ ] Static check that Node/pkg relay runtime is not reintroduced.
- [ ] Release job must depend on the complete quality/security gate.
- [ ] Release artifact built from exact reviewed commit.

---

# Phase 11 — Final Production Readiness

Plan 029 may be marked `COMPLETED` only when all are true:

- [ ] Current ChatGPT Custom MCP App connects successfully.
- [ ] ChatGPT Scan Tools succeeds.
- [ ] User-defined OAuth client works.
- [ ] DCR/CIMD behavior exactly matches advertised support.
- [ ] Authorization/token/resource metadata is correctly discovered.
- [ ] Refresh-token flow survives access-token expiry.
- [ ] Optional OIDC flow works if enabled.
- [ ] Per-tool scope enforcement is default-deny.
- [ ] Wrong subject cannot execute tools.
- [ ] Claude uses the same canonical server successfully.
- [ ] Plan 028 sandbox remains the only execution security boundary.
- [ ] Remote/tunnel path cannot bypass auth/sandbox.
- [ ] Tool schemas match actual implementation limits and behavior.
- [ ] Breaking tool-schema changes have a documented ChatGPT refresh/republication process.
- [ ] Observability does not leak tokens/source/secrets.
- [ ] CI is zero-warning and zero-bypass.
- [ ] Release gate is green.
- [ ] Real coding E2E passes.
- [ ] Security-negative E2E fails closed.
- [ ] `.agents/memories/029-chatgpt-mcp-integration-decisions.md` records actual client behavior and final decisions.

---

## Source-of-truth references

Review these again at implementation time because ChatGPT/MCP integration behavior is actively evolving:

- OpenAI — Developer mode and MCP apps in ChatGPT: https://help.openai.com/en/articles/12584461-developer-mode-and-full-mcp-connectors-in-chatgpt
- OpenAI — Apps in ChatGPT: https://help.openai.com/en/articles/11487775-connectors-in-chatgpt
- OpenAI — Build with the Apps SDK: https://help.openai.com/en/articles/12515353-build-with-the-apps-sdk
- Model Context Protocol specification/release documentation for the current `2026-07-28` protocol and authorization/client-registration model.
- Anthropic current Claude Code / remote MCP documentation.

## Definition of Done

Plan 029 is complete when the **existing** Rust Relay Agent from Plan 028 can be consumed by ChatGPT and Claude as a remote MCP app using one canonical MCP/tool/security implementation, with standards-compliant OAuth discovery and client registration, durable refresh-token behavior, least-privilege per-tool scopes, owner/subject binding, secure remote exposure, strict CI/release gates, and a real coding E2E — without duplicating the MCP core or weakening the Plan 028 sandbox.