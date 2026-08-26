# Plan 036 — Public Remote MCP and OAuth Interoperability

**Status:** PARTIALLY VERIFIED / EXTERNAL ACCEPTANCE BLOCKED (2026-08-26)

**Verified now:** first-party server-only Nuxt remote-MCP ownership binding and
HTTPS URL checks exist in source; the public relay is active; the public health,
RFC 9728 protected-resource metadata, and unauthenticated `/mcp` OAuth challenge
were read-only verified on 2026-08-26. No bearer token or callback credential was
handled or logged during this check.

**Still externally blocked:** this workspace has no deployed hosted-Nuxt
runtime to exercise, no reusable authenticated MCP connector session for an
OAuth callback/token-claim proof, and no authorized disposable tool fixture for
approval and negative-case calls. Those claims remain unproven; the existing
2026-08-16 ChatGPT proof is historical evidence, not a substitute for this
closure pass's current connector evidence.

## Goal

Expose the laptop-hosted `ai-tools relay` as a standards-compliant, authenticated public MCP endpoint so that:

1. the Nuxt application can run outside the laptop (for example in Singapore or another hosted region) and still call the laptop's coding tools;
2. ChatGPT and other standards-compliant remote MCP clients can connect to the same MCP endpoint;
3. the laptop remains inbound-closed: it creates an outbound tunnel/edge connection rather than exposing the relay listener directly to the public Internet;
4. OAuth follows the MCP authorization model instead of inventing an application-specific bearer-token flow;
5. the existing execution containment, single-owner authorization, telemetry confidentiality, and layered architecture remain intact.

The intended public resource is:

```text
https://mcp.farismunir.my.id/mcp
```

The intended high-level topology is:

```text
Nuxt (hosted anywhere) ───────┐
                              │ MCP + OAuth
ChatGPT / other MCP clients ──┼──────────────► https://mcp.farismunir.my.id/mcp
                              │                         │
                              └─────────────────────────┘
                                                        │
                                               public HTTPS edge
                                                        │
                                               outbound tunnel
                                                        │
                                                        ▼
                                              laptop / localhost
                                              127.0.0.1:47821
                                                        │
                                                        ▼
                                                 ai-tools relay
                                                        │
                                                        ▼
                                              sandboxed workspace
```

The laptop must not require a public listening socket, public NAT/firewall port-forward, or `0.0.0.0` relay binding.

---

## Why this plan exists

The repository already contains most of the difficult remote-resource-server foundation:

- `ai-tools relay` supports `local` and `remote` security modes;
- remote mode remains loopback-bound and expects HTTPS termination at an explicitly trusted local edge/tunnel;
- `/mcp` already exposes MCP Streamable HTTP behavior;
- OAuth Protected Resource Metadata is already exposed;
- bearer JWTs are validated against issuer, audience, time validity, asymmetric JWKS, owner subject, and `relay.coding` scope;
- MCP tools already advertise OAuth security schemes;
- unauthenticated/insufficient-scope tool calls already carry `mcp/www_authenticate` metadata;
- execution remains bounded by the existing Rust sandbox/process-safety path;
- Plan 035 already established request/trace/telemetry confidentiality requirements across Nuxt and Rust.

Therefore Plan 036 is not a new remote-shell architecture. It is a standards/interoperability and deployment-boundary completion effort around the existing relay.

---

## External protocol targets

Implementation must re-check the current official specifications at task start because MCP/OpenAI interoperability evolves quickly. As of this plan's creation, the intended targets are:

- MCP Streamable HTTP over a stable public HTTPS endpoint;
- MCP authorization using OAuth Protected Resource Metadata and a separate Authorization Server;
- OAuth Authorization Code flow with PKCE for interactive clients;
- resource/audience binding for the MCP resource;
- standards-compliant client discovery/registration behavior supported by current MCP clients;
- ChatGPT remote MCP compatibility using the same endpoint, not a ChatGPT-only proprietary transport.

Do not freeze implementation details merely because this plan names a currently documented mechanism. Re-read the official MCP authorization/transport specification and current OpenAI MCP/ChatGPT documentation before implementation.

---

## Non-goals

