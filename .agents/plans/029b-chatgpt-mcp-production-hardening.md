# Plan 029b — ChatGPT MCP Production Hardening & Live Acceptance

**Status: IN FLIGHT**

> [!IMPORTANT]
> ## BRANCH LOCK — DO NOT LEAVE PLAN 029 BRANCH
>
> **All Plan 029b planning, implementation, fixes, scripts, memories, and evidence MUST stay on the existing `feat/029-p0-audit` branch.**
>
> - Do **not** create a new branch for 029b.
> - Do **not** commit Plan 029b work to `dev`, `main`, or any other branch.
> - Do **not** rebase/retarget unrelated branches as part of this plan.
> - Do **not** touch unrelated application/product areas just because they are nearby.
> - Only modify files required to close the explicit unresolved issues listed below.
> - If an issue requires a change outside the Plan 028/029 relay/MCP/security/CI surface, document the dependency first instead of expanding scope silently.
>
> Plan 029b is a continuation/hardening pass on the exact Plan 029 implementation branch, not a separate feature stream.

---

## Why 029b exists

Plan 029 implemented the core ChatGPT MCP integration on `feat/029-p0-audit`. A full review found a small set of production-readiness issues that remain unresolved. This follow-up exists **only** for those remaining issues.

This plan is intentionally **not a rewrite or second implementation of Plan 029**. Everything already completed in Plan 028/029 is inherited and must not be reimplemented unless a concrete regression is discovered.

Review baseline:

- working branch: `feat/029-p0-audit`
- reviewed Plan 029 head: `a7ee91fea9b4fb02dfc2312cf82429b9fc8d5028`
- parent plan: `.agents/plans/029-chatgpt-native-mcp-integration.md`

---

# Inherited decisions — DO NOT REPEAT

Plan 029b inherits these decisions as fixed:

- MCP `2026-07-28` stateless `POST /mcp` remains canonical.
- No legacy `/sse` + `/message?session_id=...` architecture.
- Relay Agent remains an OAuth Resource Server; external IdP/Authorization Server owns login, Authorization Code, PKCE, refresh tokens, OIDC, and client registration.
- `relay.coding` remains the default coarse full-coding capability for the verified owner.
- Plan 028 bwrap sandbox remains the execution security boundary.
- Normal coding commands must remain usable; do not introduce broad command policing.
- No new Rust/TypeScript unit-test requirement. Verification is static checks + deterministic black-box protocol checks + real ChatGPT/OAuth/coding E2E.
- Linux+bwrap-only release contract remains unchanged.
- Existing MCP tool names, schemas, annotations, JWKS cache, issuer/audience validation, tool dispatch, and execution limits are reused.
- Public Plugin Directory publication and interactive Apps SDK UI remain out of scope.

If a task below can be solved by reusing existing Plan 029 code, reuse it instead of creating a parallel path.

---

# Remaining issue matrix

| ID | Severity | Remaining issue |
|---|---|---|
| 29B-1 | P1 | Remote mode automatically trusts proxy headers, allowing direct peers to spoof HTTPS via forwarded headers |
| 29B-2 | P1 | OAuth `WWW-Authenticate` challenge semantics are incomplete |
| 29B-3 | P1 | Docker coding workflow is promised by Plan 029 but currently blocked completely |
| 29B-4 | P1 release | Real ChatGPT Scan Tools/OAuth/coding acceptance remains unverified |
| 29B-5 | P2 | Current abuse protection is concurrency limiting, not actual request-rate/admission control |
| 29B-6 | P2 | Protected Resource URI/metadata routing needs canonical validation |
| 29B-7 | P2 | Conformance scripts can false-green because they grep source instead of asserting HTTP behavior |
| 29B-8 | P2 | Frozen tool contract snapshot is incomplete and manually abbreviated |
| 29B-9 | P3 | Generated correlation IDs are not consistently used by audit logs |
| 29B-10 | P3 | MCP server metadata stamping should be consistent across responses |

029b is complete only when every item above is closed or explicitly rejected with documented rationale.

---

# Phase 1 — Trusted proxy / HTTPS boundary

## Objective

Remote OAuth tokens must never rely on an attacker-controlled forwarded header to establish transport security.

## Files allowed

