# Plan 029 — ChatGPT Native MCP Integration

Status: IN FLIGHT

## Goal

Extend the Rust Relay Agent from the Plan 028 local/legacy-compatible MCP server into a production-grade **remote MCP server** that can be connected natively from ChatGPT and also remain compatible with Claude remote MCP clients.

The primary goal is **not** to build a custom ChatGPT-only protocol. The implementation should target the current MCP standard first, then add compatibility only where a real client requires it. As of the current MCP release (`2026-07-28`), the standard HTTP transport is stateless and uses a single MCP endpoint with POST/GET semantics; the older HTTP+SSE transport is deprecated. ChatGPT currently supports remote MCP apps and full write/modify actions in developer mode for supported Business/Enterprise/Edu plans, and local servers must be reached through the supported secure tunnel rather than being exposed directly. citeturn1search1turn3search0

## Non-goals

- Do not build a bespoke OAuth Authorization Server in Rust unless a later plan explicitly requires it.
- Do not make ChatGPT-specific behavior the core protocol implementation.
- Do not replace the Plan 028 execution sandbox/security boundary.
- Do not add unit-test requirements; this project currently validates behavior with compile/lint/audit checks, protocol conformance checks, focused integration/E2E scenarios, and manual security review.
- Do not expose the relay directly to the public internet merely to make ChatGPT connectivity work.
- Do not treat legacy HTTP+SSE as the primary transport for a new implementation.

## Architecture decisions

### A. MCP transport

Implement the current MCP `2026-07-28` stateless HTTP model as the primary remote transport:

- One canonical MCP endpoint, preferably `POST/GET /mcp`.
- POST accepts self-contained JSON-RPC requests/notifications/responses.
- GET may establish an SSE stream only when the current protocol requires/permits server-to-client streaming; it must not recreate the deprecated 2024-style `/sse` + `/message?session_id=` architecture as the primary design.
- Do not require `Mcp-Session-Id` or the old initialize/initialized session lifecycle for the new protocol version.
- Support `MCP-Protocol-Version` and the new method/name headers required by the current protocol.
- Keep the old `POST /mcp` behavior only as a compatibility path where it does not conflict with the current standard.
- If a real target client still requires legacy HTTP+SSE, implement it as an explicitly isolated compatibility adapter and document its deprecation/removal path.

The current MCP transport specification requires Origin validation to prevent DNS rebinding and recommends localhost binding for local deployments. citeturn1search0turn1search6

### B. ChatGPT connectivity

ChatGPT custom MCP apps connect to **remote MCP servers**. A relay running on a developer machine should therefore be exposed through the supported Secure MCP Tunnel instead of opening a raw public listener. Full MCP write/modify actions are currently available in beta for ChatGPT Business and Enterprise/Edu, while availability and permissions may change. citeturn3search0

The plan must support both deployment modes:

1. Local developer mode: relay bound to loopback and reached by the supported secure tunnel.
2. Remote/server mode: relay deployed behind HTTPS/reverse proxy with strict authentication, authorization, rate limits, and security headers.

### C. OAuth

Keep the Relay Agent as the **OAuth Resource Server**, not the Authorization Server.

Preferred architecture:

```text
ChatGPT / Claude
      |
      | OAuth authorization
      v
External IdP / Authorization Server
      |
      | access token
      v
Remote MCP endpoint
      |
      | JWT/JWKS validation + scope/subject checks
      v
Rust Relay Agent
      |
      v
Plan 028 sandboxed terminal execution
```

Use an established provider (Auth0, Clerk, Supabase Auth, or another OIDC/OAuth provider that satisfies the requirements) instead of implementing account login, consent, authorization-code handling, refresh-token storage, and key management from scratch.

The MCP 2026-07-28 direction formally shifts client registration toward Client ID Metadata Documents (CIMD) and deprecates Dynamic Client Registration (DCR), while retaining compatibility during migration. The implementation should prefer the current standard and isolate any DCR compatibility behind a feature/configuration boundary. citeturn1search1turn1search2

### D. ChatGPT OAuth metadata

ChatGPT expects an MCP app to provide endpoint/metadata information and, when OAuth is configured, the provider must support a refresh-token strategy. OpenAI specifically recommends verifying that the provider advertises and issues `offline_access` (or the provider-equivalent refresh capability), otherwise ChatGPT may lose connectivity after access-token expiry. citeturn0search1

The exact ChatGPT callback URL must be copied from the ChatGPT app setup UI and registered exactly at the IdP; do not hardcode or invent a generic callback URL. citeturn0search3turn0search4

