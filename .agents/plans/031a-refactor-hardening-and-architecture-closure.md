# Plan 031A — Refactor Hardening and Architecture Closure

**Status: REOPENED — SECOND-AUDIT BLOCKERS REMAIN**  
**Created: 2026-08-13**  
**Parent plan: Plan 031 — Repository-wide Layered Refactor**  
**Implementation branch: `refactor/031-repository-wide-layered-refactor`**  
**Original 031A audit baseline: `b241175e131e544ba7cf922f8d5865557e3f66e3`**  
**Second deep-review baseline: `dcd2fb4` branch head reviewed on 2026-08-13**

## Status correction

Plan 031A was previously described as having all P0/P1/P2 implementation findings resolved, with only release verification outstanding. A second source-level deep review found several remaining security, compatibility, tenant-boundary, architecture, and deterministic-acceptance gaps.

That previous completion claim is therefore withdrawn.

Plan 031A remains the owner of these gaps. Do **not** create Plan 031B merely to move unfinished 031A acceptance criteria elsewhere. Plan 031A may be closed only after the findings and verification requirements below are actually satisfied.

Plan 031 itself remains closed as the parent refactor pass. Closing Plan 031 is an administrative scope boundary, not proof that 031A is complete.

---

## Mission

Finish the Plan 031 refactor hardening truthfully and strictly, with emphasis on:

- tenant isolation;
- SSRF and secret handling;
- real Layered Architecture dependency direction;
- DRY, SOLID, KISS, reusable logic, and cohesive folder ownership;
- deterministic architecture/security enforcement;
- Rust relay security ordering and contract verification;
- backward-compatible data handling;
- honest release verification.

A folder move, green grep, successful compile, or fail-closed error alone is not sufficient evidence of completion.

---

## Non-negotiable constraints

1. No CI.
2. No unit-test suite or Rust `#[cfg(test)]` modules.
3. Never bypass git hooks; `pnpm verify:commit` remains the canonical commit gate.
4. No generic `Repository<T>`, `CrudService<T>`, DI framework, service locator, speculative plugin architecture, or cosmetic abstraction.
5. Application code must depend on narrow capabilities/contracts, not concrete Drizzle/provider/AI SDK/LangGraph implementations.
6. Security fixes must fail closed without creating cross-tenant enumeration or new network reachability.
7. Native terminal/curl/search execution remains Rust-owned and continues through the single reviewed sandbox/process-safety path.
8. No opportunistic dependency upgrades.
9. Do not claim a command, runtime flow, browser smoke, or security matrix passed unless it was actually executed successfully.

---

# First-pass 031A result retained

The first 031A implementation materially improved the repository and those improvements must be preserved while fixing the remaining gaps:

- conversation/model/provider/default-model ownership checks were centralized and strengthened;
- `server/api/chat.post.ts` became a thin transport adapter;
- an H3-independent `executeChatTurn()` application entrypoint was introduced;
- submit/regenerate/resume semantics were moved out of database infrastructure;
- frontend components were grouped under `chat/`, `workspace/`, `settings/`, and `shell/`;
- `AppSidebar` was reduced and workspace-dialog responsibilities were extracted;
- Plan 031 Rust `#[cfg(test)]` modules were removed;
- `@opentelemetry/sdk-node` manifest/lock mismatch was corrected;
- architecture checking was added to `pnpm verify:commit`;
- Rust execution gained clearer tool-specific invocation preparation while retaining one process-safety path;
- `pnpm audit` and `cargo audit` were reported clean during the first implementation pass;
- `scripts/phase8-zero-bypass.sh` passed during that pass.

These are historical implementation results, not proof that the current branch satisfies the final Definition of Done.

---

# Second deep-review findings

## P0 — Merge blockers

### Q. Provider SSRF protection does not validate redirect hops

`createSsrfSafeFetch()` validates only the initial URL and then delegates to native/runtime `fetch`, which may follow redirects internally. A public URL that redirects to loopback, RFC1918, link-local, or cloud-metadata space can therefore escape the intended policy.