This plan does **not** initially aim to:

- turn the Rust relay into an OAuth Authorization Server;
- expose the relay directly on a public interface;
- support arbitrary multi-tenant remote shell access;
- create a hosted job broker, queue, or fleet scheduler unless direct outbound-tunnel architecture proves insufficient;
- replace the existing Bubblewrap/process execution boundary;
- weaken single-owner authorization to make external clients easier to connect;
- introduce a generic DI framework, service locator, or new server architecture;
- introduce a unit-test suite or CI workflow;
- make OpenAI Secure MCP Tunnel the canonical transport, because the same endpoint must also be usable by the custom Nuxt application and other MCP clients.

A brokered multi-device architecture may be a future plan if requirements grow beyond one owner's laptop/relay endpoint.

---

## Architecture decisions to preserve

### 1. Public HTTPS edge, private relay listener

`ai-tools relay` remains loopback-only. A reverse tunnel or equivalent trusted local HTTPS edge provides the public hostname.

Required invariant:

```text
public Internet -> HTTPS endpoint -> outbound-established tunnel -> loopback relay
```

Forbidden shortcut:

```text
public Internet -> laptop:47821
```

Do not broaden `ServerConfig` to allow public plaintext binding merely to simplify deployment.

### 2. Relay remains an OAuth Resource Server

The relay validates access tokens and publishes resource metadata. It does not own login UI, authorization code issuance, consent, token issuance, client registration, or user identity lifecycle.

The Authorization Server is a separate system/domain/service.

Conceptually:

```text
mcp.farismunir.my.id  = MCP Resource Server
Auth service / IdP    = OAuth Authorization Server
Nuxt / ChatGPT        = OAuth clients
```

### 3. Single-owner authorization remains authoritative

A valid token is insufficient by itself. Remote access must continue to enforce the intended owner subject plus required coding scope.

Minimum authorization properties:

```text
issuer matches configured issuer
audience/resource matches public MCP resource
subject matches configured owner
scope contains relay.coding
token signature/time validity pass
```

Do not replace owner binding with UI filtering or client trust.

### 4. One standards-compliant MCP resource for all clients

Nuxt and ChatGPT should consume the same MCP resource contract.

Avoid separate protocol forks such as:

```text
/api/internal-terminal   for Nuxt
/mcp                     for ChatGPT
```

unless a concrete standard/client limitation proves unavoidable.

### 5. No direct server-side shell path in Nuxt

The hosted Nuxt application is a remote MCP client/orchestrator. It must not reintroduce a direct shell execution implementation in Nitro.

The execution authority stays on the laptop in `ai-tools relay`.

---

## Phase 0 — Re-audit current source and external standards

Before editing implementation:

- [ ] Re-read `.agents/README.md`, current canonical memory, Rust/MCP knowledge, and current source on the implementation branch.
- [ ] Re-read official current MCP transport specification.
- [ ] Re-read official current MCP authorization specification.
- [ ] Re-read current OpenAI/ChatGPT remote MCP authentication and connection documentation.
- [ ] Confirm current client-registration/discovery expectations (for example CIMD/DCR/pre-registered client support if still applicable).
- [ ] Confirm whether ChatGPT requires any compatibility behavior beyond standard MCP + OAuth.
- [ ] Inventory current relay endpoint behavior against the official protocol rather than historical Plan 028/029 claims.
- [ ] Re-run/inspect the existing frozen tool catalog and published security metadata.
- [ ] Record a concise compatibility matrix in this plan before implementation starts.

Deliverable: an explicit matrix of `current relay` vs `MCP spec` vs `Nuxt client` vs `ChatGPT client`, with gaps categorized as transport, OAuth, deployment, client integration, or documentation.

---

## Phase 1 — Define the public resource identity

Lock the canonical public MCP resource before OAuth wiring.

Target unless deployment constraints force a reviewed change:

```text
MCP endpoint: https://mcp.farismunir.my.id/mcp
resource/audience: https://mcp.farismunir.my.id/mcp
```

Tasks:

- [ ] Decide exact OAuth resource identifier expected in `aud`/resource handling.
- [ ] Ensure Protected Resource Metadata resolves correctly for the chosen resource path.
- [ ] Ensure `WWW-Authenticate` resource metadata references the public HTTPS resource rather than loopback/internal addresses.
- [ ] Ensure no generated discovery metadata leaks localhost, private tunnel identifiers, or internal topology.
- [ ] Define DNS/TLS ownership and certificate lifecycle expectations.

Acceptance:

- unauthenticated external client can discover the protected resource metadata from the public domain;
- all advertised URLs are canonical HTTPS public URLs;
- no auth metadata points clients at `127.0.0.1` or laptop-local addresses.

---

## Phase 2 — Choose and prove the outbound tunnel/HTTPS edge

Select the smallest operational design that gives a stable public HTTPS endpoint while the laptop initiates outbound connectivity.

Candidate classes include a managed reverse tunnel or an operator-owned reverse proxy/tunnel. The choice must be made from current capabilities and threat model, not assumed in this plan.

Required properties:

- [ ] Laptop establishes outbound connection only.
- [ ] Public endpoint supports HTTPS.
- [ ] Public path forwards `/mcp` and OAuth resource metadata routes correctly.
- [ ] Relay remains bound to loopback.
- [ ] Relay trusts forwarded HTTPS state only from the explicitly configured local tunnel/edge peer.
- [ ] Direct local spoofing of forwarded headers from untrusted peers remains rejected.
- [ ] Tunnel reconnect behavior is understood and documented.
- [ ] DNS/certificate lifecycle does not require exposing the relay listener.
- [ ] Public endpoint does not silently buffer/break required MCP HTTP semantics.

Do not weaken `trusted_proxy`/CIDR checks for convenience.

Acceptance evidence should include:

- relay listening only on loopback;
- successful public `/health` or equivalent non-sensitive liveness check if retained;
- successful public MCP discovery/initialize flow through the tunnel;
- a negative proof that direct non-trusted forwarded-proto spoofing is rejected.

---

## Phase 3 — Authorization Server integration

Integrate a separate standards-compliant Authorization Server rather than implementing OAuth issuance inside the relay.

Required capabilities, subject to current official MCP/OpenAI requirements at implementation time:

- [ ] Authorization endpoint.
- [ ] Token endpoint.
- [ ] Authorization Code flow.
- [ ] PKCE S256.
- [ ] OIDC/OAuth discovery metadata required by the clients in scope.
- [ ] Asymmetric signing keys with discoverable JWKS compatible with current relay validation.
- [ ] Ability to issue access tokens for the public MCP resource/audience.
- [ ] Ability to issue `relay.coding` scope.
- [ ] Stable subject claim usable by `OAUTH_OWNER_SUBJECT`.
- [ ] Client identification/registration compatible with both Nuxt and ChatGPT.
- [ ] Redirect URI validation appropriate to each OAuth client.

Explicitly decide whether the Authorization Server is:

1. an existing hosted identity provider;
2. a separately deployed first-party auth service;
3. another reviewed standards-compliant product.

The Rust relay must not absorb Authorization Server responsibilities simply because it already validates JWTs.

Acceptance:

- a token issued for another audience is rejected;
- a token for another subject is rejected;
- a token without `relay.coding` is rejected/appropriately challenged;
- expired/invalid-signature tokens are rejected;
- the correct owner token succeeds.

---

## Phase 4 — MCP OAuth discovery/challenge interoperability

Audit and harden the current relay behavior against real remote MCP clients.

Areas to verify:

- [ ] Protected Resource Metadata route(s).
- [ ] `authorization_servers` metadata.
- [ ] `scopes_supported`.
- [ ] `WWW-Authenticate` challenge on unauthenticated requests where required.
- [ ] `resource_metadata` value.
- [ ] tool-level OAuth `securitySchemes`.
- [ ] `mcp/www_authenticate` metadata for tool-call auth challenges if still expected by current clients.
- [ ] issuer/JWKS discovery behavior.
- [ ] resource/audience validation.
- [ ] redirect/client registration behavior at the Authorization Server boundary.

Do not add client-specific hacks until a real standards-compliant flow demonstrates a gap.

---