### E. Tool authorization

The MCP server must expose explicit scopes/capabilities. Terminal execution is the highest-risk capability.

Minimum conceptual policy:

- `mcp:tools:list` / discovery: low-risk.
- `mcp:terminal:exec`: high-risk and requires explicit authorization.
- Future tools must declare required scopes instead of inheriting terminal access.
- Authorization must happen before any side effect.
- Subject identity must be bound to the intended developer/account when the relay is exposed remotely.
- Do not accept a valid token from an arbitrary subject merely because the signature is valid.

ChatGPT itself can provide action controls and confirmations for write/modify actions, but these are client-side safety UX and must never replace server-side authorization or sandbox enforcement. citeturn3search0

### F. Claude compatibility

Claude Code supports remote MCP over HTTP and SSE and supports OAuth for remote MCP servers. Anthropic's MCP connector also supports remote HTTP MCP servers with OAuth bearer tokens. Therefore, the canonical implementation should target standard Streamable HTTP first while keeping a narrow compatibility strategy for clients that still require legacy SSE. citeturn2search0turn2search1

---

# Phase 0 — Resource & Protocol Baseline

### Objective

Freeze the external contracts against current official documentation before changing code.

### Tasks

- [ ] Record MCP `2026-07-28` as the target protocol version.
- [ ] Read and document current MCP Streamable HTTP requirements.
- [ ] Read current MCP authorization requirements, including Protected Resource Metadata, Authorization Server Metadata, issuer validation, resource/audience binding, and client registration direction.
- [ ] Read OpenAI ChatGPT custom MCP app/developer-mode requirements.
- [ ] Read OpenAI requirements around OAuth refresh tokens and exact callback URL registration.
- [ ] Read Anthropic remote MCP HTTP/SSE/OAuth compatibility requirements.
- [ ] Inventory Plan 028's current `/mcp`, OAuth, Origin/Host, JWKS, and execution-security behavior.
- [ ] Identify which existing behavior must remain backward compatible and which legacy transport behavior can be deprecated.
- [ ] Store implementation-specific decisions in `.agents/memories/029-chatgpt-mcp-integration-decisions.md`.

### Exit criteria

- [ ] No implementation decision relies on the old 2024/2025 MCP SSE model when the current 2026-07-28 standard provides a better path.
- [ ] Target ChatGPT and Claude client compatibility matrix is documented.

---

# Phase 1 — Canonical Streamable HTTP Transport

### Objective

Implement the current stateless MCP HTTP transport without introducing session state that the current protocol removed.

### Files

- `packages/rust-tools/src/relay_agent/transport.rs`
- `packages/rust-tools/src/relay_agent/mcp.rs`
- relevant router/config/error modules

### Tasks

- [ ] Add/normalize the canonical `/mcp` endpoint for POST and GET.
- [ ] Implement current `MCP-Protocol-Version` handling.
- [ ] Implement required `Mcp-Method` and `Mcp-Name` routing metadata where applicable.
- [ ] Ensure each request is independently routable and does not depend on sticky sessions.
- [ ] Remove any new dependence on `Mcp-Session-Id` for the 2026-07-28 path.
- [ ] Preserve valid JSON-RPC request/notification/response semantics.
- [ ] Implement correct `Accept: application/json, text/event-stream` handling for POST requests.
- [ ] Support GET SSE only for the current protocol's server-to-client streaming semantics where needed.
- [ ] Do not implement `/sse` + `/message?session_id=` as the canonical new protocol.
- [ ] If legacy SSE compatibility is required, isolate it behind a clearly named compatibility module/route.
- [ ] Enforce body-size and request-time limits.
- [ ] Preserve fail-closed Origin/Host protections.
- [ ] Preserve loopback-only default behavior for local mode.

### Exit criteria

- [ ] A current MCP client can POST an initialize/discovery-equivalent request according to the target protocol without requiring a session handshake.
- [ ] Multiple independent requests can be served without shared per-client session state.
- [ ] Legacy compatibility, if present, cannot bypass authentication or execution policy.

---

# Phase 2 — MCP Capability & Tool Contract

### Objective

Make the relay's MCP surface deterministic and safe for ChatGPT tool scanning and Claude tool discovery.

### Tasks