The address classifier also needs explicit review of IPv4-mapped IPv6 forms so private IPv4 ranges cannot bypass classification through mapped addresses.

Required outcome:

- enforce a bounded redirect policy at the actual outbound connection boundary;
- validate every redirect target before connecting;
- reject loopback, RFC1918, link-local, metadata, unspecified, and equivalent IPv6 / IPv4-mapped forms;
- preserve normal public HTTP(S) provider behavior;
- do not silently downgrade HTTPS or forward sensitive headers across an unsafe redirect boundary;
- document any remaining DNS-rebinding limitation precisely and do not overclaim protection.

Acceptance:

- a public URL redirecting to `127.0.0.1`, `10.0.0.0/8`, `169.254.169.254`, or equivalent IPv6/mapped forms is rejected before the private target is contacted;
- safe public redirects still work within a bounded hop count;
- provider discovery and actual chat SDK paths use the same authoritative rule where technically possible.

### R. Existing plaintext provider `customHeaders` have no migration/upgrade path

The earlier provider schema stored `customHeaders` as JSONB plaintext. The 031A implementation now treats stored header values as encrypted secrets, but the reviewed migration history contains no explicit backfill or compatibility path for existing plaintext rows.

Required outcome:

- define a deterministic way to distinguish legacy plaintext values from encrypted values;
- migrate/backfill safely, or implement an explicit one-time lazy upgrade with no secret exposure;
- keep new writes encrypted/redacted;
- never return decrypted secret header values through ordinary provider DTOs;
- never log plaintext/decrypted values;
- make repeated execution idempotent.

Acceptance:

- providers created before secret-header encryption continue to work after upgrade;
- malformed/corrupt encrypted values fail safely;
- no migration path can double-encrypt already encrypted values.

---

## P1 — Architecture, authorization, and verification blockers

### S. Application/infrastructure dependency direction is still incomplete

`server/api/chat.post.ts` is now thin, but `server/application/chat/execute-chat-turn.ts` still knows concrete integration details through imports/types/helpers tied to AI SDK/provider/LangGraph/MCP/context-compaction implementations. Some dependencies are hidden transitively behind `server/utils/**`, so current folder names overstate the achieved dependency inversion.

Required outcome:

- keep orchestration and turn semantics in application;
- move concrete AI SDK, provider factory, LangGraph, persistence, MCP client, and implementation-specific context-compaction details behind narrow capabilities/adapters;
- application may own plain input/output/value contracts needed for orchestration, but not concrete infrastructure constructors;
- no DI framework: use an explicit small dependency object/factory at the composition edge;
- avoid needless one-function files or abstraction for abstraction's sake.

Target direction:

```text
server/api (transport/composition)
  -> server/application (use cases/policies)
      -> narrow contracts/capabilities
          <- server/infrastructure (DB / AI SDK / providers / LangGraph / MCP)
```

Acceptance:

- `executeChatTurn()` is H3-independent and concrete-infrastructure-independent;
- application source does not import concrete infrastructure modules or SDK implementation packages directly or through deliberately disguised facades;
- infrastructure does not decide submit/regenerate/resume business semantics;
- stream cleanup, abort, persistence, approval, local-tool, MCP-close-once, reasoning, and token-accounting behavior are preserved.

### T. Architecture gate currently gives false confidence

The current checker catches only a subset of forbidden import strings. It does not fully represent the actual architecture invariant and can miss concrete dependencies through package `ai`, `server/utils/**`, auto-imported DB access, or direct application-to-infrastructure imports.

Required outcome:

- strengthen `scripts/check-architecture.sh` and/or targeted ESLint restricted imports only after the final boundaries from Finding S are established;
- catch representative direct and indirect forbidden dependency forms used by this repository;
- protect the actual shipped architecture, not a speculative ideal;
- keep the checker deterministic and dependency-free;
- ensure `pnpm verify:commit` continues to invoke it.

