# Plan 029b — external MCP client MCP Production Hardening & Live Acceptance

**Status: IN FLIGHT**

## Why 029b exists

Plan 029 implemented the core external MCP client MCP integration on `feat/029-p0-audit`. A full review found a small set of production-readiness issues that remain unresolved. This follow-up exists **only** for those remaining issues.

This plan is intentionally **not a rewrite or second implementation of Plan 029**. Everything already completed in Plan 028/029 is inherited and must not be reimplemented unless a concrete regression is discovered.

Review baseline:

- Plan 029 implementation branch: `feat/029-p0-audit`
- reviewed head: `a7ee91fea9b4fb02dfc2312cf82429b9fc8d5028`
- parent plan: `.agents/plans/029-external-mcp-native-mcp-integration.md`

---

# Inherited decisions — DO NOT REPEAT

Plan 029b inherits these decisions as fixed:

- MCP `2026-07-28` stateless `POST /mcp` remains canonical.
- No legacy `/sse` + `/message?session_id=...` architecture.
- Relay Agent remains an OAuth Resource Server; external IdP/Authorization Server owns login, Authorization Code, PKCE, refresh tokens, OIDC, and client registration.
- `relay.coding` remains the default coarse full-coding capability for the verified owner.
- Plan 028 bwrap sandbox remains the execution security boundary.
- Normal coding commands must remain usable; do not introduce broad command policing.
- No new Rust/TypeScript unit-test requirement. Verification is static checks + deterministic black-box protocol checks + real external MCP client/OAuth/coding E2E.
- Linux+bwrap-only release contract remains unchanged.
- Existing MCP tool names, JSON schemas, annotations, JWKS cache, issuer/audience validation, tool dispatch, and execution limits are reused.
- Public Plugin Directory publication and interactive Apps SDK UI remain out of scope.

If a task below can be solved by reusing existing Plan 029 code, reuse it instead of creating a parallel path.

---

# Remaining issue matrix

| ID | Severity | Remaining issue | Current evidence |
|---|---|---|---|
| 29B-1 | P1 | Remote mode automatically trusts proxy headers, allowing a direct peer to spoof `X-Forwarded-Proto: https` | `config.rs` sets `trusted_proxy=true` automatically for Remote mode; `transport.rs` trusts the header when that flag is true |
| 29B-2 | P1 | OAuth `WWW-Authenticate` challenges are incomplete | invalid-token challenge exists, but missing-auth lacks resource metadata and insufficient-scope 403 has no standards-compatible challenge |
| 29B-3 | P1 | Docker coding workflow contradicts the Plan 029 product scope | `execution.rs` currently forbids `docker` completely |
| 29B-4 | P1 release | Plan 029 is marked repository-complete while real external MCP client Scan Tools/OAuth/coding acceptance is still unverified | Phase 6 evidence explicitly records live acceptance as unavailable |
| 29B-5 | P2 | Current abuse control is concurrency limiting, not an actual request-rate/admission policy | router concurrency = 64, execution semaphore = 16 |
| 29B-6 | P2 | Protected Resource identifier/metadata routing needs canonical validation | audience/resource is accepted as an arbitrary string; only root well-known route is exposed |
| 29B-7 | P2 | Conformance scripts can false-green because they grep source instead of asserting real HTTP behavior | `phase6`/`phase8` scripts mostly use `rg` checks |
| 29B-8 | P2 | Frozen tool contract snapshot is incomplete | current snapshot stores names/required fields/risk booleans, not the serialized descriptors |
| 29B-9 | P3 | Generated correlation ID is not consistently used by request audit logging | middleware stores generated ID in request extensions while handler reads request headers |
| 29B-10 | P3 | MCP server metadata stamping should be consistent across responses | `serverInfo` is currently discover-centric rather than a reusable response `_meta` helper |

Plan 029b is complete only when these issues are closed or intentionally rejected with a documented rationale.

---

# Phase 1 — Fix the trusted-proxy / HTTPS boundary

## Objective

Remote OAuth tokens must never rely on an attacker-controlled forwarded header to establish transport security.

## Files

- `packages/rust-tools/src/relay_agent/config.rs`
- `packages/rust-tools/src/relay_agent/transport.rs`
- `packages/rust-tools/src/bin/relay-agent.rs`
- deployment/tunnel docs or memory evidence

## Tasks