- [ ] Define stable `tools/list` output.
- [ ] Provide complete tool names, descriptions, input schemas, and output schemas.
- [ ] Use JSON Schema 2020-12-compatible schemas where required by the target MCP version.
- [ ] Ensure schemas do not advertise capabilities the server does not actually enforce.
- [ ] Ensure deterministic tool ordering.
- [ ] Add cache hints (`ttlMs`/`cacheScope`) where supported and useful by the target protocol.
- [ ] Return standards-compliant JSON-RPC/MCP errors for malformed requests and invalid tool arguments.
- [ ] Ensure tool argument validation occurs before command execution.
- [ ] Ensure no tool argument can disable sandbox, authentication, authorization, timeout, or resource policies.
- [ ] Explicitly mark terminal execution as a write/dangerous capability in server metadata/tool description.
- [ ] Ensure tool descriptions are concise enough for client tool scanning while still documenting destructive behavior.

### Exit criteria

- [ ] ChatGPT tool scan discovers the intended tools with correct names/schemas.
- [ ] Claude remote MCP discovery sees the same intended tool contract.
- [ ] No tool is exposed accidentally through an internal/debug route.

---

# Phase 3 — OAuth Discovery & Provider Integration

### Objective

Make ChatGPT/Claude able to discover and complete the external OAuth flow without the relay becoming an Authorization Server.

### Tasks

- [ ] Choose the production IdP and document why it satisfies OAuth/OIDC/MCP requirements.
- [ ] Configure issuer, authorization endpoint, token endpoint, JWKS URI, and resource metadata.
- [ ] Implement `/.well-known/oauth-protected-resource` according to the current MCP authorization model.
- [ ] Implement `/.well-known/oauth-authorization-server` only when required by the chosen authorization-server/discovery model.
- [ ] Prefer current client metadata/CIMD behavior; isolate DCR fallback if a target client still needs it.
- [ ] Advertise supported scopes accurately.
- [ ] Advertise refresh/offline capability accurately.
- [ ] Configure exact ChatGPT callback URI from the ChatGPT UI; never guess it.
- [ ] Configure the corresponding Claude/Anthropic OAuth callback requirements where applicable.
- [ ] Configure consent/audience/resource identifiers correctly at the IdP.
- [ ] Ensure refresh tokens are issued for the intended connector flow where required.
- [ ] Ensure client secrets are stored only in the external provider/secret manager and never in the Rust repository.
- [ ] Document local development OAuth versus production OAuth configuration.

### Exit criteria

- [ ] ChatGPT can discover the authorization metadata.
- [ ] OAuth authorization succeeds.
- [ ] Token refresh succeeds without manual re-login.
- [ ] Claude remote MCP OAuth flow succeeds where supported by the chosen client/provider.

---

# Phase 4 — Resource Server Validation & Scope Enforcement

### Objective

Turn OAuth authentication into strict per-tool authorization.

### Tasks

- [ ] Validate JWT signature using trusted JWKS with bounded caching and refresh-on-unknown-`kid`.
- [ ] Validate issuer, audience/resource, algorithm, expiry, not-before, and token type as required.
- [ ] Bind accepted tokens to the configured MCP resource/server.
- [ ] Extract subject identity and enforce the intended developer/account identity policy.
- [ ] Reject valid-but-wrong-subject tokens for single-user relay deployments.
- [ ] Map OAuth scopes to MCP capabilities.
- [ ] Default deny when a tool requires a missing scope.
- [ ] Require terminal-execution scope before `tools/call` can execute commands.
- [ ] Ensure tool discovery does not leak protected capability details unnecessarily.
- [ ] Ensure authorization runs before execution, filesystem access, or other side effects.
- [ ] Return correct 401/403 semantics.
- [ ] Prevent token reuse across different issuers/resources.
- [ ] Never log bearer/access/refresh tokens.

### Exit criteria

- [ ] Valid token + valid scope + valid subject succeeds.
- [ ] Valid token + wrong scope fails.
- [ ] Valid token + wrong subject fails.
- [ ] Wrong issuer/audience/expired/not-before/unknown-key tokens fail.
- [ ] No MCP execution path can bypass authorization.

---

# Phase 5 — Remote Exposure & Secure Tunnel Integration

### Objective

Make local development usable from ChatGPT without turning the developer machine into a public unauthenticated command server.

### Tasks