Acceptance:

- representative forbidden imports/dependencies demonstrably fail the architecture gate;
- valid narrow contracts do not trigger false positives;
- removing the violation restores the gate.

### U. `lastActiveWorkspaceId` write path does not enforce workspace ownership

`PUT /api/workspaces/active` validates the UUID but writes it to the current user without first proving that the referenced workspace belongs to that same user.

Required outcome:

- reuse the authoritative workspace-ownership rule already introduced by 031A;
- allow `null` when the product contract permits clearing the active workspace;
- reject foreign/missing workspace IDs using the same non-enumerating semantics used elsewhere.

Acceptance:

- an authenticated user cannot persist another user's workspace ID as `lastActiveWorkspaceId`;
- no duplicate ownership rule is introduced.

### V. Malformed Bearer tokens should be rejected before expensive OIDC/JWKS work

The remote relay auth path can attempt discovery/JWKS refresh before cheap JWT header/syntax rejection when cache state requires refresh. Admission limiting reduces blast radius and current behavior remains fail closed, but junk tokens should not trigger unnecessary outbound authentication work.

Required outcome:

- perform cheap structural/JWT-header rejection before discovery/JWKS/signature work where this does not weaken valid token handling;
- preserve issuer/audience/signature/owner/scope validation and trusted-proxy ordering;
- maintain non-bypass behavior.

Acceptance:

- malformed non-JWT bearer input returns the intended authorization error without unnecessary IdP discovery/JWKS work;
- valid tokens still follow full validation.

### W. `phase4-black-box.sh` mock IdP is stale relative to current OIDC discovery flow

The deterministic black-box fixture serves JWKS but does not fully emulate the discovery endpoint now required by the relay. This makes the malformed-token case ambiguous and prevents the script from being authoritative evidence for the current auth flow.

Required outcome:

- update the deterministic fixture to serve the required OIDC discovery metadata and JWKS endpoint;
- keep the test local/deterministic and within the repository's no-unit-test policy;
- verify malformed token, bad signature, wrong issuer/audience/owner/scope, valid token, and relevant fail-closed paths as applicable.

Acceptance:

- `scripts/phase4-black-box.sh` passes completely against the reviewed relay behavior.

### X. `phase7-chatgpt-contract.sh` is stale after memory compaction

The script still reads a historical memory file that was intentionally removed when durable memory was compacted into `.agents/memories/README.md`.

Required outcome:

- move/freeze any machine-consumed contract hash or immutable fixture into an appropriate `.agents/contracts/` or other stable contract location;
- stop using mutable historical prose/memory as machine-readable contract storage;
- repair the script without weakening the intended contract check.

Acceptance:

- `scripts/phase7-chatgpt-contract.sh` passes deterministically;
- the expected contract value has one stable authoritative source.

---

## P2 — Truthfulness and polish

### Y. Rust execution comments must match the actual implementation

The current execution decomposition is broadly acceptable and should remain KISS-oriented. Clean up any comments or plan text that name a helper/function that does not actually exist or otherwise overstate the decomposition.

Do not extract another wrapper merely to make a comment true. Prefer correcting the comment when the current single authoritative process path is already clearer.

### Z. Release verification is still incomplete

The first 031A pass did not prove the full client Vue typecheck, `pnpm build`, preview/runtime behavior, browser flows, authenticated two-user isolation matrix, or full chat smoke in an environment capable of running them.

Source inspection and server-only typechecking remain useful evidence, but they are not substitutes for final release verification.

---

# Remaining execution phases

## Phase 9 — Close provider network and secret-storage blockers

**Risk: critical / security + data compatibility**

- [x] Fix redirect-aware provider SSRF enforcement and mapped-address classification.
- [x] Add deterministic redirect/private-target acceptance coverage.
- [x] Implement safe legacy `customHeaders` migration/lazy-upgrade behavior.
- [x] Verify new writes remain encrypted/redacted and legacy rows remain usable.