- [ ] Stop automatically setting `trusted_proxy=true` merely because `SecurityMode::Remote` is selected.
- [ ] Make proxy trust an **explicit operator decision**.
- [ ] Keep the easiest/safest developer path loopback-first: local relay on `127.0.0.1`, then supported Secure MCP Tunnel/private edge exposure.
- [ ] If direct reverse-proxy deployment is supported, require explicit trusted-proxy configuration.
- [ ] Do not trust `X-Forwarded-Proto`, `Forwarded`, `X-Forwarded-Host`, or similar headers from arbitrary peers.
- [ ] Prefer a concrete trusted-peer/CIDR/socket boundary over a global boolean when practical.
- [ ] If only a boolean can be delivered quickly, require the relay bind/interface/network placement to make direct untrusted access impossible and document the assumption explicitly.
- [ ] Direct Remote mode without a trusted HTTPS edge must fail closed.
- [ ] Preserve OAuth/authentication behavior; do not weaken auth to simplify proxy handling.
- [ ] Do not make normal coding calls repeatedly re-authenticate because of proxy handling.

## Black-box acceptance

- [ ] direct plaintext request to Remote relay + spoofed `X-Forwarded-Proto: https` does **not** bypass the transport-security policy.
- [ ] trusted proxy/tunnel request carrying the expected forwarded HTTPS signal succeeds.
- [ ] untrusted forwarded Host/Proto values cannot change effective security decisions.
- [ ] Authorization and MCP routing headers survive the trusted edge.

## Exit criteria

- [ ] no untrusted network client can cause the relay to treat plaintext transport as trusted HTTPS by supplying a header.

---

# Phase 2 — Complete OAuth challenge + Protected Resource semantics

## Objective

Make authentication and insufficient-scope failures recoverable by standards-aware MCP clients including external MCP client.

## Files

- `packages/rust-tools/src/relay_agent/transport.rs`
- `packages/rust-tools/src/relay_agent/config.rs`
- deterministic protocol acceptance script

## Tasks

- [ ] Add one helper for OAuth Bearer challenges so missing-auth, invalid-token, and insufficient-scope paths cannot drift.
- [ ] Missing/invalid authentication returns `401` with a standards-compatible `WWW-Authenticate: Bearer ...` challenge.
- [ ] Include the Protected Resource Metadata reference (`resource_metadata`) where required/supported by the current MCP OAuth profile.
- [ ] Invalid/expired/untrusted tokens advertise `error="invalid_token"` without leaking token contents or validation internals.
- [ ] Authenticated owner/token missing `relay.coding` returns `403` with `error="insufficient_scope"` and required `scope="relay.coding"` metadata where applicable.
- [ ] Wrong owner remains authorization failure before dispatch; document whether its client-facing challenge should expose a scope hint or a generic forbidden result.
- [ ] Do not place `offline_access` in Protected Resource scopes.
- [ ] Validate the configured MCP resource identifier as a canonical HTTPS resource URI for Remote mode: absolute HTTPS, no fragment, stable normalization rules.
- [ ] Preserve exact audience/resource validation against that canonical identifier.
- [ ] Serve the root Protected Resource Metadata route needed by current clients.
- [ ] Also support the path-derived RFC 9728 Protected Resource Metadata route when the configured resource path is `/mcp` (for example `/.well-known/oauth-protected-resource/mcp`) if current client/spec verification confirms it is applicable.
- [ ] Do not implement Authorization Server endpoints inside the relay.

## Black-box acceptance

- [ ] unauthenticated `/mcp` request -> `401` + expected Bearer challenge.
- [ ] malformed/expired token -> `401` + `invalid_token` challenge.
- [ ] valid owner token without `relay.coding` -> `403` + `insufficient_scope` + required scope.
- [ ] Protected Resource metadata exposes the exact resource used by JWT audience/resource validation.
- [ ] metadata contains `relay.coding` and does not contain `offline_access`.

## Exit criteria

- [ ] a client can distinguish "authenticate again" from "request/grant the coding scope" from the actual HTTP response, not source-code inference.

---

# Phase 3 — Restore Docker as a safe coding capability

## Objective

Plan 029 promises a real coding agent. Docker must either work through a genuinely isolated backend or remain explicitly unavailable; raw host Docker control is not acceptable.

## Important invariant

**Do not simply remove `docker` from the denylist and expose `/var/run/docker.sock`.** A raw host Docker daemon/socket would undermine the workspace/host boundary.

Docker security must be enforced by the Docker backend isolation model, not an endlessly growing string denylist.

## Files

- `packages/rust-tools/src/relay_agent/execution.rs`
- `packages/rust-tools/src/relay_agent/config.rs`
- optional dedicated Docker broker/backend module
- deployment documentation
- Plan 029b decision memory

## Tasks

- [ ] Choose and document exactly one supported Docker execution architecture for the first release:
  - isolated remote Docker/BuildKit worker/VM, **or**
  - dedicated restricted broker/proxy in front of a daemon, **or**
  - another architecture that demonstrably prevents host-root control.