- [ ] Document the supported Secure MCP Tunnel workflow for ChatGPT local/developer-machine deployments.
- [ ] Ensure relay itself remains bound to loopback by default.
- [ ] Ensure tunnel/reverse-proxy forwarding preserves the required `Host`, `Origin`, authorization, and MCP headers.
- [ ] Define trusted proxy behavior explicitly; never trust arbitrary forwarded headers.
- [ ] Reject direct public exposure unless remote mode is explicitly configured.
- [ ] Require HTTPS for remote production mode.
- [ ] Add rate limiting for `/mcp` and connection/stream establishment.
- [ ] Add request concurrency limits.
- [ ] Add per-token/per-subject execution rate limits where appropriate.
- [ ] Add idle and maximum request/stream lifetimes.
- [ ] Prevent connection exhaustion and slow-client resource leaks.
- [ ] Ensure error responses do not leak filesystem paths, tokens, command environment, or internal configuration.

### Exit criteria

- [ ] Local relay can be connected through the supported secure tunnel.
- [ ] Direct unauthenticated internet access is not a supported accidental configuration.
- [ ] Rate/concurrency limits prevent trivial MCP DoS.

---

# Phase 6 — ChatGPT Native Integration E2E

### Objective

Prove that ChatGPT can discover, authorize, and use the relay as a real custom MCP app.

### Tasks

- [ ] Create a ChatGPT custom MCP app in developer mode using the actual endpoint and metadata.
- [ ] Verify tool scanning completes successfully.
- [ ] Verify OAuth authorization prompt appears when expected.
- [ ] Verify exact callback URL configuration.
- [ ] Verify refresh-token behavior after access-token expiry/renewal.
- [ ] Verify tool list appears with expected descriptions and schemas.
- [ ] Execute a harmless read command.
- [ ] Execute a file creation/edit operation inside the workspace.
- [ ] Execute a build command.
- [ ] Execute a shell command required by a realistic coding workflow.
- [ ] Verify destructive operations still require the intended ChatGPT confirmation/permission behavior where applicable, while server-side authorization remains authoritative.
- [ ] Verify the relay's Plan 028 sandbox remains enforced during all ChatGPT calls.
- [ ] Verify an unauthorized tool call fails before command execution.
- [ ] Verify wrong-subject token fails.
- [ ] Verify expired/revoked token fails.
- [ ] Verify malformed MCP request cannot reach terminal execution.

### Exit criteria

- [ ] Real ChatGPT conversation can inspect, modify, build, and run code through the relay.
- [ ] No frontend application code change is required to consume the MCP server.
- [ ] All security boundaries remain intact during real remote execution.

---

# Phase 7 — Claude Remote MCP Compatibility

### Objective

Use the same MCP server for Claude integrations without maintaining a second implementation.

### Tasks

- [ ] Connect Claude Code to the canonical HTTP MCP endpoint.
- [ ] Verify `tools/list` and `tools/call`.
- [ ] Verify OAuth authentication and token refresh.
- [ ] Verify legacy SSE compatibility only if an actual target client requires it.
- [ ] If legacy SSE is needed, ensure it delegates to the same MCP/auth/tool execution core.
- [ ] Verify Claude cannot access terminal execution without the same required scope.
- [ ] Verify Claude can complete a realistic coding workflow.
- [ ] Verify no client-specific branch weakens security controls.

Anthropic documents remote HTTP and SSE MCP transports and OAuth support for remote MCP connections, so this phase should validate the shared server rather than build a Claude-specific API. citeturn2search0turn2search1

### Exit criteria

- [ ] ChatGPT and Claude use the same canonical MCP tool implementation.
- [ ] Any compatibility adapter is isolated and security-equivalent.

---

# Phase 8 — Observability, Audit & Operational Safety

### Objective

Make remote MCP execution supportable without leaking secrets or source data.

### Tasks

- [ ] Add structured request IDs/correlation IDs.
- [ ] Log MCP method/tool name, outcome, latency, status, subject ID, and request ID without logging tokens or sensitive arguments.
- [ ] Record command execution audit events without copying secrets or entire source files into logs.
- [ ] Add metrics for request rate, auth failures, authorization failures, tool failures, command duration, timeout count, and sandbox failures.
- [ ] Add bounded log/output sizes.
- [ ] Define retention and redaction behavior.
- [ ] Ensure logs cannot be written into the workspace through an attacker-controlled path.
- [ ] Ensure metrics/logging cannot become a denial-of-service vector.

### Exit criteria

- [ ] An operator can diagnose failed ChatGPT/Claude connections without accessing secrets.
- [ ] Security events are distinguishable from ordinary tool failures.

---

# Phase 9 — CI, Conformance & Release Gate

### Objective

Make the new remote MCP capability impossible to ship while violating the transport/auth/security contract.

