# Plan 029 — Production ChatGPT Native MCP Coding Agent Integration

**Status: IN FLIGHT**

## Goal

Turn the existing Plan 028 Rust Relay Agent into a production-grade **remote MCP coding-agent endpoint for ChatGPT** without rebuilding the MCP core, OAuth Resource Server, or execution sandbox that already exists.

Plan 029 is a **delta/integration plan**. Plan 028 remains authoritative for filesystem containment, process isolation, privilege boundaries, Docker restrictions, resource limits, and the existing Rust MCP execution core.

The target is intentionally a **real coding agent**, not a read-only automation connector. Once the intended developer is authenticated and authorized, ChatGPT must be able to perform normal software-development work inside the configured workspace: inspect, create, edit, move and delete files; run shells/interpreters; use Git; install dependencies; run package managers, compilers and build systems; make network requests; and use Docker where allowed by the Plan 028 sandbox policy.

The primary target is:

- current MCP `2026-07-28`,
- ChatGPT custom MCP apps in developer mode,
- full read/write/modify coding workflows on ChatGPT plans where OpenAI currently supports full MCP,
- external OAuth/OIDC provider integration,
- real ChatGPT `Scan Tools` + OAuth + coding E2E,
- production-grade security and CI/release gates **without adding a unit-test requirement**.

---

# Scope decisions

## 1. No unit-test gate

This is an explicit delivery decision for speed.

Plan 029 MUST NOT add or restore Rust/TypeScript unit-test suites as a completion requirement.

Completion MUST NOT depend on:

- `cargo test`,
- JS unit tests,
- test-only mocks/fixtures,
- test-only production hooks,
- production security bypasses used only to make tests easier.

Confidence for this deadline comes from:

- strict compile/static/lint checks,
- dependency/security audit,
- deterministic protocol/metadata validation scripts,
- real OAuth discovery/authorization verification,
- real ChatGPT `Scan Tools`,
- real ChatGPT tool calls,
- negative authentication/authorization checks,
- real Plan 028 sandbox escape checks,
- real coding E2E,
- manual final security review.

## 2. ChatGPT is the Plan 029 release target

Plan 029 is complete when the existing Rust Relay Agent works as a secure custom MCP coding app in ChatGPT.

Claude/other MCP clients MAY be used as secondary interoperability checks, but are **not Plan 029 release gates**.

The MCP/tool contract must remain vendor-neutral so future clients can reuse the same server without a fork.

## 3. Custom ChatGPT MCP app first

Plan 029 targets a **custom MCP app** created/tested in ChatGPT developer mode and published inside the intended workspace.

Out of scope:

- public Plugin Directory listing,
- OpenAI registry submission/review,
- marketplace/discovery copy,
- public multi-tenant SaaS onboarding,
- interactive Apps SDK UI/components,
- billing/monetization,
- broad external-customer onboarding.

If public Plugin Directory distribution is wanted later, create a separate plan.

## 4. Headless MCP coding tools only

The Relay Agent is a coding/execution backend, not an interactive ChatGPT UI app.

Do not add Apps SDK UI resources/components unless a later product requirement explicitly needs them.

## 5. Plan 028 remains the execution security boundary

ChatGPT confirmation prompts, MCP annotations, OAuth metadata, tool descriptions, and connector UI settings are **not** execution security boundaries.

Every side effect must still follow:

```text
request
  -> MCP protocol validation
  -> authentication
  -> owner/subject authorization
  -> coding capability authorization
  -> argument/schema validation
  -> Plan 028 sandbox/resource policy
  -> side effect
```

## 6. Coding-agent usability is a first-class requirement

**Do not secure this coding agent by blocking ordinary coding behavior.**

The security model protects **boundaries**, not individual normal development commands.

Normal in-scope coding behavior MUST remain usable, including where already supported by Plan 028:

- shell chaining and scripts (`sh`, `bash`, `zsh`),
- interpreters/eval modes needed for development (`python`, `node`, etc.),
- file create/edit/delete/move inside the allowed workspace,
- Git read/write/local operations,
- package managers (`npm`, `pnpm`, `yarn`, `pip`, Cargo tooling, etc.),
- compilers/build systems (`cargo`, `rustc`, `make`, `gcc`, language runtimes),
- network tools required by builds/dependency installation,
- archive/build utilities,
- Docker build/run workflows that stay inside the approved Plan 028 Docker policy,
- long enough timeouts/output limits for realistic builds while remaining bounded.