- [ ] Do not expose the normal host Docker socket to the MCP sandbox.
- [ ] If a Docker endpoint is configurable, make it explicit and fail closed when absent/malformed.
- [ ] Ensure the Docker endpoint cannot be changed by MCP tool arguments.
- [ ] Ensure credentials/TLS material for a remote Docker backend never appear in tool arguments or logs.
- [ ] Remove the blanket `docker` ban only after the safe backend is wired.
- [ ] Allow normal coding workflows such as image build, container run, logs, inspect, and compose-equivalent workflow where the chosen backend supports them.
- [ ] Prevent host namespace/device/socket/capability escape at the backend boundary.
- [ ] Prevent arbitrary host filesystem mounts outside the intended isolated worker/workspace mapping.
- [ ] Do not turn normal `docker build` / ordinary container execution into a large fragile CLI denylist when the backend is already isolated.
- [ ] If a safe backend cannot be delivered in this iteration, keep Docker disabled and mark 29B-3 OPEN; do not claim Plan 029b complete.

## Coding acceptance

- [ ] build an image from the workspace.
- [ ] run the image.
- [ ] read logs/result.
- [ ] perform a realistic project build using Docker.
- [ ] normal Docker workflow does not require privilege elevation.

## Security-negative acceptance

- [ ] raw host Docker socket is inaccessible.
- [ ] host-root bind mount is impossible.
- [ ] privileged container / host PID/network/device/capability escape is impossible at the chosen backend boundary.
- [ ] Docker cannot access filesystem outside the backend's intended workspace mapping.

## Exit criteria

- [ ] Docker is usable for coding **and** cannot become a host-control bypass.

---

# Phase 4 — Replace source-grep false-greens with black-box conformance

## Objective

Keep the no-unit-test decision while making repository gates prove actual behavior instead of merely proving strings exist in source files.

## Files

- `scripts/phase6-external-mcp-e2e.sh`
- `scripts/phase8-zero-bypass.sh`
- new focused black-box script if cleaner
- `.github/workflows/ci.yml`

## Tasks

- [ ] Preserve static grep checks only for invariants that are genuinely source-structural.
- [ ] Add a deterministic local black-box harness that starts `relay-agent` with temporary configuration/keys or a controlled fixture IdP/JWKS endpoint **without adding a unit-test suite**.
- [ ] Assert actual HTTP status, headers, and JSON bodies for MCP protocol failures.
- [ ] Assert actual `WWW-Authenticate` behavior from Phase 2.
- [ ] Assert Local mode Origin/Host fail-closed behavior.
- [ ] Assert Remote mode cannot be made HTTPS-trusted by spoofed proxy headers.
- [ ] Assert missing `relay.coding` fails before tool execution.
- [ ] Assert tool schema rejection happens before dispatch.
- [ ] Keep Plan 028 boundary E2E checks separate from ordinary command denylists.
- [ ] CI must execute these scripts, not only keep them in the repository.
- [ ] Preserve `cargo fmt`, strict `cargo check`, strict Clippy and `cargo audit`.
- [ ] Preserve no `continue-on-error`, `|| true`, swallowed exit, or broad lint suppression.

## Exit criteria

- [ ] a regression in OAuth challenge/proxy/auth behavior makes CI fail even if the expected strings still exist somewhere in source.

---

# Phase 5 — Add real abuse admission control without hurting builds

## Objective

Protect the public MCP edge from request floods while preserving long-running, bursty coding workflows.

## Tasks

- [ ] Keep existing global request concurrency and execution semaphore limits.
- [ ] Add or explicitly delegate a coarse request-rate/admission policy at the trusted edge or relay.
- [ ] Prefer generous burst/token-bucket semantics suitable for agent tool bursts over tiny per-second limits.
- [ ] Do not count a long-running build as repeated requests merely because it runs for minutes.
- [ ] If the trusted tunnel/proxy already provides the chosen rate limit, document and verify that boundary instead of duplicating it in Rust.
- [ ] Return a clear overload/rate-limit response without leaking internal capacity details.
- [ ] Ensure unauthenticated floods cannot consume all execution permits.

## Acceptance

- [ ] normal external MCP client coding burst succeeds.
- [ ] realistic build remains unaffected by request-rate policy.
- [ ] sustained request flood is throttled/rejected.
- [ ] execution concurrency remains bounded.

## Exit criteria

- [ ] Plan 029b can truthfully distinguish concurrency limiting from request-rate/admission limiting.

---

# Phase 6 — Make the published tool snapshot authoritative

## Objective

Prevent a manually maintained mini-contract from drifting away from the actual `tools/list` descriptors external MCP client sees.

## Files

- `.agents/contracts/029-tool-catalog-v1.json`
- `scripts/phase7-external-mcp-contract.sh`
- `packages/rust-tools/src/relay_agent/mcp.rs`

## Tasks

