# Plan 029b — external MCP client MCP Production Hardening & Live Acceptance

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

Plan 029 implemented the core external MCP client MCP integration on `feat/029-p0-audit`. A full review found a small set of production-readiness issues that remain unresolved. This follow-up exists **only** for those remaining issues.

This plan is intentionally **not a rewrite or second implementation of Plan 029**. Everything already completed in Plan 028/029 is inherited and must not be reimplemented unless a concrete regression is discovered.

Review baseline:

- working branch: `feat/029-p0-audit`
- reviewed Plan 029 head: `a7ee91fea9b4fb02dfc2312cf82429b9fc8d5028`
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
| 29B-4 | P1 release | Real external MCP client Scan Tools/OAuth/coding acceptance remains unverified |
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

- [x] Stop automatically setting `trusted_proxy=true` just because `SecurityMode::Remote` is selected.
- [x] Make proxy trust an explicit operator choice.
- [x] Keep the easiest developer path loopback-first: relay on loopback plus supported secure tunnel/private edge.
- [x] If reverse-proxy deployment is supported, require explicit trusted-proxy configuration.
- [x] Never trust `X-Forwarded-Proto`, `Forwarded`, `X-Forwarded-Host`, or similar headers from arbitrary peers.
- [x] Prefer concrete trusted peer/CIDR/socket identity over a global boolean where practical.
- [x] If a boolean is kept for speed, document and enforce the network placement assumption that prevents direct untrusted access.
- [x] Direct Remote mode without a trusted HTTPS edge fails closed.
- [x] Do not weaken OAuth/auth to simplify proxy handling.

## Black-box acceptance

- [x] direct plaintext request + spoofed `X-Forwarded-Proto: https` does not pass transport trust.
- [x] actual trusted edge/tunnel request succeeds.
- [x] untrusted forwarded host/proto values cannot alter security decisions.
- [x] Authorization and MCP headers survive the approved edge.

---

# Phase 2 — OAuth challenge + Protected Resource semantics

## Objective

Make authentication failures and insufficient-scope failures recoverable by standards-aware MCP clients including external MCP client.

## Files allowed

- `packages/rust-tools/src/relay_agent/transport.rs`
- `packages/rust-tools/src/relay_agent/config.rs`
- deterministic conformance script(s)
- Plan 029b evidence/memory files

## Tasks

- [x] Add one reusable Bearer challenge helper so auth paths do not drift.
- [x] Missing authentication -> HTTP `401` with standards-compatible `WWW-Authenticate`.
- [x] Invalid/expired token -> HTTP `401` + `error="invalid_token"`.
- [x] Include Protected Resource Metadata reference (`resource_metadata`) where applicable to current MCP OAuth guidance.
- [x] Valid authenticated request without `relay.coding` -> HTTP `403` + `error="insufficient_scope"` + required `scope="relay.coding"` where applicable.
- [x] Wrong owner stays an authorization failure before dispatch and does not leak claim internals.
- [x] Keep `offline_access` out of Protected Resource scopes.
- [x] Validate configured resource identifier for Remote mode as canonical absolute HTTPS URI with no fragment.
- [x] Keep resource identifier identical to audience/resource validation policy.
- [x] Preserve root Protected Resource Metadata route.
- [x] Add path-derived RFC 9728 route for `/mcp` only if current client/spec verification says it is applicable.
- [x] Do not implement Authorization Server endpoints inside the relay.

## Black-box acceptance

- [x] unauthenticated `/mcp` -> expected `401` challenge.
- [x] invalid/expired token -> `401 invalid_token`.
- [x] valid owner without `relay.coding` -> `403 insufficient_scope`.
- [x] metadata resource exactly matches enforced audience/resource.
- [x] metadata includes `relay.coding` and excludes `offline_access`.

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
- [x] If safe Docker backend cannot be delivered, keep Docker disabled and leave 29B-3 OPEN.

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

- `scripts/phase6-external-mcp-e2e.sh`
- `scripts/phase8-zero-bypass.sh`
- one new focused black-box script if cleaner
- `.github/workflows/ci.yml`
- minimal fixture material strictly needed by the script

## Tasks