- `packages/rust-tools/src/relay_agent/config.rs`
- `packages/rust-tools/src/relay_agent/transport.rs`
- `packages/rust-tools/src/bin/relay-agent.rs`
- Plan 029b evidence/memory files

## Tasks

- [ ] Stop automatically setting `trusted_proxy=true` just because `SecurityMode::Remote` is selected.
- [ ] Make proxy trust an explicit operator choice.
- [ ] Keep the easiest developer path loopback-first: relay on loopback plus supported secure tunnel/private edge.
- [ ] If reverse-proxy deployment is supported, require explicit trusted-proxy configuration.
- [ ] Never trust `X-Forwarded-Proto`, `Forwarded`, `X-Forwarded-Host`, or similar headers from arbitrary peers.
- [ ] Prefer concrete trusted peer/CIDR/socket identity over a global boolean where practical.
- [ ] If a boolean is kept for speed, document and enforce the network placement assumption that prevents direct untrusted access.
- [ ] Direct Remote mode without a trusted HTTPS edge fails closed.
- [ ] Do not weaken OAuth/auth to simplify proxy handling.

## Black-box acceptance

- [ ] direct plaintext request + spoofed `X-Forwarded-Proto: https` does not pass transport trust.
- [ ] actual trusted edge/tunnel request succeeds.
- [ ] untrusted forwarded host/proto values cannot alter security decisions.
- [ ] Authorization and MCP headers survive the approved edge.

---

# Phase 2 — OAuth challenge + Protected Resource semantics

## Objective

Make authentication failures and insufficient-scope failures recoverable by standards-aware MCP clients including ChatGPT.

## Files allowed

- `packages/rust-tools/src/relay_agent/transport.rs`
- `packages/rust-tools/src/relay_agent/config.rs`
- deterministic conformance script(s)
- Plan 029b evidence/memory files

## Tasks

- [ ] Add one reusable Bearer challenge helper so auth paths do not drift.
- [ ] Missing authentication -> HTTP `401` with standards-compatible `WWW-Authenticate`.
- [ ] Invalid/expired token -> HTTP `401` + `error="invalid_token"`.
- [ ] Include Protected Resource Metadata reference (`resource_metadata`) where applicable to current MCP OAuth guidance.
- [ ] Valid authenticated request without `relay.coding` -> HTTP `403` + `error="insufficient_scope"` + required `scope="relay.coding"` where applicable.
- [ ] Wrong owner stays an authorization failure before dispatch and does not leak claim internals.
- [ ] Keep `offline_access` out of Protected Resource scopes.
- [ ] Validate configured resource identifier for Remote mode as canonical absolute HTTPS URI with no fragment.
- [ ] Keep resource identifier identical to audience/resource validation policy.
- [ ] Preserve root Protected Resource Metadata route.
- [ ] Add path-derived RFC 9728 route for `/mcp` only if current client/spec verification says it is applicable.
- [ ] Do not implement Authorization Server endpoints inside the relay.

## Black-box acceptance

- [ ] unauthenticated `/mcp` -> expected `401` challenge.
- [ ] invalid/expired token -> `401 invalid_token`.
- [ ] valid owner without `relay.coding` -> `403 insufficient_scope`.
- [ ] metadata resource exactly matches enforced audience/resource.
- [ ] metadata includes `relay.coding` and excludes `offline_access`.

---

# Phase 3 — Restore Docker as a safe coding capability

## Objective

Docker should work for normal coding only through a backend that cannot become host control.

## Critical invariant

**Do not simply remove `docker` from the denylist and expose `/var/run/docker.sock`.** Raw host Docker control defeats the intended host/workspace boundary.

## Files allowed

- `packages/rust-tools/src/relay_agent/execution.rs`
- `packages/rust-tools/src/relay_agent/config.rs`
- optional dedicated Docker backend/broker module under `packages/rust-tools/src/relay_agent/`
- Plan 029b deployment/evidence memory

## Tasks

- [ ] Choose exactly one supported Docker architecture for first release:
  - isolated remote Docker/BuildKit worker/VM, or
  - restricted broker/proxy in front of a daemon, or
  - equivalent backend with demonstrated host isolation.
