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

Make authentication failures and insufficient-scope failures recoverable by standards-aware MCP clients including ChatGPT.

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

- `scripts/phase6-chatgpt-e2e.sh`
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

- [x] ordinary ChatGPT coding burst succeeds.
- [x] normal long build unaffected.
- [x] sustained request flood is throttled/rejected.
- [x] execution concurrency stays bounded.

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

- [x] 29B-1 closed.
- [x] 29B-2 closed.
- [x] 29B-3 is an accepted v1 limitation: Docker remains disabled until an isolated backend exists; do not expose the host Docker socket or remove the ban without that backend.
- [ ] 29B-4 live acceptance closed.
- [x] 29B-5 closed.
- [x] 29B-6 closed.
- [x] 29B-7 closed.
- [ ] 29B-8 reopened by final review: the frozen snapshot is fuller but must still be compared against the actual serialized `tools/list` descriptors.
- [x] 29B-9 closed.
- [x] 29B-10 closed via the centralized response metadata helper.
- [x] no Plan 028/029 subsystem was reimplemented unnecessarily.
- [x] no broad coding denylist was introduced beyond the explicitly deferred Docker capability and existing privilege-boundary restrictions.
- [x] no unit-test **requirement or CI gate** was added.
- [ ] strict format/check/clippy/audit gates pass; `cargo audit` currently fails on `RUSTSEC-2023-0071` through the selected `jsonwebtoken` crypto backend.
- [x] black-box connector/security conformance passes locally.
- [ ] real ChatGPT acceptance passes.
- [x] all commits remain on `feat/029-p0-audit` until the existing Plan 029 branch is intentionally merged through the normal repository workflow.

## Phase 9 closeout evidence — 2026-08-12

Available repository gates passed:

- `cargo fmt --all -- --check`
- `RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `pnpm run lint`, `pnpm run typecheck`, and `pnpm audit`
- `scripts/phase4-black-box.sh`
- `scripts/phase7-chatgpt-contract.sh`
- `scripts/phase8-zero-bypass.sh`
- `scripts/phase6-chatgpt-e2e.sh` static checks

Open gates and blockers:

- Phase 8 live ChatGPT acceptance is unavailable in this repository run: no approved HTTPS/tunnel deployment, ChatGPT workspace/app, OAuth tenant/client, callback verification, or redacted live evidence is available. The Phase 6 harness reports this as unavailable; it is not treated as a pass.
- Docker is intentionally deferred for v1. No isolated Docker worker, restricted broker, or equivalent backend is available, so Docker remains disabled to preserve the host boundary. This is an accepted capability limitation, not permission to weaken the sandbox. See `.agents/memories/029b-docker-capability-blocker.md`.
- `cargo audit` fails on `RUSTSEC-2023-0071` through the current `jsonwebtoken` `rust_crypto` dependency path. Do not suppress or ignore the advisory; attempt the supported non-`rsa` crypto provider path first.
- GitHub CI must explicitly provide the Linux `bubblewrap` dependency before executing `scripts/phase4-black-box.sh`; do not rely on runner-image accident/defaults.
- The current Phase 7 snapshot is not yet authoritative until CI compares canonicalized actual `tools/list.result.tools` output against the frozen descriptor snapshot.

Phase 9 does not close Plan 029b while repository closeout items or live ChatGPT acceptance remain open.

---

# Final Closeout Delta — post-implementation review

This section is the **only new implementation delta** after the Phase 1–9 pass. Do not reopen completed Plan 029/029b architecture or create Plan 029c for these items.

## A. RustSec dependency path — repository release blocker

Current state:

- `jsonwebtoken = { version = "11.0.0", features = ["rust_crypto"] }` pulls the RustCrypto RSA path that currently triggers `RUSTSEC-2023-0071`.
- The relay uses JWT public-key verification; nevertheless the repository's zero-bypass `cargo audit` policy remains authoritative.

Tasks:

- [ ] Switch `jsonwebtoken` to its supported `aws_lc_rs` crypto-provider feature if it preserves required RS256/ES256 verification behavior.
- [ ] Regenerate `Cargo.lock` normally; do not hand-edit dependency resolution.
- [ ] Verify no `rsa` advisory path remains in `cargo audit`.
- [ ] Re-run locked `cargo check`, strict Clippy, black-box MCP conformance, and `cargo audit`.
- [ ] Do **not** add an advisory ignore/allowlist merely to make CI green.
- [ ] If the alternative provider cannot satisfy the required algorithms/platform contract, document the exact blocker before considering any waiver.

Exit:

- [ ] `cargo audit` passes without suppressing `RUSTSEC-2023-0071`.

## B. CI bubblewrap dependency — repository release blocker

Current state:

- `scripts/phase4-black-box.sh` correctly requires `bwrap` because relay startup is fail-closed without the sandbox.
- The CI workflow invokes the black-box harness but does not explicitly install/provision `bubblewrap` first.

Tasks:

- [ ] Explicitly provision `bubblewrap` in the Linux CI job before the black-box harness.
- [ ] Keep the relay's production startup requirement unchanged; do not add a no-bwrap test bypass.
- [ ] Verify the black-box harness runs in GitHub Actions rather than only on a developer machine.

Exit:

- [ ] GitHub CI has a verifiable green black-box conformance step with real `bwrap` available.

## C. Authoritative `tools/list` contract snapshot — reopen 29B-8

Current state:

- `.agents/contracts/029-tool-catalog-v1.json` now contains substantially more descriptor detail, but it is still maintained separately from the actual runtime `tools/list` serialization.
- `scripts/phase7-chatgpt-contract.sh` hashes the snapshot but does not yet prove that ChatGPT's runtime descriptor output equals it.

Tasks:

- [ ] Obtain actual `tools/list` output from the deterministic local black-box relay.
- [ ] Extract canonical `.result.tools` descriptors from the real MCP response.
- [ ] Canonicalize both runtime descriptors and frozen snapshot with deterministic key ordering.
- [ ] Fail CI when runtime descriptors differ from the frozen reviewed snapshot.
- [ ] Make the frozen file represent the actual on-wire `Tool` descriptor shape only; remove parallel helper fields that are not serialized to ChatGPT unless they are explicitly documented as non-wire review metadata.
- [ ] A deliberate descriptor change requires an explicit snapshot update/review; accidental drift must fail.

Exit:

- [ ] changing title/description/input schema/annotations/on-wire security metadata in Rust without updating the reviewed snapshot fails the contract gate.

## D. No-unit-test scope cleanup — non-blocking

Final review found `#[cfg(test)]` modules added to `config.rs` and `transport.rs`.