- [x] Keep grep only for genuinely structural invariants.
- [x] Add deterministic local black-box harness that starts relay with controlled configuration/JWKS fixture or local fixture IdP endpoint.
- [x] Assert actual HTTP status, headers, and JSON responses.
- [x] Assert OAuth challenge semantics from Phase 2.
- [x] Assert Local Origin/Host fail closed behavior.
- [x] Assert spoofed forwarded HTTPS cannot bypass Phase 1.
- [x] Assert missing `relay.coding` fails before tool execution.
- [x] Assert bad tool schema/arguments fail before dispatch.
- [x] CI must actually execute the black-box script.
- [x] Preserve `cargo fmt`, strict `cargo check`, strict Clippy, `cargo audit`.
- [x] Preserve no `continue-on-error`, `|| true`, swallowed exits, broad lint suppression.

## Exit criteria

- [x] auth/proxy/protocol regression fails CI even if expected strings still exist in source.

---

# Phase 5 — Real abuse admission control

## Objective

Protect remote MCP edge from floods without throttling ordinary coding bursts/builds.

## Tasks

- [x] Keep existing request concurrency and execution semaphore limits.
- [x] Add or explicitly delegate coarse request-rate/admission control at trusted edge or relay.
- [x] Prefer generous burst/token-bucket semantics suitable for agent bursts.
- [x] Do not treat a multi-minute build as repeated traffic merely because it stays active.
- [x] If trusted edge already provides rate limiting, document and verify it rather than duplicate Rust logic.
- [x] Return clear overload/rate-limit response without exposing internal capacity.
- [x] Prevent unauthenticated floods from consuming execution capacity.

## Acceptance

- [x] ordinary external MCP client coding burst succeeds.
- [x] normal long build unaffected.
- [x] sustained request flood is throttled/rejected.
- [x] execution concurrency stays bounded.

---

# Phase 6 — Authoritative published tool snapshot

## Objective

Freeze what external MCP client actually sees, not a manually abbreviated parallel contract.

## Files allowed

- `.agents/contracts/029-tool-catalog-v1.json`
- `scripts/phase7-external-mcp-contract.sh`
- `packages/rust-tools/src/relay_agent/mcp.rs`
- Plan 029b evidence/memory files

## Tasks

- [x] Generate/capture canonical snapshot from actual serialized `tools/list` descriptors.
- [x] Snapshot name, title, description, complete input schema, annotations, and any on-wire security/scope metadata.
- [x] Remove the manually abbreviated representation as the authoritative source.
- [x] Hash canonicalized real descriptors.
- [x] CI fails on descriptor changes until deliberately reviewed/updated.
- [x] Scope/risk/write-semantics changes require explicit review.
- [x] Reuse Plan 029 publication/Refresh guidance instead of duplicating it.

## Exit criteria

- [x] actual descriptor/schema changes are detected by the contract gate.

---

# Phase 7 — Correlation logging + MCP response metadata polish

## Objective

Close small observability/conformance gaps without redesigning logging.

## Tasks

- [x] Store/read correlation ID from one canonical request extension/helper.
- [x] Valid client-provided correlation ID is reused.
- [x] Otherwise generated UUID is used.
- [x] Audit log and response header always share the same correlation ID.
- [x] Keep subject privacy-preserving; never log raw token/command/source args by default.
- [x] Add reusable response `_meta` helper for `io.modelcontextprotocol/serverInfo` if current MCP `2026-07-28` spec/client guidance recommends it across responses.
- [x] Avoid duplicate per-handler serialization logic.

## Acceptance

- [x] no client correlation header -> generated ID matches audit + response.
- [x] supplied valid ID -> same ID in audit + response.
- [x] no raw bearer token/command/source content in audit output.

---

# Phase 8 — Real external MCP client live acceptance

## Objective

Close the remaining integration evidence that repository/static checks cannot prove.

## Required live evidence

- [ ] expose reviewed relay through approved trusted HTTPS/tunnel path.
- [ ] create external MCP client custom MCP app in a plan/mode supporting intended write-capable coding workflow.
- [ ] `Scan Tools` succeeds.
- [ ] intended descriptors appear exactly as expected.
- [ ] Advanced OAuth discovery shows correct resource and Authorization Server metadata.
- [ ] User-defined OAuth client flow succeeds.
- [ ] exact external MCP client callback URL works.
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
- [ ] external MCP client confirmation UX cannot override server denial.