- [ ] Generate or capture the canonical snapshot from the **actual serialized tool descriptors** used by `tools/list`.
- [ ] Snapshot at least: name, title, description, complete input schema, annotations, and any tool-level security/scope metadata actually exposed on the wire.
- [ ] Do not maintain a second manually abbreviated representation as the release authority.
- [ ] Hash canonicalized serialized descriptors.
- [ ] Make CI fail on descriptor changes until the snapshot is deliberately reviewed/updated.
- [ ] Keep additive changes reviewable; do not auto-approve scope/risk/write-semantic changes.
- [ ] Preserve the external MCP client Refresh/recreate guidance from Plan 029 without duplicating that documentation here.

## Exit criteria

- [ ] the contract gate detects a real breaking descriptor/schema change, not just tool renames.

---

# Phase 7 — Fix correlation/audit consistency and response metadata polish

## Objective

Close small observability/conformance gaps without redesigning logging.

## Tasks

- [ ] Store/read the generated correlation ID from one canonical request extension/helper.
- [ ] If the client sends an accepted correlation ID, reuse it; otherwise use the generated UUID.
- [ ] Ensure audit log and response header carry the same correlation ID.
- [ ] Keep subject privacy-preserving and never log raw token/command/source arguments by default.
- [ ] Add one response helper for standard MCP server `_meta` stamping if the current MCP `2026-07-28` client/spec behavior recommends `io.modelcontextprotocol/serverInfo` on all server responses.
- [ ] Do not duplicate per-handler serialization logic unnecessarily.

## Acceptance

- [ ] request without correlation header -> response and audit share generated UUID.
- [ ] request with valid correlation header -> response and audit share supplied ID.
- [ ] no raw bearer token, command arguments, or source text appears in audit output.

## Exit criteria

- [ ] logs are actually correlatable end-to-end.

---

# Phase 8 — Real external MCP client live acceptance

## Objective

Close the one part repository/static validation cannot prove: the current external MCP client product actually accepts and uses the connector.

This phase is intentionally narrow and references Plan 029 for the broader product rationale. Do not repeat completed implementation work.

## Required live evidence

- [ ] deploy/expose the reviewed relay through the approved trusted HTTPS/tunnel path.
- [ ] create a external MCP client custom MCP app in a plan/mode that supports the intended write-capable coding workflow.
- [ ] `Scan Tools` succeeds.
- [ ] exactly the intended tool descriptors appear.
- [ ] Advanced OAuth discovery shows the expected resource and Authorization Server metadata.
- [ ] User-defined OAuth client flow succeeds.
- [ ] exact external MCP client callback URL is registered and works.
- [ ] PKCE/authorization succeeds through the external IdP.
- [ ] refresh-token/access-token renewal works without manual reconnect.
- [ ] verified owner receives `relay.coding`.
- [ ] real coding workflow succeeds: inspect -> edit -> dependency/build/run -> verify.
- [ ] shell/interpreter/Git/package-manager functionality remains usable.
- [ ] Docker coding workflow succeeds if Phase 3 is delivered.
- [ ] invalid token, wrong owner, wrong resource/audience, and missing coding scope fail before execution.
- [ ] workspace/privilege/sandbox escape attempts remain blocked.

## Evidence

Record redacted live evidence in:

- `.agents/memories/029b-external-mcp-live-acceptance.md`

Do not store access tokens, refresh tokens, client secrets, private source contents, or OAuth credentials.

## Exit criteria

- [ ] current external MCP client can actually use the connector as the intended coding agent.

---

# Final completion gate

Plan 029b is `COMPLETED` only when:

- [ ] 29B-1 trusted-proxy/HTTPS spoofing gap is closed.
- [ ] 29B-2 OAuth challenges are black-box verified.
- [ ] 29B-3 Docker is safely usable, or the project explicitly changes the product requirement before completion.
- [ ] 29B-4 live external MCP client acceptance is complete.
- [ ] 29B-5 actual abuse admission/rate policy is verified or explicitly delegated to the trusted edge.
- [ ] 29B-6 resource identifier/metadata behavior is canonical and verified.
- [ ] 29B-7 CI uses real black-box protocol assertions for the relevant security contract.
- [ ] 29B-8 tool snapshot derives from real serialized descriptors.
- [ ] 29B-9 correlation IDs are consistent across audit and response.
- [ ] 29B-10 response metadata behavior is resolved against current MCP guidance.
- [ ] strict fmt/check/clippy/audit/conformance CI passes on the exact reviewed commit.
- [ ] no unit-test requirement has been introduced.
- [ ] no broad coding-command denylist has been added as a substitute for sandbox/backend isolation.

## Definition of Done

The Plan 029 implementation keeps its existing MCP/OAuth/coding architecture, while the remaining production gaps are closed: trusted HTTPS/proxy handling cannot be spoofed, OAuth challenges are interoperable, Docker coding uses an isolated backend rather than host control, conformance gates prove real behavior, the published tool snapshot matches the real wire contract, operational correlation works, abuse admission is explicit, and a live external MCP client session successfully completes the intended coding workflow.