## Phase 5 — Nuxt remote MCP client path

Replace the localhost-only assumption for hosted Nuxt deployments with the standards-compliant remote MCP resource.

Desired flow:

```text
hosted Nuxt
   -> OAuth client/token handling
   -> https://mcp.farismunir.my.id/mcp
   -> outbound tunnel
   -> laptop relay
   -> tool execution
```

Tasks:

- [ ] Inventory the current local-terminal/paired-relay client path and determine what is superseded vs retained.
- [ ] Define whether Nuxt acts as a confidential OAuth client server-side or whether any browser-side OAuth interaction is required.
- [ ] Keep access/refresh tokens out of client-visible state unless the protocol explicitly requires otherwise.
- [ ] Integrate through existing MCP abstractions/application boundaries instead of adding ad hoc HTTP calls in Vue components.
- [ ] Preserve tool approval semantics; remote transport must not imply auto-approval of terminal execution.
- [ ] Preserve conversation ownership, model/provider ownership, and workspace ownership checks.
- [ ] Preserve abort/stop behavior across remote tool calls where practical.
- [ ] Preserve Plan 035 trace/log confidentiality.
- [ ] Ensure tool outputs/errors remain bounded and sanitized exactly as on the current relay path.

Architecture preference:

```text
UI/application chat semantics
        -> application MCP/tool contract
            <- infrastructure remote MCP adapter
```

Do not place OAuth/token/network orchestration directly in Vue components.

Acceptance:

- a Nuxt deployment that cannot reach laptop localhost can call the public MCP endpoint successfully;
- no direct shell/process execution exists in Nitro;
- user approval still gates destructive terminal execution according to current product policy;
- laptop offline/tunnel-down state produces a bounded user-visible failure rather than hanging indefinitely.

---

## Phase 6 — ChatGPT interoperability proof

Prove the same public MCP endpoint against ChatGPT's current supported remote MCP connection flow.

Tasks:

- [x] Connect the public endpoint using ChatGPT's current MCP/custom-app/developer connection mechanism. Live ChatGPT session on 2026-08-16 reached the configured `Masih_Awam_MCP` server.
- [x] Prove MCP server discovery. ChatGPT surfaced the relay tool catalog in the live session.
- [ ] Prove OAuth authorization flow from ChatGPT to the chosen Authorization Server.
- [ ] Prove correct callback/redirect handling.
- [ ] Prove token resource/audience and scope compatibility.
- [x] Prove `tools/list` visibility. `terminal_exec`, `http_fetch`, and `web_search` were available through the connected server.
- [x] Prove at least one safe read-only/non-destructive tool call if available. The live session executed repository inspection commands through `terminal_exec`.
- [ ] Prove terminal tool authorization/approval behavior without weakening server security.
- [ ] Record any ChatGPT-specific interoperability requirement separately from the MCP standard.

If ChatGPT cannot complete the flow, diagnose whether the failure belongs to:

- MCP transport/version;
- Protected Resource Metadata;
- Authorization Server metadata;
- client registration/identification;
- redirect URI handling;
- PKCE;
- audience/resource binding;
- scope/challenge behavior;
- tool schema/response compatibility.

Do not weaken the relay auth model merely to get a green connection UI.

**2026-08-16 live ChatGPT evidence:** connection, discovery/tool visibility, and non-destructive tool execution are proven through this session. OAuth callback internals, resource/audience assertions, destructive-action approval behavior, hosted-Nuxt reachability, and the broader negative-case matrix remain unproven here and stay open.

---

## Phase 7 — Security hardening for a public coding endpoint

Making terminal execution reachable from a public hostname materially increases the attack surface even when OAuth is present.

Re-audit at minimum:

### Network/admission

- [ ] Request admission happens before expensive OAuth/JWKS/tool work.
- [ ] HTTP concurrency remains bounded.
- [ ] Tool execution concurrency remains bounded.
- [ ] Body size remains bounded.
- [ ] Tunnel/proxy forwarding trust remains explicit.
- [ ] Public endpoint cannot reach unintended local services through proxy misconfiguration.

### OAuth

