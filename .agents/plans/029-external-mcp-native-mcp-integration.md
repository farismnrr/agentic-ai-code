# Plan 029 — Production external MCP client Native MCP App Integration

**Status: IN FLIGHT**

## Goal

Turn the existing Plan 028 Rust Relay Agent into a production-grade **remote MCP app endpoint for external MCP client** without rebuilding the MCP core, OAuth Resource Server, or execution sandbox that already exists.

Plan 029 is a **delta/integration plan**. Plan 028 remains authoritative for process execution, filesystem containment, privilege boundaries, Docker restrictions, resource limits, and the existing Rust MCP core.

The primary target is:

- current MCP `2026-07-28`,
- external MCP client custom MCP apps in developer mode,
- full read/write/modify tool usage on the external MCP client plans where OpenAI currently supports full MCP,
- external OAuth/OIDC provider integration,
- real external MCP client tool scanning + OAuth + coding E2E,
- production-grade security and CI/release gates **without adding a unit-test requirement**.

---

# Scope decisions

## 1. No unit-test gate for Plan 029

This is an explicit delivery decision for speed.

Plan 029 MUST NOT add a requirement to create or restore Rust/TypeScript unit-test suites.

Completion MUST NOT depend on `cargo test`, JS unit tests, mocks, fixtures, or test-only production hooks.

Instead, verification is intentionally concentrated on the boundaries that matter for this integration:

- strict compile/static/lint checks,
- dependency/security audit,
- deterministic protocol/metadata validation scripts,
- real OAuth discovery/authorization verification,
- real external MCP client `Scan Tools`,
- real external MCP client tool calls,
- negative authentication/authorization checks,
- real Plan 028 sandbox escape checks,
- real coding E2E,
- manual final security review.

No production bypass may be introduced merely to make E2E easier.

## 2. external MCP client is the Plan 029 release target

Plan 029 is complete when the existing Rust Relay Agent works as a secure custom MCP app in external MCP client.

external MCP client or other MCP clients MAY be used as secondary interoperability checks, but **external MCP client compatibility is not a Plan 029 release gate**. Do not expand this plan into a second vendor integration project.

The protocol and tool contracts must remain vendor-neutral MCP so future clients can reuse them without a fork.

## 3. Custom external MCP client MCP app first; Plugin Directory/public distribution later

Current OpenAI product terminology distinguishes custom MCP apps from Plugin Directory distribution.

Plan 029 targets a **custom MCP app** created/tested in external MCP client developer mode and published inside the intended workspace.

Out of scope for Plan 029:

- public Plugin Directory listing,
- OpenAI registry submission/review workflow,
- marketplace/discovery copy,
- public multi-tenant SaaS onboarding,
- interactive Apps SDK UI/components,
- app monetization/billing,
- organization-wide external customer onboarding.

If public Plugin Directory distribution is wanted later, create a separate plan after the MCP backend is proven.

## 4. Headless MCP tools only

The Relay Agent is a coding/execution backend, not an interactive external MCP client UI app.

Do not add Apps SDK UI resources/components unless a concrete product requirement appears later.

Use standard MCP tool descriptors and external MCP client custom-app behavior only.

## 5. Existing Plan 028 security boundary is immutable

external MCP client confirmation prompts, MCP tool annotations, OAuth metadata, and connector UI settings are never the execution security boundary.

Every side effect must still follow:

```text
request
  -> MCP protocol validation
  -> authentication
  -> subject authorization
  -> scope/tool authorization
  -> argument/schema validation
  -> Plan 028 execution/sandbox policy
  -> side effect
```

---

# Current implementation baseline — DO NOT REIMPLEMENT

The following exists on `dev` and must be reused:

- [x] Rust `relay-agent` binary.
- [x] Transport-independent Rust MCP core.
- [x] Canonical `POST /mcp` JSON-RPC endpoint.
- [x] MCP `2026-07-28` protocol constant.
- [x] Stateless MCP request model.
- [x] No legacy `Mcp-Session-Id` authorization boundary.
- [x] `MCP-Protocol-Version` validation.
- [x] `Mcp-Method` validation.
- [x] `Mcp-Name` validation.
- [x] request `_meta` parsing/validation.
- [x] `server/discover`.
- [x] `tools/list`.
- [x] `tools/call`.
- [x] JSON Schema 2020-12-compatible input validation before tool execution.
- [x] `terminal_exec`.
- [x] `http_fetch`.
- [x] `web_search`.
- [x] local-mode fail-closed Origin/Host policy.
- [x] explicit Local vs Remote security mode.
- [x] OAuth Resource Server boundary in Remote mode.
- [x] Bearer token parsing.
- [x] asymmetric JWKS validation.
- [x] JWKS TTL cache.
- [x] refresh-on-unknown-`kid`.
- [x] bounded JWKS fetch timeout.
- [x] issuer/audience validation foundation.
- [x] Plan 028 filesystem/process/privilege/container sandbox.
- [x] Plan 028 output/timeout/resource controls.
- [x] strict Rust CI/release quality foundation.

If a baseline item is found incomplete during implementation, reopen only that concrete gap. Do not create parallel implementations.

---

# Current external baseline

Before implementation, re-check these official sources because external MCP client custom-app behavior is actively evolving:

- OpenAI — Developer mode and MCP apps in external MCP client:
  `https://help.openai.com/en/articles/12584461-developer-mode-and-full-mcp-connectors-in-external-mcp`
- OpenAI — Apps in external MCP client:
  `https://help.openai.com/en/articles/11487775-connectors-in-external-mcp`
- OpenAI — Build with the Apps SDK:
  `https://help.openai.com/en/articles/12515353-build-with-the-apps-sdk`
- MCP — `2026-07-28` release:
  `https://blog.modelcontextprotocol.io/posts/2026-07-28/`
- MCP — Authorization:
  `https://modelcontextprotocol.io/specification/draft/basic/authorization`
- MCP — Tool schema/annotations:
  `https://modelcontextprotocol.io/specification/2025-11-25/schema`

Implementation must prefer current official OpenAI + MCP behavior over stale memories or older SSE tutorials.

---

# external MCP client compatibility contract

## Product availability baseline

As currently documented by OpenAI:

- external MCP client Business / Enterprise / Edu: target for full MCP including write/modify actions.
- external MCP client Pro: custom MCP connectivity is limited compared with full MCP; read/fetch is currently the relevant compatibility surface.
- Deep Research: custom apps are read/fetch only.
- Agent mode: custom apps are currently unavailable.
- external MCP client connects to remote MCP servers; a local/private relay needs the supported Secure MCP Tunnel or an explicitly secured remote deployment.

Plan 029's **release E2E must use a external MCP client plan/mode that supports the intended write/modify coding workflow**.

Do not declare full write compatibility based only on a Pro/read-only flow.

## MCP endpoint

- `POST /mcp` is the canonical MCP endpoint.
- Do not rebuild the deprecated `/sse` + `/message?session_id=...` architecture.
- Do not add `GET /mcp` or long-lived SSE unless a current MCP extension or live external MCP client behavior actually requires it.
- Do not reintroduce `initialize`/`initialized` or `Mcp-Session-Id` for the MCP `2026-07-28` path.
- Preserve header-based routing/authorization metadata.

## OAuth client registration

Support/document the registration methods external MCP client exposes, but do not implement unnecessary infrastructure.

Priority:

1. **CIMD** — preferred current MCP direction when supported by the chosen IdP/external MCP client path.
2. **User-defined OAuth client** — MUST work as the deterministic Plan 029 baseline.
3. **DCR** — compatibility only; enable only when the external IdP provides a real secure registration endpoint.

Never silently downgrade to an insecure/unvalidated registration mode.

Plan 029 does **not** implement its own OAuth client registration database.

## OAuth roles

The Relay Agent remains an OAuth **Protected Resource / Resource Server**.

The external provider is responsible for:

- authorization endpoint,
- token endpoint,
- Authorization Code flow,
- PKCE S256,
- client registration/CIMD/DCR capability,
- refresh token issuance,
- refresh token rotation,
- OIDC discovery/userinfo if enabled,
- key lifecycle/JWKS publication.

The Relay Agent is responsible for:

- Protected Resource Metadata,
- resource identifier,
- Bearer validation,
- issuer validation,
- audience/resource validation,
- expiry/not-before validation,
- trusted algorithms/JWK validation,
- subject ownership,
- OAuth scope enforcement per tool,
- correct OAuth challenge responses.

---

# Tool authorization model

Initial server-owned scopes:

- `relay.tools.read` — tool/discovery metadata only.
- `relay.search` — `web_search`.
- `relay.http.fetch` — `http_fetch`.
- `relay.terminal.execute` — `terminal_exec`.

Rules:

- default deny,
- a valid token does not imply permission for every tool,
- `relay.tools.read` never grants terminal execution,
- required scopes are server-owned metadata, never request-provided,
- scope validation occurs before dispatch/side effects,
- subject ownership occurs before dispatch/side effects,
- external MCP client approvals cannot override server denial,
- tool annotations cannot override server denial,
- missing/invalid auth is distinct from insufficient authorization.

`offline_access` is **not an MCP resource permission**.

It belongs at the Authorization Server/OIDC refresh-token layer and SHOULD NOT be advertised by the relay as a required Protected Resource scope.

---

# Tool risk/annotation model

Use standard MCP tool hints because external MCP client/admin UX can benefit from accurate risk classification.

Annotations are advisory only.

Required annotation audit:

- `readOnlyHint`
- `destructiveHint`
- `idempotentHint`
- `openWorldHint`

Initial conservative intent:

### `terminal_exec`

- `readOnlyHint: false`
- `destructiveHint: true`
- `idempotentHint: false`
- `openWorldHint: true`

Reason: it can edit/delete files, run network tools, invoke package managers, and perform arbitrary sandboxed coding operations.

### `web_search`

- `readOnlyHint: true`
- `destructiveHint: false`
- `openWorldHint: true`
- `idempotentHint: true` only if actual semantics justify it.

### `http_fetch`

Do not falsely mark the current generic tool read-only while it supports mutating HTTP methods.

Before publishing, choose one:

1. split read-only HTTP retrieval from mutating HTTP requests, **or**
2. conservatively annotate generic `http_fetch` as potentially mutating/open-world.

Do not use annotations as OAuth authorization decisions.

---

# Phase 0 — Delta audit and implementation freeze

### Objective

Confirm what Plan 028 already provides and freeze only real Plan 029 work.

### Tasks

- [ ] Re-read current `dev` implementations of `transport.rs`, `mcp.rs`, `config.rs`, `security.rs`, `execution.rs`, CI, and relay release workflow.
- [ ] Create/update `.agents/memories/029-external-mcp-mcp-integration-decisions.md`.
- [ ] Mark each required capability as `EXISTING`, `PARTIAL`, or `MISSING`.
- [ ] Record the exact current external MCP client custom-app OAuth UI fields.
- [ ] Re-check official OpenAI developer-mode/custom-MCP documentation.
- [ ] Re-check current MCP authorization/client-registration documentation.
- [ ] Freeze the tested external MCP client plan/mode for final write-capable E2E.
- [ ] Freeze the external OAuth/OIDC provider choice.
- [ ] Freeze one canonical MCP resource identifier.
- [ ] Freeze initial OAuth scopes.
- [ ] Do not implement duplicate Plan 028 functionality.

### Exit criteria

- [ ] Every implementation task maps to a documented `PARTIAL` or `MISSING` gap.
- [ ] No unit-test work is scheduled.

---

# Phase 1 — external MCP client-grade MCP tool contract

### Objective

Make the existing tool catalog accurate, stable, understandable, risk-classified, and suitable for external MCP client `Scan Tools`.

### Tasks