Plan 029 MUST NOT introduce a new broad denylist merely because a command can mutate files. `rm`, shells, compilers, Git, package managers and interpreters are expected coding capabilities.

Still forbidden are **boundary escapes**, including:

- root/elevation (`sudo`, `su`, `doas`, `pkexec`, equivalent privilege escalation),
- filesystem escape outside configured workspace/home boundary,
- sandbox-disable/no-guard paths,
- host-root Docker escape, privileged containers, host namespace/device/socket escape,
- authentication/authorization bypass,
- resource-limit bypass,
- secret/token exfiltration through server-controlled metadata/logging.

If a restriction breaks a normal coding workflow but does not materially strengthen one of these boundaries, prefer removing/narrowing the restriction rather than weakening the coding experience.

## 7. Authorization should match the real capability model

`terminal_exec` is intentionally powerful and can already perform many operations that narrower tools also expose (network access, Git, package managers, file mutation). Pretending that every capability is strongly isolated by separate OAuth scopes would provide misleading security once terminal access is granted.

Therefore the primary production authorization model for the intended single-developer coding agent is a **coarse, explicit coding capability**:

- `relay.coding` — authorizes the full intended coding toolset, including `terminal_exec` and the coding operations reachable through it.
- `relay.tools.read` — optional discovery/metadata scope when useful.

Optional narrow scopes MAY also exist for deployments that intentionally expose only a subset of tools:

- `relay.search`,
- `relay.http.fetch`,
- `relay.terminal.execute`.

These narrow scopes are **optional deployment profiles**, not required Plan 029 complexity and not a substitute for the Plan 028 sandbox.

The default ChatGPT coding-app flow SHOULD avoid repeated/fragmented re-authorization across normal coding steps. One explicit `relay.coding` grant to the verified owner is acceptable and is more honest than claiming strong isolation between tools that `terminal_exec` can already subsume.

---

# Current implementation baseline — DO NOT REIMPLEMENT

The following exists on `dev` and must be reused:

- [x] Rust `relay-agent` binary.
- [x] Transport-independent Rust MCP core.
- [x] Canonical `POST /mcp` JSON-RPC endpoint.
- [x] MCP `2026-07-28` protocol constant.
- [x] Stateless MCP request model.
- [x] No legacy `Mcp-Session-Id` authorization boundary.
- [x] `MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name` validation.
- [x] request `_meta` parsing/validation.
- [x] `server/discover`.
- [x] `tools/list`.
- [x] `tools/call`.
- [x] JSON Schema 2020-12-compatible input validation before execution.
- [x] `terminal_exec`, `http_fetch`, `web_search`.
- [x] local-mode fail-closed Origin/Host policy.
- [x] explicit Local vs Remote security mode.
- [x] OAuth Resource Server boundary in Remote mode.
- [x] Bearer token parsing.
- [x] asymmetric JWKS validation.
- [x] JWKS TTL cache + refresh-on-unknown-`kid`.
- [x] bounded JWKS fetch timeout.
- [x] issuer/audience validation foundation.
- [x] Plan 028 filesystem/process/privilege/container sandbox.
- [x] Plan 028 output/timeout/resource controls.
- [x] strict Rust CI/release quality foundation.

If a baseline item is incomplete, reopen only that concrete gap. Do not create parallel implementations.

---

# Current external baseline

Before implementation, re-check current official sources because ChatGPT custom-app behavior is actively evolving:

- OpenAI — Developer mode and MCP apps in ChatGPT:
  `https://help.openai.com/en/articles/12584461-developer-mode-and-full-mcp-connectors-in-chatgpt`
- OpenAI — Apps in ChatGPT:
  `https://help.openai.com/en/articles/11487775-connectors-in-chatgpt`
- OpenAI — Build with the Apps SDK:
  `https://help.openai.com/en/articles/12515353-build-with-the-apps-sdk`
- MCP — `2026-07-28` release:
  `https://blog.modelcontextprotocol.io/posts/2026-07-28/`