- [ ] Strict issuer.
- [ ] Strict audience/resource.
- [ ] Strict owner subject.
- [ ] Required scope.
- [ ] Asymmetric algorithms only as intended.
- [ ] JWKS URL safety/HTTPS policy preserved.
- [ ] Unknown-key refresh remains bounded.
- [ ] No token values in logs/traces/tool output.

### Tool execution

- [ ] Execution root containment preserved.
- [ ] Forbidden shallow/system roots preserved.
- [ ] Bubblewrap/Linux production boundary preserved.
- [ ] Environment clearing/safe PATH preserved.
- [ ] Timeout/output/process-group cleanup preserved.
- [ ] No Docker socket/root/privileged namespace exposure.
- [ ] Tool arguments/output remain private from telemetry.

### Public errors

- [ ] No filesystem paths in public errors.
- [ ] No auth/JWKS upstream URLs in public errors.
- [ ] No raw provider/process exceptions in MCP results.
- [ ] No secrets in stdout/Loki/trace attributes.

Run the relevant deterministic security/acceptance scripts and add targeted deterministic scripts when a new protocol/security boundary needs executable proof. Do not create a general unit-test suite.

---

## Phase 8 — Observability across hosted Nuxt -> public MCP -> laptop

Extend the current Plan 035 mental model to the public boundary without leaking sensitive information.

Desired trace topology where standards/client behavior allows first-party propagation:

```text
browser / Nuxt request
        -> Nuxt server/application span
            -> outbound MCP client span
                -> public edge/tunnel
                    -> relay request span
                        -> tool execution span
```

Tasks:

- [ ] Preserve server-generated request IDs at each trust boundary.
- [ ] Propagate W3C trace context only through reviewed first-party paths.
- [ ] Never use client-supplied request IDs as authoritative IDs.
- [ ] Never expose bearer tokens, OAuth codes, PKCE verifier, tool arguments, command output, or filesystem paths in telemetry.
- [ ] Distinguish public MCP edge failures, OAuth failures, tunnel-offline failures, and actual tool execution failures using bounded classifications.
- [ ] Ensure public-domain route logging follows the existing low-cardinality/privacy policy.

ChatGPT is a third-party client; do not require or invent internal trace propagation through ChatGPT.

---

## Phase 9 — Deployment and operator workflow

Define a practical laptop operator workflow.

The operator should be able to configure and run the remote relay without editing source.

Document/configure at minimum:

- public MCP URL;
- relay port;
- remote mode;
- allowed/trusted edge settings;
- Authorization Server issuer;
- public MCP audience/resource;
- owner subject;
- execution root;
- tunnel startup/reconnect behavior;
- logs/health troubleshooting;
- clean shutdown/revocation procedure.

Prefer environment/CLI configuration consistent with the existing `ai-tools relay` contract.

Do not place long-lived access tokens in CLI arguments, repository files, shell history, or committed config.

Acceptance:

- fresh operator setup can be followed from documentation;
- restarting the laptop/relay/tunnel restores the public MCP service without reconfiguring application source;
- revoking/rotating auth credentials does not require rebuilding the binary.

---

## Phase 10 — Backward compatibility and migration

The current application has a local paired-terminal path. Decide explicitly whether Plan 036:

1. keeps local mode for same-machine development and adds remote MCP as another transport; or
2. converges the product onto the public MCP resource for both local and hosted usage.

Preference: retain local mode as a secure development/offline path while making remote MCP the deployment path, unless maintaining both creates duplicate product semantics.

Migration questions to resolve:

- [ ] Does `local_terminal` remain a special client-executed tool?
- [ ] Does the remote relay catalog become the authoritative terminal tool surface for hosted deployments?
- [ ] How do existing `user_devices` pairing records relate to OAuth remote authorization?
- [ ] Can pairing metadata be retired, or is it still useful for local-mode UX/device presence?
- [ ] How are remembered approvals represented when the tool source changes from local client execution to remote MCP execution?

Do not silently change destructive-action approval behavior during transport migration.

---

## Phase 11 — Verification matrix

Because the repository intentionally has no CI and no unit-test suite, closure requires explicit local and live verification.

### Mandatory repository gate