The project decision is **no unit-test requirement for Plan 029/029b**, not "tests are forbidden at runtime". These modules do not affect release behavior because CI does not use `cargo test`, but they are unnecessary scope for this deadline because the relevant cases are already covered by the black-box harness.

Tasks:

- [ ] Prefer removing the newly added Plan 029b unit-test modules if doing so is low-risk and their behavior is already covered by black-box conformance.
- [ ] Do not spend meaningful delivery time rewriting them into another test framework.
- [ ] Never add `cargo test` as a Plan 029b completion gate.

Exit:

- [ ] no test-only work blocks release.

## E. OAuth issuer HTTPS validation — production hardening, non-blocking before live setup

Current Remote-mode validation strictly canonicalizes the resource/audience URI, but the configured `oauth_issuer` still needs an explicit production HTTPS policy.

Tasks:

- [ ] Require an absolute HTTPS issuer for production Remote mode.
- [ ] Preserve deterministic local black-box testing through an explicit fixture-only mechanism or local harness arrangement rather than weakening production issuer validation globally.
- [ ] Do not accept issuer URLs from MCP tool arguments.

Exit:

- [ ] production Remote config cannot use a plaintext external Authorization Server issuer.

## F. Docker v1 decision — accepted limitation, deferred follow-up

Docker is **not a repository merge blocker for the first ChatGPT coding-agent release** provided all of the following remain true:

- [x] Docker stays disabled when no isolated backend exists.
- [x] raw host Docker socket is not exposed.
- [x] no claim is made that Docker coding is supported in v1.
- [x] the limitation is documented.

A future Docker capability should be planned separately only when there is an actual isolated worker/broker architecture to implement and test. Do not keep Plan 029b open solely because the workstation currently lacks that backend.

## G. Live ChatGPT/OAuth acceptance — final external production gate

After A–E repository items are resolved (with D optional cleanup), the remaining production claim depends on current ChatGPT behavior and cannot be replaced by repository checks.

Required:

- [ ] approved trusted HTTPS/tunnel path is available.
- [ ] ChatGPT `Scan Tools` succeeds against the actual endpoint.
- [ ] OAuth user-defined client flow, exact callback, PKCE and refresh work.
- [ ] verified owner receives `relay.coding`.
- [ ] real inspect -> edit -> install/build/run -> verify coding workflow succeeds.
- [ ] invalid token/wrong owner/wrong audience/missing scope remain denied in the real integration.
- [ ] redacted evidence is recorded without secrets or source leakage.

Final status semantics:

- **Repository implementation merge-ready:** A, B, C and required static/black-box/security gates are green; E is completed before real remote deployment; Docker remains explicitly unsupported.
- **Production-verified ChatGPT MCP coding agent:** repository implementation is merge-ready **and** G passes in the live ChatGPT/OAuth environment.
- Do not mark Plan 029b `COMPLETED` merely because live acceptance is unavailable; use an explicit pending/blocked status until G is evidenced.

---

## Definition of Done

Plan 029b closes only the unresolved production gaps left by Plan 029 while preserving its existing architecture and coding-agent ergonomics. The relay has a trustworthy HTTPS/proxy boundary, standards-compatible OAuth challenges and Protected Resource metadata, real flood admission control, behavior-based conformance gates, authoritative runtime-vs-snapshot tool descriptor checking, consistent observability metadata, clean dependency/security gates, and verified live ChatGPT Scan Tools/OAuth/coding acceptance.

Docker is an explicitly deferred v1 capability unless/until an isolated backend exists; its absence must not be hidden, but it does not justify exposing the host Docker daemon or blocking the otherwise production-ready coding agent.

**Branch invariant:** every 029b change is implemented and documented on `feat/029-p0-audit`; this plan does not create or use another implementation branch.