- [ ] Keep existing tool names unless a real compatibility bug requires a breaking rename.
- [ ] Add top-level `title` where useful/supported.
- [ ] Add accurate MCP annotations.
- [ ] Add `outputSchema` only where output can be described reliably; do not invent structured contracts over free-form terminal output.
- [ ] Ensure every input schema matches actual runtime behavior.
- [ ] Remove schema fields execution ignores.
- [ ] Align schema limits with runtime limits.
- [ ] Keep `additionalProperties: false` where appropriate.
- [ ] Make descriptions concise and action-oriented for model selection.
- [ ] Explicitly communicate terminal side effects/risk in descriptor metadata.
- [ ] Resolve `http_fetch` read-vs-write semantics before publication.
- [ ] Keep deterministic tool ordering.
- [ ] Keep cache metadata deterministic where MCP list responses require it.
- [ ] Do not add external MCP client-specific execution code to tool handlers.

### Exit criteria

- [ ] `Scan Tools` shows exactly the intended tools.
- [ ] tool descriptors match actual behavior.
- [ ] risk hints are conservative and accurate.
- [ ] annotations are not consumed by authorization code.

---

# Phase 2 — OAuth discovery and registration compatibility

### Objective

Make external MCP client Advanced OAuth settings discover real metadata without turning the relay into an Authorization Server.

### Tasks

- [ ] Keep `/.well-known/oauth-protected-resource` as relay-owned metadata.
- [ ] Ensure `resource` exactly matches the resource/audience policy enforced by the relay.
- [ ] Advertise only real Authorization Server issuer/base URL values.
- [ ] Advertise only resource-specific scopes from Protected Resource Metadata.
- [ ] Do not advertise `offline_access` as a required MCP resource scope.
- [ ] Ensure external MCP client can discover real Authorization Server metadata:
  - [ ] authorization endpoint,
  - [ ] token endpoint,
  - [ ] issuer/base,
  - [ ] `scopes_supported`,
  - [ ] PKCE S256 capability,
  - [ ] refresh/offline capability,
  - [ ] registration endpoint only if DCR exists.
- [ ] User-defined OAuth Client MUST work.
- [ ] CIMD SHOULD be used when the chosen provider + external MCP client path support it.
- [ ] DCR MUST NOT be advertised unless the chosen provider actually supports it securely.
- [ ] Configure exact external MCP client callback URL copied from the live UI.
- [ ] Never guess or globally hardcode connector-instance callback URLs.
- [ ] Verify the configured token endpoint auth method matches the selected external MCP client registration mode.
- [ ] Do not create a Rust OAuth registration database.

### Exit criteria

- [ ] external MCP client Advanced OAuth settings populate with real values.
- [ ] User-defined OAuth client completes authorization.
- [ ] CIMD/DCR options shown by external MCP client match actual advertised support.

---

# Phase 3 — OAuth durability and identity

### Objective

Make authorization durable, least-privilege, and safe for a command-capable remote MCP server.

### Tasks

- [ ] External Authorization Server handles Authorization Code flow.
- [ ] Require PKCE S256 for public-client flow.
- [ ] Configure refresh-token issuance for durable external MCP client connectivity.
- [ ] Configure refresh-token rotation/reuse policy at the IdP.
- [ ] Advertise/request `offline_access` or provider equivalent only at AS/OIDC layer when needed.
- [ ] Verify access token renewal without user re-login.
- [ ] Verify revoked refresh credentials require reauthorization.
- [ ] Keep refresh tokens out of Relay Agent tool/runtime state.
- [ ] OIDC remains optional.
- [ ] If OIDC is enabled, use real provider discovery/userinfo.
- [ ] Keep `iss + sub` as stable identity anchors.
- [ ] Treat email/domain as supplemental identity attributes only.
- [ ] Never trust an unverified email string as the owner boundary.

### Exit criteria

- [ ] normal access-token expiry does not permanently break the connector.
- [ ] OAuth works without requiring custom login/session code in Rust.

---

# Phase 4 — Per-tool scope and owner authorization

### Objective

Make a valid access token insufficient by itself to execute high-risk tools.

### Tasks