```sh
pnpm verify:commit
```

### Build/runtime

```sh
pnpm build
pnpm build:tools
```

Use a clean build/preview when Nuxt runtime behavior is part of the phase.

### Security-sensitive Rust/MCP

Run the applicable deterministic protocol/security scripts and `cargo audit` where appropriate.

### Live acceptance matrix

The final plan cannot close until results are recorded honestly for:

| Client / path | Discovery | OAuth | tools/list | tools/call | offline/error path |
| --- | --- | --- | --- | --- | --- |
| hosted Nuxt -> public MCP -> laptop | required | required | required | required | required |
| ChatGPT -> public MCP -> laptop | required | required | required | required | required |
| local relay mode regression | required as applicable | n/a/local policy | required | required | required |

Also prove negative cases:

- invalid token;
- wrong audience;
- wrong owner subject;
- missing scope;
- expired token;
- relay offline;
- tunnel offline;
- malformed MCP request;
- oversized request;
- command timeout;
- output bound behavior;
- sandbox escape attempts covered by existing deterministic security checks.

Do not mark a live external-client item complete if credentials/client access are unavailable. Record it as UNPROVEN with the exact environmental blocker.

---

## Phase 12 — Documentation and closeout

Before closure:

- [ ] Update README/runtime docs for the supported remote topology.
- [ ] Update `.env.example` for new non-secret configuration keys.
- [ ] Update `.agents/knowledge/project.md` if the shipped architecture changes.
- [ ] Update `.agents/knowledge/tooling.md` with stable operator configuration.
- [ ] Update canonical `.agents/memories/README.md` with durable decisions/traps and remove stale statements superseded by Plan 036.
- [ ] Keep security claims limited to what was actually proven.
- [ ] Record exact verification commands and live acceptance evidence.
- [ ] Run the mandatory closeout review in `.agents/knowledge/self-improvement.md`.
- [ ] Run `pnpm verify:commit` successfully before each normal local implementation commit and before final implementation closeout.

Implementation work must use one or more short-lived branches/PRs targeting `dev`; this plan file itself is docs-only and is created directly on `dev` according to repository policy.

---

## Definition of Done

Plan 036 is complete only when all of the following are true:

1. `https://mcp.farismunir.my.id/mcp` (or a reviewed replacement canonical URL) is a stable public HTTPS MCP resource.
2. The laptop relay remains loopback-bound and public reachability is provided through an outbound-established trusted tunnel/edge.
3. The relay remains an OAuth Resource Server, with authorization handled by a separate standards-compliant Authorization Server.
4. Tokens are strictly validated for issuer, resource/audience, owner subject, scope, signature, and time validity.
5. Hosted Nuxt can execute the intended MCP tools on the laptop without localhost connectivity and without introducing a Nitro shell execution path.
6. ChatGPT can connect to the same MCP endpoint, complete the supported OAuth flow, discover tools, and execute an authorized tool call; if external access is impossible during implementation, that item remains explicitly UNPROVEN and Plan 036 must not claim full interoperability closure.
7. Destructive terminal execution preserves explicit product approval policy and the Rust sandbox/process-safety boundary.
8. Public error responses and observability remain free of secrets, raw command output, filesystem paths, token material, and arbitrary upstream diagnostics.
9. Relevant deterministic security/protocol acceptance checks pass, `pnpm verify:commit` passes, and production build/runtime verification is recorded.
10. Documentation, canonical memory, configuration examples, and this plan accurately describe the shipped system.

---

## Immediate implementation order

When implementation is explicitly authorized, start in this order:

1. Phase 0 compatibility re-audit.
2. Phase 1 canonical public resource identity.
3. Phase 2 outbound tunnel/HTTPS proof with the current relay unchanged where possible.
4. Phase 3 Authorization Server selection/integration.
5. Phase 4 real MCP OAuth interoperability proof.
6. Phase 5 Nuxt remote-client integration.
7. Phase 6 ChatGPT live proof.
8. Phases 7–12 hardening, observability, deployment, migration, verification, and closeout.

Do not start by rewriting the relay transport. First prove which gaps are actually missing from the current implementation.