- MCP — Authorization:
  `https://modelcontextprotocol.io/specification/draft/basic/authorization`
- MCP — Tool schema/annotations:
  `https://modelcontextprotocol.io/specification/2025-11-25/schema`

Implementation MUST prefer current official OpenAI + MCP behavior over stale memories or old SSE tutorials.

Current OpenAI guidance also makes two scope assumptions explicit for Plan 029:

- full MCP write/modify support is currently targeted at Business / Enterprise / Edu,
- ChatGPT connects to remote MCP servers; local/private servers use the supported Secure MCP Tunnel rather than direct localhost access.

---

# ChatGPT compatibility contract

## Product availability

Final write-capable E2E MUST use a ChatGPT plan/mode that currently supports full MCP write/modify actions.

Do not declare full coding compatibility based only on a read/fetch-only flow.

## MCP endpoint

- `POST /mcp` remains canonical.
- Do not rebuild deprecated `/sse` + `/message?session_id=...` architecture.
- Do not add `GET /mcp` or long-lived SSE unless a current MCP extension/live ChatGPT behavior actually requires it.
- Do not reintroduce `initialize`/`initialized` or `Mcp-Session-Id` for MCP `2026-07-28`.
- Preserve header-based routing/authorization metadata.

## OAuth client registration

Priority:

1. **CIMD** — preferred current MCP direction when supported by the chosen IdP/ChatGPT path.
2. **User-defined OAuth client** — MUST work as the deterministic baseline.
3. **DCR** — compatibility only; enable only when the external IdP exposes a real secure registration endpoint.

Never silently downgrade to an insecure/unvalidated registration mode.

Plan 029 does not implement its own OAuth client-registration database.

## OAuth roles

External Authorization Server / IdP owns:

- authorization endpoint,
- token endpoint,
- Authorization Code flow,
- PKCE S256,
- client registration/CIMD/DCR capability,
- refresh-token issuance/rotation,
- OIDC discovery/userinfo if enabled,
- signing key lifecycle/JWKS publication.

Relay Agent owns:

- Protected Resource Metadata,
- resource identifier,
- Bearer validation,
- issuer validation,
- audience/resource validation,
- expiry/not-before validation,
- trusted algorithms/JWK validation,
- owner/subject authorization,
- coding-capability authorization,
- standards-compatible OAuth challenge responses.

`offline_access` is an Authorization Server/OIDC refresh concern, **not an MCP resource permission**.

---

# Tool annotation model

Use current MCP annotations because they improve ChatGPT/admin risk UX, but keep them truthful and non-authoritative.

Annotations are hints only:

- `readOnlyHint`,
- `destructiveHint`,
- `idempotentHint`,
- `openWorldHint`.

### `terminal_exec`

This is an intentionally powerful coding tool. Its static descriptor must be conservative because one invocation may only read while another may edit/delete/build/network-access.

Recommended baseline:

- `readOnlyHint: false`,
- `destructiveHint: true`,
- `idempotentHint: false`,
- `openWorldHint: true`.

Do **not** split normal terminal functionality into dozens of artificial micro-tools solely to make annotations look less risky. Accurate warnings are preferable to lying about capabilities.

### `web_search`

- `readOnlyHint: true`,
- `destructiveHint: false`,
- `openWorldHint: true`,
- `idempotentHint: true` only if actual semantics justify it.

### `http_fetch`

Do not falsely mark generic `http_fetch` read-only while mutating HTTP methods remain supported.

Either conservatively annotate it as potentially mutating/open-world or narrow/split the tool only if doing so improves actual product usability rather than adding ceremony.

Annotations MUST NOT be consumed as authorization decisions.

---

# Phase 0 — Delta audit and implementation freeze

### Objective

Confirm what Plan 028 already provides and freeze only real Plan 029 work.

### Tasks