- [ ] Add/freeze server-owned `tool -> required scopes` mapping.
- [ ] enforce `relay.search` for `web_search`.
- [ ] enforce `relay.http.fetch` for `http_fetch`.
- [ ] enforce `relay.terminal.execute` for `terminal_exec`.
- [ ] default deny missing scopes.
- [ ] enforce configured owner `sub` for single-developer deployment.
- [ ] optionally enforce tenant/client identity if the chosen deployment requires it.
- [ ] reject wrong issuer.
- [ ] reject wrong audience/resource.
- [ ] reject wrong subject.
- [ ] reject expired/not-yet-valid tokens.
- [ ] reject unknown/untrusted signing algorithm/key.
- [ ] preserve bounded JWKS caching + refresh-on-unknown-`kid`.
- [ ] execute authorization before tool dispatch/side effects.
- [ ] return HTTP `401` for missing/invalid authentication.
- [ ] return HTTP `403` for authenticated but insufficient authorization.
- [ ] include standards-compatible `WWW-Authenticate` challenges.
- [ ] include Protected Resource Metadata discovery reference where current MCP guidance requires/supports it.
- [ ] expose `invalid_token`/`insufficient_scope` semantics without leaking internal token details.

### Exit criteria

- [ ] correct token + correct subject + correct scope succeeds.
- [ ] every wrong-subject/wrong-scope case fails before execution.

---

# Phase 5 — Remote exposure boundary

### Objective

Make external MCP client reach the relay without converting the developer workstation into a raw public command server.

### Tasks

- [ ] Re-check current OpenAI Secure MCP Tunnel guidance before wiring deployment.
- [ ] Keep local relay bound to loopback by default.
- [ ] Use Secure MCP Tunnel for developer/private-network usage where supported.
- [ ] If using direct remote deployment, require explicit Remote mode + HTTPS.
- [ ] Define trusted proxy/tunnel boundary.
- [ ] Never trust arbitrary `X-Forwarded-*` headers from untrusted peers.
- [ ] Preserve Authorization and MCP routing headers through the trusted edge.
- [ ] Fail closed when effective HTTPS/proxy identity cannot be established.
- [ ] Add bounded request rate limits.
- [ ] Keep execution concurrency bounded.
- [ ] Add per-subject execution limiting where practical.
- [ ] Do not add stream-specific resource machinery unless streaming is actually required.
- [ ] Ensure public health/discovery endpoints expose no sensitive filesystem/config state.

### Exit criteria

- [ ] external MCP client can reach the remote MCP endpoint.
- [ ] no unauthenticated internet path reaches tool execution.
- [ ] trusted-proxy assumptions are explicit and fail closed.

---

# Phase 6 — Operational safety and observability

### Objective

Make production debugging possible without leaking code, credentials, or command secrets.

### Tasks

- [ ] structured request/correlation ID.
- [ ] record method/tool/outcome/latency/status safely.
- [ ] record stable privacy-conscious subject identifier where needed.
- [ ] never log Authorization headers/access tokens/refresh tokens.
- [ ] do not log full terminal commands/tool arguments by default.
- [ ] redact sensitive metadata.
- [ ] bound log message size.
- [ ] bound metric cardinality.
- [ ] prevent log injection from attacker-controlled strings.
- [ ] record auth failures, authorization failures, rate-limit events, tool failures, timeouts, and sandbox failures separately.
- [ ] define retention/redaction behavior.

### Exit criteria

- [ ] connector/auth failures can be diagnosed without exposing secrets or source content.

---

# Phase 7 — external MCP client real E2E acceptance

### Objective

Use the live external MCP client product as the integration acceptance test.

No unit-test substitute is required or requested.

### Setup

- [ ] use a external MCP client plan/mode that currently supports full MCP write/modify actions.
- [ ] enable developer mode.
- [ ] create a custom MCP app with the real remote MCP URL.
- [ ] select OAuth.
- [ ] verify Advanced OAuth discovery before creating the app.

### Tool discovery