## Phase 10 — Finish tenant and application boundaries

**Risk: high / security + architecture**

- [ ] Enforce ownership on `lastActiveWorkspaceId` writes using the authoritative workspace rule.
- [ ] Refactor `executeChatTurn()` dependencies into narrow application-facing capabilities.
- [ ] Move remaining concrete provider/AI SDK/LangGraph/MCP/persistence implementation ownership to infrastructure/composition boundaries.
- [ ] Preserve all reviewed chat lifecycle semantics.

## Phase 11 — Make architecture enforcement match the architecture

**Risk: medium**

- [ ] Expand architecture checks for the finalized boundaries from Phase 10.
- [ ] Demonstrate representative negative probes fail.
- [ ] Keep `pnpm check:architecture` inside `pnpm verify:commit`.
- [ ] Avoid a new architecture-lint dependency.

## Phase 12 — Repair relay/contract deterministic acceptance

**Risk: high / security-sensitive**

- [ ] Cheap-reject malformed bearer syntax/header before expensive IdP work where safe.
- [ ] Repair the Phase 4 mock OIDC discovery/JWKS fixture.
- [ ] Repair the Phase 7 contract script and move machine-readable expected state out of historical memory prose.
- [ ] Run all applicable Rust/MCP/native-tool deterministic scripts to completion.

## Phase 13 — Final integrated verification and closure

**Risk: high because completion is repository-wide**

Run at minimum in a real checkout/environment capable of producing full Nuxt artifacts:

```sh
pnpm verify:commit
pnpm build
pnpm check:architecture
pnpm audit
cargo audit
```

Also run all applicable deterministic MCP/relay/native-tool scripts and browser/runtime smoke for touched flows.

Required live/runtime matrix includes at least:

- two-user model/provider/workspace/default-model/active-workspace isolation;
- provider public target success and private/metadata/redirect rejection;
- legacy and newly-written provider custom headers;
- chat send/regenerate/stop/abort;
- approval allow/deny/remember;
- MCP tool call and close/cleanup behavior;
- local-terminal offline/error path;
- reasoning/provider variants that are configured in the test environment;
- sidebar/workspace/conversation/settings flows.

Do not mark a matrix item passed if the necessary provider/database/browser environment was unavailable.

---

## Final Definition of Done

Plan 031A is complete only when all of the following are true:

1. Findings Q–X are resolved, not merely documented.
2. No P0 or P1 finding from either the first or second deep review remains open.
3. Provider SSRF enforcement covers actual redirect behavior and reviewed address forms at the real connection boundary.
4. Existing plaintext custom-header rows have a safe, idempotent upgrade path.
5. Every persisted user-owned model/provider/workspace reference, including active workspace, is authorized server-side.
6. `server/application/**` depends on narrow contracts/capabilities rather than concrete DB/AI/provider/LangGraph/MCP implementations.
7. The architecture checker deterministically enforces the boundaries that actually ship.
8. `server/api/chat.post.ts` remains a thin transport/composition adapter and `executeChatTurn()` remains H3-independent.
9. Rust auth/admission/trusted-proxy/JWKS/owner/scope and process-sandbox invariants remain fail closed.
10. `scripts/phase4-black-box.sh`, `scripts/phase7-chatgpt-contract.sh`, `scripts/phase8-zero-bypass.sh`, and other applicable deterministic security/contract scripts pass.
11. `pnpm verify:commit` passes completely without substitute typecheck commands.
12. `pnpm build` succeeds; preview/runtime/browser smoke required by touched surfaces is executed and recorded.
13. `pnpm audit` and `cargo audit` are clean at final verification, or any finding has an explicit reviewed disposition.
14. Plan text, source, comments, and verification evidence agree exactly about what shipped and what was run.
15. No unnecessary architecture framework, generic service layer, micro-component explosion, or duplicate security rule was introduced to achieve closure.

Until all of the above are satisfied, **Plan 031A remains open and the branch is not merge-ready for `dev`.**