- [ ] Re-read current `dev` implementations of `transport.rs`, `mcp.rs`, `config.rs`, `security.rs`, `execution.rs`, CI, and relay release workflow.
- [ ] Create/update `.agents/memories/029-chatgpt-mcp-integration-decisions.md`.
- [ ] Mark each capability `EXISTING`, `PARTIAL`, or `MISSING`.
- [ ] Record exact current ChatGPT custom-app OAuth UI fields.
- [ ] Re-check official OpenAI developer-mode/custom-MCP docs.
- [ ] Re-check current MCP authorization/client-registration docs.
- [ ] Freeze the ChatGPT plan/mode used for write-capable E2E.
- [ ] Freeze the external OAuth/OIDC provider.
- [ ] Freeze canonical MCP resource identifier.
- [ ] Freeze `relay.coding` as the default full-coding resource scope unless implementation evidence requires a different name/model.
- [ ] Decide whether optional narrow scopes are worth supporting; do not make them mandatory without a real use case.
- [ ] Do not duplicate Plan 028 functionality.

### Exit criteria

- [ ] Every implementation task maps to a documented `PARTIAL`/`MISSING` gap.
- [ ] No unit-test work is scheduled.
- [ ] No new restriction is scheduled merely because the server is capable of editing/running code.

---

# Phase 1 — ChatGPT-grade MCP tool contract

### Objective

Make the existing tool catalog accurate, stable, understandable and suitable for ChatGPT `Scan Tools` without weakening coding ergonomics.

### Tasks

- [ ] Keep existing tool names unless a real compatibility bug requires change.
- [ ] Add top-level `title` where useful/supported.
- [ ] Add accurate MCP annotations.
- [ ] Add `outputSchema` only where output can be described reliably; do not invent structured contracts over free-form terminal output.
- [ ] Ensure every input schema matches actual runtime behavior.
- [ ] Remove schema fields execution ignores.
- [ ] Align schema limits with runtime limits.
- [ ] Keep `additionalProperties: false` where appropriate.
- [ ] Make descriptions concise/action-oriented for model selection.
- [ ] Clearly state that `terminal_exec` is a full sandboxed coding terminal.
- [ ] Resolve `http_fetch` annotation semantics before publication.
- [ ] Keep deterministic tool ordering/cache metadata where required.
- [ ] Do not add ChatGPT-specific execution code to handlers.
- [ ] Do not split `terminal_exec` into artificial micro-tools unless live ChatGPT behavior proves it necessary.

### Exit criteria

- [ ] `Scan Tools` shows exactly intended coding tools.
- [ ] descriptors match actual behavior.
- [ ] risk hints are accurate without restricting legitimate coding.
- [ ] annotations are not consumed by authorization code.

---

# Phase 2 — OAuth discovery and registration compatibility

### Objective

Make ChatGPT Advanced OAuth settings discover real metadata without turning the relay into an Authorization Server.

### Tasks

- [ ] Keep `/.well-known/oauth-protected-resource` relay-owned.
- [ ] Ensure `resource` matches relay resource/audience validation.
- [ ] Advertise only real Authorization Server issuer/base URLs.
- [ ] Advertise only resource-specific scopes in Protected Resource Metadata.
- [ ] Use `relay.coding` as the primary coding resource scope for the intended full coding app.
- [ ] Do not advertise `offline_access` as MCP resource permission.
- [ ] Ensure ChatGPT discovers real AS metadata: authorization endpoint, token endpoint, issuer/base, scopes, PKCE S256, refresh/offline capability, DCR endpoint only if real.
- [ ] User-defined OAuth Client MUST work.
- [ ] CIMD SHOULD be used when provider + ChatGPT support it.
- [ ] DCR MUST NOT be advertised unless provider actually supports it safely.
- [ ] Configure exact ChatGPT callback URL copied from live UI.
- [ ] Never guess or globally hardcode connector-instance callback URLs.
- [ ] Verify token endpoint auth method matches selected ChatGPT registration mode.
- [ ] Do not create a Rust OAuth registration database.

### Exit criteria

- [ ] Advanced OAuth settings populate real values.
- [ ] User-defined OAuth client completes authorization.
- [ ] CIMD/DCR options shown match actual support.

---

# Phase 3 — OAuth durability, owner binding, and coding authorization

### Objective

Authorize the intended developer for the intended coding capability while keeping the flow simple enough for daily development.

### Tasks