- [ ] `Scan Tools` succeeds.
- [ ] only intended tools are visible.
- [ ] names/descriptions/input schemas are correct.
- [ ] risk annotations are correct.
- [ ] tool order is stable.

### OAuth

- [ ] Auth URL is correct.
- [ ] Token URL is correct.
- [ ] Authorization Server base is correct.
- [ ] Resource is correct.
- [ ] Registration URL appears only when DCR is genuinely supported.
- [ ] CIMD availability matches advertised support.
- [ ] User-defined OAuth client flow succeeds.
- [ ] exact external MCP client callback URI succeeds.
- [ ] PKCE S256 succeeds.
- [ ] access-token refresh succeeds.
- [ ] OIDC fields are correct if OIDC is enabled.

### Least privilege

- [ ] search-scoped authorization can call `web_search`.
- [ ] search-only authorization cannot call `terminal_exec`.
- [ ] HTTP-fetch-scoped authorization can call `http_fetch`.
- [ ] HTTP-fetch-only authorization cannot call `terminal_exec`.
- [ ] terminal authorization can call `terminal_exec`.
- [ ] wrong subject fails.
- [ ] wrong issuer fails.
- [ ] wrong resource/audience fails.
- [ ] expired token fails.
- [ ] revoked credentials fail/re-authenticate as expected.

### Real coding workflow

- [ ] inspect repository.
- [ ] read files.
- [ ] create a file.
- [ ] edit an existing file.
- [ ] move/rename a file.
- [ ] delete an in-scope file.
- [ ] run shell chaining needed for coding.
- [ ] run Git read operation.
- [ ] run Git write/local operation where intended.
- [ ] run package manager/build command.
- [ ] run compiler/runtime command.
- [ ] verify resulting output/artifact.

### Security-negative E2E

- [ ] attempt write outside execution root -> blocked.
- [ ] attempt symlink/path escape -> blocked by Plan 028 boundary.
- [ ] attempt privilege escalation -> blocked.
- [ ] attempt Docker host escape -> blocked according to Plan 028 policy.
- [ ] attempt sandbox/no-guard override injection -> blocked.
- [ ] malformed MCP metadata -> rejected before execution.
- [ ] missing scope -> rejected before execution.
- [ ] wrong subject -> rejected before execution.
- [ ] invalid token -> rejected before execution.
- [ ] external MCP client confirmation/approval cannot override server denial.

### Exit criteria

- [ ] external MCP client can perform a realistic coding task from inspect -> edit -> build/run -> verify.
- [ ] server-side auth/sandbox boundaries remain authoritative throughout the real external MCP client session.

---

# Phase 8 — Published-app contract lifecycle

### Objective

Avoid breaking external MCP client after the workspace has approved a frozen tool snapshot.

### Tasks

- [ ] treat tool names as stable public API identifiers after publication.
- [ ] record a canonical tool-catalog hash/snapshot.
- [ ] prefer additive optional schema changes.
- [ ] treat tool rename/removal/required-property changes as breaking.
- [ ] treat required-scope changes as security-relevant breaking changes.
- [ ] treat risk-annotation/write-semantics changes as security-relevant changes.
- [ ] document external MCP client Refresh/action-review procedure.
- [ ] do not assume new actions become enabled automatically after server change.
- [ ] ensure old snapshots fail safely rather than widening authorization.
- [ ] for Business workspaces, account for the current limitation that published custom apps may need recreation/republishing to update metadata/tools.
- [ ] for Enterprise/Edu, account for admin Refresh + action-control review behavior.

### Exit criteria

- [ ] server updates cannot silently widen a published app's privileges.

---

# Phase 9 — Zero-bypass CI / release gate

### Objective

Compensate for the intentional lack of unit-test gate with strict deterministic build/security/conformance checks.

### Required CI gates

- [ ] `cargo fmt --all -- --check`.
- [ ] `cargo check --workspace --all-targets --all-features --locked`.
- [ ] warnings denied for the Rust check/build path.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- [ ] `cargo audit`.
- [ ] no broad `#[allow(...)]` / lint suppression used to bypass failures.
- [ ] no `continue-on-error`, `|| true`, swallowed exits, or equivalent security-gate bypass.
- [ ] release depends on the strict quality/security gate.