- [ ] Do not expose host Docker socket to the MCP sandbox.
- [ ] Docker endpoint/config is operator-owned, never provided by tool arguments.
- [ ] Docker credentials/TLS material never enters tool args or logs.
- [ ] Only remove blanket `docker` prohibition after safe backend wiring exists.
- [ ] Allow ordinary image build/run/logs/inspect workflow.
- [ ] Support compose-equivalent workflow only if the chosen backend safely supports it.
- [ ] Prevent host PID/network/device/capability/namespace escape at backend boundary.
- [ ] Prevent arbitrary host filesystem bind mounts outside isolated workspace mapping.
- [ ] Do not replace backend isolation with a giant fragile Docker CLI denylist.
- [ ] If safe Docker backend cannot be delivered, keep Docker disabled and leave 29B-3 OPEN.

## Acceptance

- [ ] build image from workspace.
- [ ] run image.
- [ ] inspect logs/result.
- [ ] realistic Docker project build succeeds.
- [ ] host Docker socket inaccessible.
- [ ] host-root bind mount impossible.
- [ ] privileged/host namespace/device escape impossible under chosen backend.

---

# Phase 4 — Black-box conformance instead of source-grep false-greens

## Objective

Keep the no-unit-test decision while proving actual protocol/security behavior.

## Files allowed

- `scripts/phase6-chatgpt-e2e.sh`
- `scripts/phase8-zero-bypass.sh`
- one new focused black-box script if cleaner
- `.github/workflows/ci.yml`
- minimal fixture material strictly needed by the script

## Tasks

- [ ] Keep grep only for genuinely structural invariants.
- [ ] Add deterministic local black-box harness that starts relay with controlled configuration/JWKS fixture or local fixture IdP endpoint.
- [ ] Assert actual HTTP status, headers, and JSON responses.
- [ ] Assert OAuth challenge semantics from Phase 2.
- [ ] Assert Local Origin/Host fail closed behavior.
- [ ] Assert spoofed forwarded HTTPS cannot bypass Phase 1.
- [ ] Assert missing `relay.coding` fails before tool execution.
- [ ] Assert bad tool schema/arguments fail before dispatch.
- [ ] CI must actually execute the black-box script.
- [ ] Preserve `cargo fmt`, strict `cargo check`, strict Clippy, `cargo audit`.
- [ ] Preserve no `continue-on-error`, `|| true`, swallowed exits, broad lint suppression.

## Exit criteria

- [ ] auth/proxy/protocol regression fails CI even if expected strings still exist in source.

---

# Phase 5 — Real abuse admission control

## Objective

Protect remote MCP edge from floods without throttling ordinary coding bursts/builds.

## Tasks

- [ ] Keep existing request concurrency and execution semaphore limits.
- [ ] Add or explicitly delegate coarse request-rate/admission control at trusted edge or relay.
- [ ] Prefer generous burst/token-bucket semantics suitable for agent bursts.
- [ ] Do not treat a multi-minute build as repeated traffic merely because it stays active.
- [ ] If trusted edge already provides rate limiting, document and verify it rather than duplicate Rust logic.
- [ ] Return clear overload/rate-limit response without exposing internal capacity.
- [ ] Prevent unauthenticated floods from consuming execution capacity.

## Acceptance

- [ ] ordinary ChatGPT coding burst succeeds.
- [ ] normal long build unaffected.
- [ ] sustained request flood is throttled/rejected.
- [ ] execution concurrency stays bounded.

---

# Phase 6 — Authoritative published tool snapshot

## Objective

Freeze what ChatGPT actually sees, not a manually abbreviated parallel contract.

## Files allowed

- `.agents/contracts/029-tool-catalog-v1.json`
- `scripts/phase7-chatgpt-contract.sh`
- `packages/rust-tools/src/relay_agent/mcp.rs`
- Plan 029b evidence/memory files

## Tasks

- [ ] Generate/capture canonical snapshot from actual serialized `tools/list` descriptors.
- [ ] Snapshot name, title, description, complete input schema, annotations, and any on-wire security/scope metadata.
- [ ] Remove the manually abbreviated representation as the authoritative source.
- [ ] Hash canonicalized real descriptors.
- [ ] CI fails on descriptor changes until deliberately reviewed/updated.
- [ ] Scope/risk/write-semantics changes require explicit review.
- [ ] Reuse Plan 029 publication/Refresh guidance instead of duplicating it.