- [ ] External Authorization Server handles Authorization Code flow.
- [ ] Require PKCE S256 for public-client flow.
- [ ] Configure refresh-token issuance and safe rotation/reuse policy.
- [ ] Advertise/request `offline_access` or provider equivalent only at AS/OIDC layer when needed.
- [ ] Verify access-token renewal without user re-login.
- [ ] Verify revoked refresh credentials require reauthorization.
- [ ] Keep refresh tokens out of Relay Agent tool/runtime state.
- [ ] OIDC remains optional.
- [ ] Keep `iss + sub` as stable owner identity anchors.
- [ ] Treat email/domain as supplemental only.
- [ ] Enforce `relay.coding` for the full coding toolset in the default deployment profile.
- [ ] Do not require separate per-tool consent for every ordinary coding step once `relay.coding` is granted.
- [ ] If optional narrow scopes are enabled, ensure they cannot widen into `relay.coding`/terminal access.
- [ ] Default deny requests without a recognized coding/narrow capability.
- [ ] reject wrong issuer, audience/resource, subject, expiry/nbf, algorithm/key.
- [ ] preserve bounded JWKS cache + refresh-on-unknown-`kid`.
- [ ] authorization occurs before dispatch/side effects.
- [ ] return `401` for invalid/missing auth and `403` for authenticated-but-unauthorized requests.
- [ ] expose standards-compatible `WWW-Authenticate` / resource metadata / invalid-token / insufficient-scope semantics where current guidance requires/supports them.

### Exit criteria

- [ ] verified owner + `relay.coding` can use the complete intended coding surface without unnecessary reauthorization churn.
- [ ] wrong owner/resource/issuer/capability fails before execution.
- [ ] optional narrow scopes, if implemented, cannot grant terminal/coding capability accidentally.

---

# Phase 4 — Remote exposure boundary

### Objective

Make ChatGPT reach the relay without turning the workstation into a raw public command server.

### Tasks

- [ ] Re-check current OpenAI Secure MCP Tunnel guidance before deployment wiring.
- [ ] Keep local relay loopback-only by default.
- [ ] Use Secure MCP Tunnel for developer/private-network usage where supported.
- [ ] If direct remote deployment is used, require explicit Remote mode + HTTPS.
- [ ] Define trusted proxy/tunnel boundary.
- [ ] Never trust arbitrary `X-Forwarded-*` from untrusted peers.
- [ ] Preserve Authorization/MCP routing headers through trusted edge.
- [ ] Fail closed when effective HTTPS/proxy identity cannot be established.
- [ ] Keep request/execution concurrency bounded.
- [ ] Add coarse abuse/rate limits that stop DoS without throttling normal builds/tool bursts.
- [ ] Do not add stream-specific machinery unless streaming is actually required.
- [ ] Ensure public health/discovery endpoints expose no sensitive filesystem/config state.

### Exit criteria

- [ ] ChatGPT reaches the MCP endpoint reliably during normal coding bursts/builds.
- [ ] no unauthenticated internet path reaches execution.
- [ ] proxy assumptions are explicit/fail-closed.

---

# Phase 5 — Operational safety and observability

### Objective

Make production debugging possible without leaking code, credentials or command secrets.

### Tasks

- [ ] structured request/correlation ID.
- [ ] record method/tool/outcome/latency/status safely.
- [ ] record privacy-conscious subject identifier where needed.
- [ ] never log Authorization/access/refresh tokens.
- [ ] do not log full terminal commands/tool arguments by default.
- [ ] redact sensitive metadata.
- [ ] bound log size/metric cardinality.
- [ ] prevent log injection.
- [ ] distinguish auth failures, authorization failures, rate limits, tool failures, timeouts and sandbox failures.
- [ ] define retention/redaction behavior.

### Exit criteria

- [ ] connector/auth failures can be diagnosed without exposing secrets/source content.

---

# Phase 6 — ChatGPT real coding E2E acceptance

### Objective

Use the live ChatGPT product as the acceptance test for the real intended coding workflow.

**No unit-test substitute is required or requested.**

### Setup

- [ ] use Business / Enterprise / Edu (or another current plan/mode explicitly documented by OpenAI as supporting full MCP write/modify actions).
- [ ] enable developer mode.
- [ ] create custom MCP app with real remote MCP URL.
- [ ] select OAuth.
- [ ] verify Advanced OAuth discovery before app creation.

### Tool discovery