### Deterministic connector checks

These are protocol/config/conformance checks, not a unit-test suite.

- [ ] canonical `/mcp` route exists.
- [ ] no deprecated legacy SSE session core becomes canonical.
- [ ] Remote mode cannot run unauthenticated.
- [ ] Protected Resource Metadata is valid.
- [ ] resource identifier is consistent with token validation.
- [ ] `offline_access` is not advertised as a required MCP resource permission.
- [ ] authorization metadata contains real endpoints, not placeholders.
- [ ] required tool scope mapping exists.
- [ ] tool annotations exist and are not referenced as auth decisions.
- [ ] `WWW-Authenticate` invalid-token semantics are present.
- [ ] insufficient-scope semantics are present.
- [ ] no external MCP client-specific route bypasses canonical authentication/authorization/tool dispatch.
- [ ] tool contract snapshot/change detection is reviewed for breaking/security changes.
- [ ] no Node/pkg relay runtime is reintroduced.

### Explicit non-requirements

- [ ] no new Rust unit-test module required.
- [ ] no new JS unit-test suite required.
- [ ] no `cargo test` completion gate required.
- [ ] no test-only security bypass/environment hook allowed in production code.

### Exit criteria

- [ ] build/security/conformance gates are green with zero warning bypass.
- [ ] release cannot publish if a required gate fails.

---

# Phase 10 — Final production readiness

Plan 029 is `COMPLETED` only when all of the following are true:

- [ ] no Plan 028 subsystem was unnecessarily duplicated.
- [ ] canonical MCP `2026-07-28` path remains authoritative.
- [ ] external MCP client `Scan Tools` succeeds.
- [ ] tool descriptors/schemas accurately match runtime behavior.
- [ ] tool annotations are accurate and advisory only.
- [ ] Advanced OAuth settings show correct discovered values.
- [ ] User-defined OAuth client works.
- [ ] CIMD works if advertised.
- [ ] DCR works if advertised.
- [ ] PKCE S256 works.
- [ ] refresh-token flow works.
- [ ] OIDC works if enabled, while core OAuth remains independent of OIDC.
- [ ] resource/issuer/audience/subject checks are strict.
- [ ] per-tool scopes are enforced before dispatch.
- [ ] OAuth challenges distinguish invalid auth from insufficient scope.
- [ ] local relay remains private by default.
- [ ] approved remote/tunnel path works.
- [ ] rate/concurrency controls are bounded.
- [ ] Plan 028 sandbox remains authoritative.
- [ ] no secret/token/source leakage appears in operational logs.
- [ ] real external MCP client coding E2E passes.
- [ ] security-negative external MCP client E2E fails closed.
- [ ] tool snapshot/change-management workflow is documented.
- [ ] strict static/security/conformance CI passes.
- [ ] release gate passes.
- [ ] no unit-test requirement has been added back into the Plan 029 completion gate.
- [ ] `.agents/memories/029-external-mcp-mcp-integration-decisions.md` records actual external MCP client behavior, chosen IdP, registration mode, resource/scopes, tool catalog snapshot, E2E evidence, and accepted limitations.

---

## Definition of Done

The existing Plan 028 Rust Relay Agent can be consumed as a production-grade **custom external MCP client MCP app** for real coding workflows through the current MCP `2026-07-28` protocol, using a trusted external OAuth/OIDC provider, correct Protected Resource/Authorization Server discovery, durable refresh-token behavior, strict per-tool scopes and subject ownership, accurate MCP tool annotations, private/secured remote exposure, the existing Plan 028 sandbox, and zero-bypass CI/release gates.

Plan 029 deliberately does **not** require a unit-test suite. Confidence for this deadline comes from strict static/security checks plus real external MCP client protocol/OAuth/security/coding E2E and final manual review.

Public Plugin Directory submission, interactive Apps SDK UI, and non-external MCP client client certification are separate follow-up scopes.