## Evidence

- [ ] record redacted screenshots/logs/metadata in a Plan 029b memory file.
- [ ] do not record secrets, access tokens, refresh tokens, client secrets, or sensitive source content.

---

# Phase 9 — Closeout

## Required gates

- [x] 29B-1 closed.
- [x] 29B-2 closed.
- [x] 29B-3 is an accepted v1 limitation: Docker remains disabled until an isolated backend exists; do not expose the host Docker socket or remove the ban without that backend.
- [ ] 29B-4 live acceptance closed.
- [x] 29B-5 closed.
- [x] 29B-6 closed.
- [x] 29B-7 closed.
- [x] 29B-8 closed: CI compares canonicalized runtime `tools/list.result.tools` descriptors with the reviewed frozen snapshot.
- [x] 29B-9 closed.
- [x] 29B-10 closed via the centralized response metadata helper.
- [x] no Plan 028/029 subsystem was reimplemented unnecessarily.
- [x] no broad coding denylist was introduced beyond the explicitly deferred Docker capability and existing privilege-boundary restrictions.
- [x] no unit-test **requirement or CI gate** was added.
- [x] strict format/check/clippy/audit gates pass after switching `jsonwebtoken` to its `aws_lc_rs` provider; no advisory ignore is used.
- [x] black-box connector/security conformance passes locally.
- [ ] real external MCP client acceptance passes (still unavailable without the approved external deployment and external MCP client/OAuth environment).
- [x] all commits remain on `feat/029-p0-audit` until the existing Plan 029 branch is intentionally merged through the normal repository workflow.

## Phase 9 closeout evidence — 2026-08-12

Available repository gates passed:

- `cargo fmt --all -- --check`
- `RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `pnpm run lint`, `pnpm run typecheck`, and `pnpm audit`
- `scripts/phase4-black-box.sh`
- `scripts/phase7-external-mcp-contract.sh`
- `scripts/phase8-zero-bypass.sh`
- `scripts/phase6-external-mcp-e2e.sh` static checks
- `scripts/phase7-external-mcp-contract.sh` runtime `tools/list` extraction and canonical comparison
- `cargo audit` with the `aws_lc_rs` jsonwebtoken provider

Open gates and blockers:

- Phase 8 live external MCP client acceptance is unavailable in this repository run: no approved HTTPS/tunnel deployment, external MCP client workspace/app, OAuth tenant/client, callback verification, or redacted live evidence is available. The Phase 6 harness reports this as unavailable; it is not treated as a pass.
- Docker is intentionally deferred for v1. No isolated Docker worker, restricted broker, or equivalent backend is available, so Docker remains disabled to preserve the host boundary. This is an accepted capability limitation, not permission to weaken the sandbox. See `.agents/memories/029b-docker-capability-blocker.md`.
- The prior `RUSTSEC-2023-0071` path was removed by switching `jsonwebtoken` from `rust_crypto` to `aws_lc_rs` and regenerating `Cargo.lock` normally; `cargo audit` passes without an advisory waiver.
- GitHub CI now explicitly installs Linux `bubblewrap` before the Phase 4 and Phase 7 runtime gates.
- Phase 7 now starts the local relay, captures actual `tools/list.result.tools`, canonicalizes it with deterministic key ordering, and compares it to the descriptor-only frozen snapshot.

Phase 9 does not close Plan 029b while repository closeout items or live external MCP client acceptance remain open.

---

# Final Closeout Delta — post-implementation review

This section is the **only new implementation delta** after the Phase 1–9 pass. Do not reopen completed Plan 029/029b architecture or create Plan 029c for these items.

## A. RustSec dependency path — repository release blocker

Current state:

- `jsonwebtoken = { version = "11.0.0", features = ["rust_crypto"] }` pulls the RustCrypto RSA path that currently triggers `RUSTSEC-2023-0071`.
- The relay uses JWT public-key verification; nevertheless the repository's zero-bypass `cargo audit` policy remains authoritative.

Tasks:

- [x] Switch `jsonwebtoken` to its supported `aws_lc_rs` crypto-provider feature; locked checks and the RS256 fixture black-box path pass.
- [x] Regenerate `Cargo.lock` normally with Cargo; no dependency resolution was hand-edited.
- [x] Verify no `rsa` advisory path remains in `cargo audit`.
- [x] Re-run locked `cargo check`, strict Clippy, black-box MCP conformance, and `cargo audit`.
- [x] Do **not** add an advisory ignore/allowlist merely to make CI green.
- [x] The alternative provider satisfies the required RS256/ES256 verification contract in the locked build and fixture black-box path; no waiver is needed.

Exit:

- [x] `cargo audit` passes without suppressing `RUSTSEC-2023-0071`.

## B. CI bubblewrap dependency — repository release blocker

Current state:

- `scripts/phase4-black-box.sh` correctly requires `bwrap` because relay startup is fail-closed without the sandbox.
- The CI workflow now explicitly installs/provisions `bubblewrap` before the runtime black-box gates.

Tasks:

- [x] Explicitly provision `bubblewrap` in the Linux CI job before the black-box harness.
- [x] Keep the relay's production startup requirement unchanged; no no-bwrap test bypass was added.
- [x] The CI workflow runs the Phase 4 and Phase 7 runtime harnesses with the provisioned dependency.

Exit:

- [x] GitHub CI has explicit `bwrap` provisioning and runtime black-box conformance steps; repository-local execution passes.

## C. Authoritative `tools/list` contract snapshot — reopen 29B-8

Current state:

- `.agents/contracts/029-tool-catalog-v1.json` now contains substantially more descriptor detail, but it is still maintained separately from the actual runtime `tools/list` serialization.
- `scripts/phase7-external-mcp-contract.sh` hashes the snapshot but does not yet prove that external MCP client's runtime descriptor output equals it.

Tasks:

- [x] Obtain actual `tools/list` output from the deterministic local black-box relay.
- [x] Extract canonical `.result.tools` descriptors from the real MCP response.
- [x] Canonicalize both runtime descriptors and frozen snapshot with deterministic key ordering.
- [x] Fail CI when runtime descriptors differ from the frozen reviewed snapshot.
- [x] Make the frozen file represent the actual on-wire `Tool` descriptor shape only; non-wire wrapper fields were removed.
- [x] A deliberate descriptor change requires an explicit snapshot update/review; accidental drift fails.

Exit:

- [x] changing title/description/input schema/annotations/on-wire security metadata in Rust without updating the reviewed snapshot fails the contract gate.

## D. No-unit-test scope cleanup — non-blocking

Final review found `#[cfg(test)]` modules added to `config.rs` and `transport.rs`.

The project decision is **no unit-test requirement for Plan 029/029b**, not "tests are forbidden at runtime". These modules do not affect release behavior because CI does not use `cargo test`, but they are unnecessary scope for this deadline because the relevant cases are already covered by the black-box harness.

Tasks:

- [x] Removed the newly added low-risk config/transport unit-test modules; their release behavior is covered by black-box conformance.
- [x] No test-only rewrite was undertaken.
- [x] No `cargo test` Plan 029b completion gate was added.

Exit:

- [x] no test-only work blocks release.

## E. OAuth issuer HTTPS validation — production hardening, non-blocking before live setup

Current Remote-mode validation strictly canonicalizes the resource/audience URI, but the configured `oauth_issuer` still needs an explicit production HTTPS policy.

Tasks:

- [x] Require a canonical absolute HTTPS issuer for production Remote mode.
- [x] Preserve deterministic local black-box testing through a debug-only, explicit fixture environment variable; release builds cannot enable the exception.
- [x] Issuer URLs remain configuration-only and are not accepted from MCP tool arguments.

Exit:

- [x] production Remote config cannot use a plaintext external Authorization Server issuer; the only exception is debug-build fixture mode.

## F. Docker v1 decision — accepted limitation, deferred follow-up

Docker is **not a repository merge blocker for the first external MCP client coding-agent release** provided all of the following remain true:

- [x] Docker stays disabled when no isolated backend exists.
- [x] raw host Docker socket is not exposed.
- [x] no claim is made that Docker coding is supported in v1.
- [x] the limitation is documented.