- [ ] `Scan Tools` succeeds.
- [ ] only intended tools are visible.
- [ ] names/descriptions/schemas are correct.
- [ ] risk annotations are correct.
- [ ] tool order is stable.

### OAuth

- [ ] Auth URL, Token URL, Authorization Server base and Resource are correct.
- [ ] Registration URL appears only if DCR is real.
- [ ] CIMD availability matches advertised support.
- [ ] User-defined OAuth client flow succeeds.
- [ ] exact ChatGPT callback URI succeeds.
- [ ] PKCE S256 succeeds.
- [ ] access-token refresh succeeds.
- [ ] OIDC fields are correct if enabled.
- [ ] verified owner receives/uses `relay.coding` for full coding mode.

### Coding capability

With the intended owner + `relay.coding`:

- [ ] `web_search` works.
- [ ] `http_fetch` works.
- [ ] `terminal_exec` works.
- [ ] shell chaining works.
- [ ] Python/Node/interpreter usage needed by development works.
- [ ] inspect/read repository.
- [ ] create/edit/move/delete in-scope files.
- [ ] Git read operations work.
- [ ] Git local write operations needed for development work.
- [ ] package installation works.
- [ ] compiler/build/runtime commands work.
- [ ] network-dependent build/dependency steps work within policy.
- [ ] Docker coding/build workflow works where Plan 028 policy permits it.
- [ ] realistic build duration/output does not hit unnecessarily tight limits.
- [ ] resulting artifact/output can be verified.

### Authorization negative checks

- [ ] no/invalid token -> rejected before execution.
- [ ] wrong owner `sub` -> rejected before execution.
- [ ] wrong issuer/resource/audience -> rejected.
- [ ] expired/revoked credentials -> fail/re-authenticate as expected.
- [ ] token without `relay.coding` cannot use the full coding terminal.
- [ ] optional narrow scopes, if implemented, do not accidentally imply `relay.coding`.

### Sandbox/boundary negative checks

- [ ] write outside execution root -> blocked.
- [ ] symlink/path escape -> blocked.
- [ ] privilege escalation -> blocked.
- [ ] Docker host escape -> blocked according to Plan 028 policy.
- [ ] sandbox/no-guard override -> blocked.
- [ ] malformed MCP metadata -> rejected before execution.
- [ ] ChatGPT confirmation cannot override server denial.

### Exit criteria

- [ ] ChatGPT can complete a realistic coding task from inspect -> edit -> install/build/run -> verify without artificial command restrictions.
- [ ] server-side owner/auth/sandbox boundaries remain authoritative.

---

# Phase 7 — Published-app contract lifecycle

### Objective

Avoid breaking ChatGPT after the workspace approves a frozen tool snapshot.

### Tasks

- [ ] treat tool names as stable public API identifiers after publication.
- [ ] record canonical tool-catalog hash/snapshot.
- [ ] prefer additive optional schema changes.
- [ ] treat tool rename/removal/required-property changes as breaking.
- [ ] treat coding-scope/risk-annotation/write-semantics changes as security-relevant changes.
- [ ] document ChatGPT Refresh/action-review procedure.
- [ ] do not assume new actions become enabled automatically.
- [ ] old snapshots fail safely rather than widening authorization.
- [ ] account for current Business recreate/republish limitations when tools/metadata change.
- [ ] account for Enterprise/Edu Refresh + action-control review behavior.

### Exit criteria

- [ ] server updates cannot silently widen a published app's privileges.

---

# Phase 8 — Zero-bypass CI / release gate

### Objective

Compensate for intentional lack of unit-test gate with strict deterministic build/security/conformance checks.

### Required CI gates

- [ ] `cargo fmt --all -- --check`.
- [ ] `cargo check --workspace --all-targets --all-features --locked`.
- [ ] warnings denied for Rust check/build path.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- [ ] `cargo audit`.
- [ ] no broad lint/security suppression used to bypass failures.
- [ ] no `continue-on-error`, `|| true`, swallowed exits or equivalent gate bypass.
- [ ] release depends on strict quality/security gate.

### Deterministic connector checks

These are protocol/config/conformance checks, not a unit-test suite.