### Tasks

- [ ] Keep `cargo fmt --all -- --check` strict.
- [ ] Keep workspace-wide `cargo check --workspace --all-targets --all-features --locked` strict.
- [ ] Keep workspace-wide Clippy with `-D warnings` strict.
- [ ] Keep `cargo audit` mandatory.
- [ ] Add protocol conformance validation for the canonical MCP endpoint.
- [ ] Add static checks that reject accidental legacy-only transport implementations.
- [ ] Add checks for missing Origin validation, missing auth middleware, or tool dispatch paths that bypass authorization.
- [ ] Add checks that no production route exposes debug/no-auth terminal execution.
- [ ] Ensure release jobs depend on the complete quality/security/conformance gate.
- [ ] Ensure release artifacts are built from the exact reviewed commit.
- [ ] Ensure no Node/pkg relay runtime is reintroduced.
- [ ] Ensure deployment documentation matches the released binary behavior.

### Exit criteria

- [ ] CI proves protocol, security, lint, and dependency gates.
- [ ] Release cannot publish an artifact if a required gate fails.

---

# Phase 10 — Final Production Readiness Review

### Objective

Close the plan only after an end-to-end production review, not merely a successful local connection.

### Checklist

- [ ] ChatGPT native MCP connection works.
- [ ] Claude remote MCP connection works.
- [ ] Current MCP `2026-07-28` transport path is the canonical implementation.
- [ ] Legacy SSE, if present, is explicitly compatibility-only and has no weaker security path.
- [ ] OAuth discovery is correct.
- [ ] OAuth authorization and refresh work.
- [ ] Issuer/resource/audience/subject/scope validation is strict.
- [ ] Local Secure MCP Tunnel workflow works without public listener exposure.
- [ ] Remote HTTPS deployment works with trusted proxy configuration.
- [ ] Rate/concurrency/timeout controls work.
- [ ] Plan 028 sandbox remains authoritative for terminal execution.
- [ ] MCP cannot bypass filesystem/privilege/Docker/process controls.
- [ ] Tool schemas match actual authorization and execution behavior.
- [ ] No tokens/secrets appear in logs or tool output.
- [ ] CI is green with zero warnings and no suppression/bypass.
- [ ] Release gate is green.
- [ ] Real coding E2E passes: inspect → edit → install/build → execute → verify.
- [ ] Security-negative E2E passes: invalid auth, wrong scope, wrong subject, expired token, malformed MCP, origin/host abuse, rate-limit abuse, and execution-policy bypass attempts all fail closed.
- [ ] Update `.agents/memories/029-chatgpt-mcp-integration-decisions.md` with final decisions and actual client behavior observed during E2E.
- [ ] Change plan status to `COMPLETED` only after all acceptance criteria are independently verified.

---

## External references / best-practice baseline

- OpenAI — Developer mode and MCP apps in ChatGPT: current remote-MCP requirements, custom app lifecycle, OAuth/refresh-token considerations, write-action behavior, and Secure MCP Tunnel guidance. citeturn3search0
- OpenAI — Apps in ChatGPT: custom MCP apps, app/plugin relationship, workspace administration, and publishing model. citeturn0search0
- OpenAI — ChatGPT app templates: exact callback URL handling and provider configuration guidance. citeturn0search3turn0search4
- MCP — current transport specification: Streamable HTTP, Origin validation, localhost guidance, session behavior, and legacy HTTP+SSE compatibility. citeturn1search0
- MCP — `2026-07-28` release: stateless protocol core, header-based routing, authorization hardening, CIMD direction, DCR deprecation, and legacy HTTP+SSE deprecation. citeturn1search1
- MCP — authorization implementation guidance: Protected Resource Metadata, Authorization Server Metadata, token verification, and resource-specific token validation. citeturn0search6
- Anthropic — Claude Code MCP: remote HTTP/SSE transport and OAuth support. citeturn2search0
- Anthropic — MCP Connector: remote HTTP MCP, OAuth bearer tokens, and compatibility testing with Streamable HTTP/SSE. citeturn2search1

## Definition of Done

Plan 029 is complete only when the same Rust Relay Agent can be securely consumed as a remote MCP server by ChatGPT and Claude, using the current MCP transport as the canonical path, with standards-compliant OAuth discovery/authorization, explicit scope/subject enforcement, the existing Plan 028 sandbox as the authoritative execution boundary, strict CI/release gates, and a successful real-world coding workflow. No client-specific compatibility path may weaken the security model.