A future Docker capability should be planned separately only when there is an actual isolated worker/broker architecture to implement and test. Do not keep Plan 029b open solely because the workstation currently lacks that backend.

## G. Live external MCP client/OAuth acceptance — final external production gate

After A–E repository items are resolved (with D optional cleanup), the remaining production claim depends on current external MCP client behavior and cannot be replaced by repository checks.

Required:

- [ ] approved trusted HTTPS/tunnel path is available.
- [ ] external MCP client `Scan Tools` succeeds against the actual endpoint.
- [ ] OAuth user-defined client flow, exact callback, PKCE and refresh work.
- [ ] verified owner receives `relay.coding`.
- [ ] real inspect -> edit -> install/build/run -> verify coding workflow succeeds.
- [ ] invalid token/wrong owner/wrong audience/missing scope remain denied in the real integration.
- [ ] redacted evidence is recorded without secrets or source leakage.

Final status semantics:

- **Repository implementation merge-ready:** A, B, C, E and required static/black-box/security gates are green; Docker remains explicitly unsupported.
- **Production-verified external MCP client MCP coding agent:** repository implementation is merge-ready **and** G passes in the live external MCP client/OAuth environment.
- Do not mark Plan 029b `COMPLETED` merely because live acceptance is unavailable; use an explicit pending/blocked status until G is evidenced.

---

## Definition of Done

Plan 029b closes only the unresolved production gaps left by Plan 029 while preserving its existing architecture and coding-agent ergonomics. The relay has a trustworthy HTTPS/proxy boundary, standards-compatible OAuth challenges and Protected Resource metadata, real flood admission control, behavior-based conformance gates, authoritative runtime-vs-snapshot tool descriptor checking, consistent observability metadata, clean dependency/security gates, and verified live external MCP client Scan Tools/OAuth/coding acceptance.

Docker is an explicitly deferred v1 capability unless/until an isolated backend exists; its absence must not be hidden, but it does not justify exposing the host Docker daemon or blocking the otherwise production-ready coding agent.

**Branch invariant:** every 029b change is implemented and documented on `feat/029-p0-audit`; this plan does not create or use another implementation branch.

---

## H. external MCP client tool auth metadata — pre-E2E repository blocker

This is an **append-only follow-up** from the final repository review. Do not reopen or rewrite completed sections above.

Current OpenAI Plugin authentication guidance requires each OAuth-protected MCP tool to declare its auth policy with `securitySchemes`. The current custom top-level `security` object only duplicates risk annotations and does not express external MCP client's OAuth tool policy.

Tasks:

- [x] Remove the custom top-level `ToolSecurity` / `security` descriptor field from the on-wire tool contract.
- [x] Keep MCP risk annotations (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`) unchanged unless actual side effects changed.
- [x] Add per-tool `securitySchemes` to `terminal_exec`, `http_fetch`, and `web_search`.
- [x] For the current coarse coding profile, declare OAuth-required semantics using `oauth2` with scope `relay.coding` for all three tools.
- [x] Do not introduce fake per-tool scopes merely to make metadata look granular; server-side authorization remains the existing `relay.coding` policy.
- [x] Treat `securitySchemes` as client-visible auth metadata only; token signature, issuer, audience, expiry, owner and scope checks remain authoritative server-side controls.
- [x] Regenerate the frozen runtime `tools/list` snapshot only from the actual serialized descriptors after this change.
- [x] Make Phase 7 fail if `securitySchemes` disappears or drifts from the reviewed OAuth policy.

Exit:

- [x] runtime `tools/list` exposes standards-compatible per-tool `securitySchemes` and no longer exposes the custom duplicate `security` field.

## I. external MCP client tool-level OAuth challenge — pre-E2E repository blocker

OpenAI's current Plugin authentication guidance requires both tool auth metadata and a runtime tool-result challenge for external MCP client's tool-level OAuth linking/re-linking UI. Existing HTTP `401/403` `WWW-Authenticate` behavior remains required and must not be removed.

Tasks:

- [x] Extend tool error results so they can emit result `_meta["mcp/www_authenticate"]` without affecting normal successful tool results.
- [x] Generate the challenge from the same protected-resource metadata URL/auth policy used by the HTTP `WWW-Authenticate` helper so the two paths cannot drift.
- [x] Include an OAuth error parameter and a human-safe `error_description` in the tool-level challenge, as required by current OpenAI guidance.
- [x] Do not place bearer tokens, claims, owner identifiers, client secrets or sensitive command/source content inside `_meta` or `error_description`.
- [x] Preserve HTTP `401 invalid_token` and `403 insufficient_scope` responses for resource-server enforcement.
- [x] Preserve authorization-before-execution: an unauthenticated, wrong-owner or missing-scope request must never reach tool dispatch merely to manufacture a tool-level challenge.
- [x] Implement the external MCP client-compatible challenge path at the correct MCP result boundary without weakening the existing middleware/resource-server checks.
- [x] Add deterministic black-box coverage for the emitted `_meta["mcp/www_authenticate"]` shape and verify the value references the same protected-resource metadata URL.
- [x] Verify both the metadata half (`securitySchemes`) and runtime challenge half are present before live external MCP client OAuth testing.

Exit:

- [x] a deterministic auth-required tool flow can return an MCP error result carrying `_meta["mcp/www_authenticate"]` with `error` + `error_description`, while existing HTTP auth enforcement remains intact.

## J. Discovery metadata wording — pre-E2E polish

The current `server/discover.instructions` still describes the relay as a local server and references internal implementation history. external MCP client imports discovery/tool metadata during setup, so publish neutral product-facing wording before `Scan Tools`.

Tasks:

- [x] Replace `Local relay-agent MCP server` wording with neutral coding-agent wording that is true for both loopback+tunnel and trusted-edge deployments.
- [x] Remove references to internal Plan numbers or migration history from client-visible discovery instructions.
- [x] Keep the description concise and capability-oriented: sandboxed coding terminal, configured HTTP access, web search, and workspace policy.
- [x] Do not claim Docker support while Docker remains deferred for v1.
- [x] Run `server/discover` black-box validation after the wording change.

Exit:

- [x] discovery metadata is accurate, deployment-neutral, and suitable for the first real external MCP client `Scan Tools` run.

### Pre-E2E handoff gate

Do **not** start the live external MCP client acceptance in section G until H and I are complete and J has been applied.

Before handoff to live external MCP client testing:

- [ ] strict format/check/Clippy/audit gates remain green.
- [ ] Phase 4 auth/security black-box remains green.
- [ ] Phase 7 runtime descriptor snapshot gate is regenerated and green.
- [ ] Phase 8 zero-bypass gate remains green.
- [ ] real `tools/list` contains reviewed `securitySchemes` and no duplicate custom `security` field.
- [ ] deterministic OAuth error path proves `_meta["mcp/www_authenticate"]` without executing an unauthorized tool.
- [ ] `server/discover.instructions` contains no `Local`/internal-plan wording.

Once those checks pass, the only remaining Plan 029b release gate is the real external MCP client/OAuth acceptance already listed in section G.

---

## K. MCP `2026-07-28` `tools/call` result wire conformance — pre-E2E repository blocker

### Why this is open

The final review found that `tools/list` already returns `resultType: "complete"`, but `ToolCallResult` currently serializes only `content` + `isError`. MCP `2026-07-28` requires a `resultType` discriminator on wire results; a completed normal tool invocation must therefore carry `resultType: "complete"`.

Do this **before** implementing section I so the OAuth challenge result is built on the correct `tools/call` wire shape from the start.

### Files expected to change

- `packages/rust-tools/src/relay_agent/mcp.rs`
- `packages/rust-tools/src/relay_agent/execution.rs`
- `packages/rust-tools/src/relay_agent/transport.rs`
- `scripts/phase4-black-box.sh`
- Plan 029b evidence only if needed

### Implementation checklist

- [x] Add `result_type` to `ToolCallResult` with `#[serde(rename = "resultType")]`.
- [x] Completed success results serialize exactly `resultType: "complete"`.
- [x] Completed tool/business errors (`isError: true`) also serialize `resultType: "complete"`; `isError` does **not** change the result discriminator.
- [x] Do not emit `resultType: "input_required"` unless a future MRTR flow is actually implemented.
- [x] Add one or more constructors/helpers (`complete`, `error`, or equivalent) so callers cannot accidentally build a `ToolCallResult` without `resultType`.
- [x] Replace every direct `ToolCallResult { ... }` construction in `execution.rs` with the canonical helper or include the required discriminator explicitly.
- [x] Replace the execution-error fallback construction in `transport.rs` with the same canonical path.
- [x] Keep `Response::new()` server `_meta` stamping intact; adding `resultType` must not overwrite existing result `_meta`.
- [x] Keep `content` non-empty and `isError` semantics unchanged unless a concrete protocol requirement says otherwise.

### Required black-box acceptance

Extend `scripts/phase4-black-box.sh` with **real successful and failed tool execution**, not source grep:

- [x] valid local `terminal_exec` call using a harmless command such as `true` returns HTTP `200`.
- [x] success response has `result.resultType == "complete"`.
- [x] success response has `result.isError == false` and a `content` array.
- [x] a harmless failing command such as `false` returns a normal MCP tool result, not a JSON-RPC protocol error.
- [x] failed tool response still has `result.resultType == "complete"`.
- [x] failed tool response has `result.isError == true`.
- [x] both responses retain `_meta["io.modelcontextprotocol/serverInfo"]`.

### Exit

- [x] no reachable `tools/call` success/error path can serialize a completed result without `resultType: "complete"`.

---

## L. Exact implementation sequence for remaining I + J + K work

This is the **authoritative execution order** for the final repository pass. It clarifies the unchecked H/I/J addendum without rewriting completed sections above.

### Step 1 — Fix the base `ToolCallResult` shape (K)

**Target:** `mcp.rs`, `execution.rs`, `transport.rs`.

- [x] implement `resultType` first.
- [x] add optional result metadata support to `ToolCallResult`, preferably `meta: Option<Value>` serialized as `_meta` and omitted when empty.
- [x] centralize constructors so `resultType`, `isError`, `content`, and optional `_meta` are created consistently.
- [x] run the new local success/failure `tools/call` black-box assertions before touching OAuth challenge behavior.

**Stop condition:** do not continue if a strict 2026-07-28 client would still receive a completed tool result without `resultType`.

### Step 2 — Add the external MCP client OAuth result challenge (I)

**Target:** primarily `transport.rs` + `mcp.rs`; touch `execution.rs` only if result constructors require it.

#### 2A. One challenge formatter

- [x] Refactor the existing Bearer challenge construction so there is one canonical function that produces the challenge value/string.
- [x] Reuse that exact value for the HTTP `WWW-Authenticate` header and `_meta["mcp/www_authenticate"]`; do not maintain two independently formatted challenges.
- [x] Challenge must reference the existing canonical protected-resource metadata URL.
- [x] Tool-level challenge must include both `error` and `error_description`.
- [x] Keep `error_description` generic and safe; never include token text, subject, claims, command args, source contents, or IdP secrets.

#### 2B. Narrow auth-state handling; never dispatch unauthorized tools

The runtime challenge needs a request-aware auth failure state, but it must **not** weaken the execution boundary.

- [x] Introduce an explicit internal auth decision/state (name is implementation-defined) instead of representing every auth failure only as an immediate HTTP response.
- [x] Keep malformed/expired/unverifiable bearer tokens on the existing hard HTTP `401 invalid_token` path.
- [x] Keep wrong-owner authorization on a hard deny path; do not turn an owner mismatch into a normal executable tool request.
- [x] For a `tools/call` that needs initial linking or additional `relay.coding` authorization, allow only enough request processing to construct the MCP auth-required result; **never** acquire the execution semaphore or call `dispatch_tool_call`.
- [x] A missing-auth challenge uses a safe auth error such as `invalid_token`/authentication-required semantics.
- [x] A valid token missing `relay.coding` uses `insufficient_scope` and identifies the required `relay.coding` scope in the challenge where applicable.
- [x] If the implementation changes the exact HTTP status for the `tools/call` challenge path from the older Phase 2 baseline, make that exception explicit in the black-box assertions; do not silently weaken the hard-deny behavior for malformed tokens or wrong owners.
- [x] Preserve normal server-side signature, issuer, audience, `exp`/`nbf`, subject, and scope enforcement before any side effect.

#### 2C. Build the challenge result

The resulting JSON-RPC success envelope for the auth-required tool condition must contain a normal MCP tool error result:

- [x] `result.resultType == "complete"`.
- [x] `result.isError == true`.
- [x] `result.content` contains a short generic authentication-required message.
- [x] `result._meta["mcp/www_authenticate"]` is an array of challenge strings.
- [x] the challenge contains the same `resource_metadata` URL used by HTTP auth challenges.
- [x] `Response::new()` merges `io.modelcontextprotocol/serverInfo` into `_meta` without deleting `mcp/www_authenticate`.

#### 2D. Deterministic auth black-box matrix

Extend `scripts/phase4-black-box.sh` so the final behavior is explicit:

- [x] no/missing auth on the selected `tools/call` linking path returns the intended auth-required behavior and **does not create the dispatch marker**.
- [x] malformed bearer remains HTTP `401` + `invalid_token`; no dispatch.
- [x] expired bearer remains HTTP `401` + `invalid_token`; no dispatch.
- [x] wrong owner remains hard denied; no dispatch and no subject leakage.
- [x] valid owner token missing `relay.coding` produces the intended reauthorization challenge or documented hard-deny exception; no dispatch.
- [x] challenge result includes `resultType: "complete"`, `isError: true`, and `_meta["mcp/www_authenticate"]`.
- [x] challenge string includes `error`, `error_description`, and exact `resource_metadata`.
- [x] valid owner + valid audience + `relay.coding` reaches real tool execution and returns a normal completed result.

**Stop condition:** a request without effective coding authorization must have zero path to `dispatch_tool_call`.

### Step 3 — Clean discovery wording (J)

**Target:** `packages/rust-tools/src/relay_agent/mcp.rs` and existing black-box script.

- [x] replace the current `Local relay-agent MCP server...Plan 027...` text.
- [x] use deployment-neutral product wording only.
- [x] mention the actual v1 capabilities: sandboxed coding terminal, HTTP requests, web search, and configured workspace policy.
- [x] do not mention internal Plan 027/028/029 history.
- [x] do not claim Docker support.
- [x] add/extend `server/discover` black-box assertion so the returned instruction text contains neither `Local` nor `Plan 0`/internal plan wording.

Suggested intent, not mandatory exact copy:

> Relay Agent MCP coding server exposing a sandboxed coding terminal, HTTP requests, and web search within the configured workspace policy.

### Step 4 — Re-run the repository handoff gates

Run in this order after code changes:

- [ ] `cargo fmt --all -- --check`
- [ ] `RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo audit`
- [ ] `bash scripts/phase4-black-box.sh`
- [ ] `bash scripts/phase7-external-mcp-contract.sh`
- [ ] `bash scripts/phase8-zero-bypass.sh`
- [ ] `bash scripts/phase6-external-mcp-e2e.sh` for its repository/static portion

### Step 5 — Final pre-E2E evidence review

Do **not** start external MCP client live E2E until all are true:

- [ ] H remains green: every tool exposes `securitySchemes: [{"type":"oauth2","scopes":["relay.coding"]}]` and no custom top-level `security` field.
- [ ] K is green: real successful and failed `tools/call` responses carry `resultType: "complete"`.
- [ ] I is green: deterministic auth-required tool flow emits `_meta["mcp/www_authenticate"]` with safe `error` + `error_description`, and unauthorized requests never dispatch.
- [ ] J is green: discovery wording is deployment-neutral and contains no internal Plan references.
- [ ] HTTP `401 invalid_token`, protected-resource metadata, issuer/audience validation, trusted-proxy boundary, owner binding, admission control, and bwrap sandbox regressions remain green.
- [ ] Phase 7 runtime descriptor snapshot still matches the reviewed frozen tool catalog; update the snapshot only if actual `tools/list` descriptors changed.
- [ ] `cargo audit` remains clean with no advisory ignore.
- [ ] Docker remains explicitly unsupported/deferred and host Docker socket remains unavailable.

When Step 5 passes, repository review status becomes **GO FOR LIVE EXTERNAL-MCP E2E**. Section G is then the only remaining acceptance work.