- [ ] canonical `/mcp` route exists.
- [ ] deprecated legacy SSE session core is not canonical.
- [ ] Remote mode cannot run unauthenticated.
- [ ] Protected Resource Metadata is valid.
- [ ] resource identifier matches token validation.
- [ ] `offline_access` is not advertised as MCP resource permission.
- [ ] authorization metadata contains real endpoints, not placeholders.
- [ ] `relay.coding` mapping exists for the intended full-coding profile.
- [ ] optional narrow-scope mappings cannot imply/widen full coding scope.
- [ ] tool annotations exist/are valid but are never used as auth decisions.
- [ ] OAuth invalid-token/insufficient-scope challenge semantics are present.
- [ ] no ChatGPT-specific route bypasses canonical auth/dispatch/sandbox core.
- [ ] no new broad coding-command denylist is introduced by Plan 029 without documented boundary rationale.
- [ ] tool-contract snapshot/change detection is reviewed for breaking/security changes.
- [ ] no Node/pkg relay runtime is reintroduced.

### Explicit non-requirements

- [ ] no new Rust unit-test module required.
- [ ] no new JS unit-test suite required.
- [ ] no `cargo test` completion gate required.
- [ ] no test-only security bypass/environment hook allowed in production.

### Exit criteria

- [ ] build/security/conformance gates are green with zero warning bypass.
- [ ] release cannot publish if a required gate fails.

---

# Phase 9 — Final production readiness

Plan 029 is `COMPLETED` only when:

- [ ] no Plan 028 subsystem was unnecessarily duplicated.
- [ ] canonical MCP `2026-07-28` path remains authoritative.
- [ ] ChatGPT `Scan Tools` succeeds.
- [ ] descriptors/schemas accurately match coding behavior.
- [ ] annotations are truthful and advisory only.
- [ ] Advanced OAuth settings show correct discovered values.
- [ ] User-defined OAuth client works.
- [ ] CIMD/DCR work only if advertised.
- [ ] PKCE S256 works.
- [ ] refresh-token flow works.
- [ ] OIDC works if enabled while core OAuth remains independent.
- [ ] resource/issuer/audience/owner checks are strict.
- [ ] `relay.coding` authorizes the intended full coding workflow without unnecessary per-action auth friction.
- [ ] optional narrow scopes, if present, cannot escalate into coding/terminal access.
- [ ] OAuth challenges distinguish invalid auth from insufficient authorization.
- [ ] local relay remains private by default.
- [ ] approved remote/tunnel path works.
- [ ] rate/concurrency controls stop abuse without breaking ordinary builds.
- [ ] normal shells/interpreters/Git/package managers/build tools/file mutation remain usable inside the workspace.
- [ ] Plan 028 sandbox remains authoritative for filesystem/privilege/Docker/process boundaries.
- [ ] no secret/token/source leakage appears in operational logs.
- [ ] real ChatGPT coding E2E passes.
- [ ] security-negative E2E fails closed at actual boundaries.
- [ ] tool snapshot/change-management workflow is documented.
- [ ] strict static/security/conformance CI passes.
- [ ] release gate passes.
- [ ] no unit-test requirement has been added back.
- [ ] `.agents/memories/029-chatgpt-mcp-integration-decisions.md` records actual ChatGPT behavior, chosen IdP, registration mode, resource/scopes, tool catalog snapshot, E2E evidence, and accepted limitations.

---

## Definition of Done

The existing Plan 028 Rust Relay Agent is consumable as a production-grade **custom ChatGPT MCP coding agent** through current MCP `2026-07-28`, using a trusted external OAuth/OIDC provider, correct discovery/registration, durable refresh behavior, verified-owner `relay.coding` authorization, accurate MCP annotations, secured remote exposure, the existing Plan 028 sandbox, and zero-bypass CI/release gates.

The finished integration must preserve the thing it is being built for: **ChatGPT can actually code**. Security is enforced at identity, workspace, privilege, container, process, resource and protocol boundaries rather than by broadly forbidding normal development commands.

Plan 029 deliberately does **not** require a unit-test suite. Confidence for this deadline comes from strict static/security checks plus real ChatGPT protocol/OAuth/security/coding E2E and final manual review.

Public Plugin Directory submission, interactive Apps SDK UI, and non-ChatGPT client certification are separate follow-up scopes.