## Exit criteria

- [ ] actual descriptor/schema changes are detected by the contract gate.

---

# Phase 7 — Correlation logging + MCP response metadata polish

## Objective

Close small observability/conformance gaps without redesigning logging.

## Tasks

- [ ] Store/read correlation ID from one canonical request extension/helper.
- [ ] Valid client-provided correlation ID is reused.
- [ ] Otherwise generated UUID is used.
- [ ] Audit log and response header always share the same correlation ID.
- [ ] Keep subject privacy-preserving; never log raw token/command/source args by default.
- [ ] Add reusable response `_meta` helper for `io.modelcontextprotocol/serverInfo` if current MCP `2026-07-28` spec/client guidance recommends it across responses.
- [ ] Avoid duplicate per-handler serialization logic.

## Acceptance

- [ ] no client correlation header -> generated ID matches audit + response.
- [ ] supplied valid ID -> same ID in audit + response.
- [ ] no raw bearer token/command/source content in audit output.

---

# Phase 8 — Real ChatGPT live acceptance

## Objective

Close the remaining integration evidence that repository/static checks cannot prove.

## Required live evidence

- [ ] expose reviewed relay through approved trusted HTTPS/tunnel path.
- [ ] create ChatGPT custom MCP app in a plan/mode supporting intended write-capable coding workflow.
- [ ] `Scan Tools` succeeds.
- [ ] intended descriptors appear exactly as expected.
- [ ] Advanced OAuth discovery shows correct resource and Authorization Server metadata.
- [ ] User-defined OAuth client flow succeeds.
- [ ] exact ChatGPT callback URL works.
- [ ] PKCE flow succeeds through external IdP.
- [ ] access-token refresh works without manual reconnect.
- [ ] verified owner receives `relay.coding`.
- [ ] real workflow: inspect -> edit -> install/build/run -> verify.
- [ ] shell/interpreter/Git/package manager workflows remain usable.
- [ ] Docker coding workflow succeeds if Phase 3 is completed.
- [ ] invalid token rejected.
- [ ] wrong owner rejected.
- [ ] wrong audience/resource rejected.
- [ ] missing `relay.coding` rejected.
- [ ] workspace/symlink/privilege/Docker-host boundary checks remain blocked.
- [ ] ChatGPT confirmation UX cannot override server denial.

## Evidence

- [ ] record redacted screenshots/logs/metadata in a Plan 029b memory file.
- [ ] do not record secrets, access tokens, refresh tokens, client secrets, or sensitive source content.

---

# Phase 9 — Closeout

## Required gates

- [ ] 29B-1 closed.
- [ ] 29B-2 closed.
- [ ] 29B-3 closed or explicitly remains unsupported and therefore plan stays open.
- [ ] 29B-4 live acceptance closed.
- [ ] 29B-5 closed.
- [ ] 29B-6 closed.
- [ ] 29B-7 closed.
- [ ] 29B-8 closed.
- [ ] 29B-9 closed.
- [ ] 29B-10 closed or documented as unnecessary after current-spec verification.
- [ ] no Plan 028/029 subsystem was reimplemented unnecessarily.
- [ ] no broad coding denylist was introduced.
- [ ] no unit-test requirement was added.
- [ ] strict format/check/clippy/audit gates pass.
- [ ] black-box connector/security conformance passes.
- [ ] real ChatGPT acceptance passes.
- [ ] all commits remain on `feat/029-p0-audit` until the existing Plan 029 branch is intentionally merged through the normal repository workflow.

---

## Definition of Done

Plan 029b closes only the unresolved production gaps left by Plan 029 while preserving its existing architecture and coding-agent ergonomics. The relay has a trustworthy HTTPS/proxy boundary, standards-compatible OAuth challenges and Protected Resource metadata, safe Docker coding capability, real flood admission control, behavior-based conformance gates, authoritative tool descriptor snapshots, consistent observability metadata, and verified live ChatGPT Scan Tools/OAuth/coding acceptance.

**Branch invariant:** every 029b change is implemented and documented on `feat/029-p0-audit`; this plan does not create or use another implementation branch.