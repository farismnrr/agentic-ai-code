# Plan 031A — Refactor Hardening and Architecture Closure

**Status: PLANNED / NOT STARTED**  
**Created: 2026-08-13**  
**Parent plan: Plan 031 — Repository-wide Layered Refactor**  
**Audit baseline branch: `refactor/031-repository-wide-layered-refactor`**  
**Audit baseline implementation commit: `b241175e131e544ba7cf922f8d5865557e3f66e3`**

## Why this follow-up exists

Plan 031 completed a large repository-wide refactor pass and materially improved reuse, decomposition, and Rust relay modularity. A strict post-implementation review found that several acceptance criteria were either only partially achieved or were contradicted by the final source tree.

The user explicitly chose to **close Plan 031** rather than keep growing the original plan. This Plan 031A therefore owns every unresolved finding from that review. Closing Plan 031 is an administrative scope boundary; it is **not** evidence that the findings below are already fixed.

This plan is intentionally a hardening/closure effort, not another broad redesign. It must repair concrete security, dependency-direction, verification, and foldering gaps without creating a new architecture framework.

---

## Mission

Bring the Plan 031 implementation to a state where the repository can truthfully claim:

- tenant-owned references are authorized end-to-end;
- user-controlled outbound network configuration cannot silently become server-side SSRF;
- the mandatory local commit gate is internally consistent and actually enforces the architecture rules introduced by the refactor;
- server `application` and `infrastructure` directories reflect real dependency ownership rather than labels only;
- `server/api/chat.post.ts` is a thin transport adapter around one application chat-turn use case;
- frontend feature foldering and the app shell are cohesive without micro-component fragmentation;
- Rust relay refactoring preserves the reviewed MCP/security contract and does not violate the repository's no-unit-test policy;
- Plan/checklist claims match the code that actually ships.

---

## Non-negotiable constraints

1. **No CI.** Do not add GitHub Actions or another remote CI system.
2. **No unit-test suite.** Do not add or retain Rust `#[cfg(test)]` modules, `*.test.*`, `*.spec.*`, test directories, or package `test` scripts. Critical behavior belongs in deterministic local acceptance/security scripts.
3. **Mandatory local gate remains `pnpm verify:commit`.** Never bypass it with `--no-verify` or hook-path changes.
4. **No product redesign.** Preserve current UX/API behavior unless a security fix necessarily rejects previously unsafe input.
5. **No generic repository/service/DI framework.** Use narrow feature-specific functions, ports, and adapters.
6. **No weakening of relay security.** Bubblewrap, non-root execution, explicit execution root, process cleanup, OAuth/JWKS validation, trusted-proxy policy, and SSRF protections remain fail-closed.
7. **No JS executable fallback.** Native terminal/curl/search executable ownership remains Rust.
8. **No schema migration merely for folder symmetry.** Add a migration only if a concrete data-integrity/security constraint warrants it and the migration is explicitly reviewed.
9. **No hidden scope expansion.** MCP compatibility changes, dependency upgrades, or new provider behavior must be separately justified rather than smuggled into structural cleanup.

---

# Audit findings owned by Plan 031A

## P0 — Merge blockers

### A. Cross-tenant model/provider authorization

Current conversation creation/update can persist an arbitrary `modelId`. Chat loading authorizes the conversation itself but then resolves its model/provider by ID without reasserting ownership. Settings can likewise store an arbitrary `defaultModelId`.

Required invariant:

> Every persisted or loaded model reference used for a user's conversation/settings must resolve to a model owned by that same user, and the provider backing that model must belong to the same user.

This must have one authoritative server-side implementation. UI filtering is not authorization.

### B. Cross-tenant workspace authorization

Conversation creation can persist an arbitrary `workspaceId`, and workspace context resolution currently loads by workspace ID without the session user as part of the lookup.

Required invariant:

> Every conversation workspace reference must point to a workspace owned by the conversation/session user before it is stored or used.

Foreign keys alone are insufficient because they prove existence, not same-tenant ownership.

### C. Repository policy contradicts Rust source

`scripts/check-repo-policy.sh` explicitly rejects Rust `#[cfg(test)]`, but Plan 031 introduced/retained unit-test modules in refactored relay modules. Therefore the candidate tree cannot truthfully satisfy the mandatory local gate as written.

Required outcome:

- remove source-level unit-test modules from the Plan 031 implementation;
- preserve important assertions in deterministic acceptance/security scripts where necessary;
- keep the repository policy unchanged unless the user explicitly reverses the no-unit-test decision.

### D. Dependency manifest / lockfile gate inconsistency

The Plan 031 branch records a dependency requirement that is inconsistent with the existing locked dependency line for `@opentelemetry/sdk-node`. The exact manifest/lock state must be corrected before clean install/verification can be treated as evidence.

Required outcome:

- manifest references a published/intended package version line;
- lockfile matches the manifest;
- no unrelated opportunistic dependency upgrades;
- run `pnpm audit` if the dependency graph changes.

---

## P1 — Security and architecture blockers

### E. Provider base-URL SSRF

User-configurable OpenAI/Anthropic-compatible provider base URLs are used for server-side model discovery/chat integration without the same public-network SSRF policy already used by outbound MCP connections.

Required decision and implementation:

- define whether compatible providers are public-network-only or may intentionally target private/self-hosted networks;
- for the default multi-tenant path, validate scheme and resolved addresses at the actual connection boundary and fail closed against loopback/private/link-local/metadata ranges;
- reuse the authoritative SSRF policy where semantics match rather than creating a second address classifier;
- if private provider targets are a supported operator feature, gate them behind an explicit trusted/operator policy rather than arbitrary authenticated-user access;
- consider redirect/DNS-rebinding behavior of the actual provider SDK path; do not overclaim protection that the SDK cannot enforce.

### F. Provider custom-header secret handling

`customHeaders` are stored as plain JSON and projected back to the client while API keys are encrypted/redacted. Custom gateway headers frequently carry credentials.

Required decision:

- classify header values as either intentionally non-secret configuration or secrets;
- if secrets are supported, encrypt/redact them and expose only safe metadata to the client;
- never log decrypted header values.

### G. Server layer ownership is crossed

Current source has infrastructure behavior in `server/application/**` and application turn semantics in `server/infrastructure/database/**`.

Examples to correct:

- application modules must not directly own Drizzle queries/schema access when the concern is persistence;
- database infrastructure must not decide submit/regenerate/resume business semantics;
- AI SDK/LangGraph/provider construction belongs at infrastructure/integration boundaries;
- application orchestration may depend on narrow capabilities, not SDK/database implementation details.

The fix should improve dependency direction, not merely move files again.

### H. Missing authoritative `executeChatTurn` use case

Plan 031 targeted a route shape equivalent to:

```text
HTTP adapter
  -> authenticate + parse
  -> build request-scoped dependencies
  -> executeChatTurn(input, dependencies)
  -> return stream response
```

The current `server/api/chat.post.ts` still owns substantial orchestration. Plan 031A must establish one real application use case that:

- has no H3 event dependency;
- coordinates authorized context/history mutation/compaction/workspace/tools/stream selection/persistence/cleanup;
- receives cancellation and narrow adapters explicitly;
- keeps transport response construction and request event wiring at the HTTP edge;
- keeps Drizzle/provider SDK details out of application policy.

Do not introduce a DI container to achieve this.

### I. Architecture checker is not a mandatory architecture gate

`check:architecture` exists but protects only a small subset of the intended dependencies and is not part of `pnpm verify:commit`.

Required outcome:

- architecture enforcement runs from `pnpm verify:commit`;
- add only deterministic rules for boundaries that have actually been migrated;
- at minimum catch application imports of Drizzle/schema/infrastructure implementations that are forbidden by the final design, domain imports of server/runtime/SDK implementations, and protected Rust protocol-boundary violations;
- use ESLint `no-restricted-imports` where clear and a tiny script where cross-language/file-system rules are simpler;
- avoid a new architecture-lint dependency.

### J. Candidate MCP behavior must be separated from structural refactor claims

The candidate branch includes legacy MCP compatibility behavior (`initialize` / legacy negotiation/tools-list handling) that was not part of the committed Plan 031 baseline behavior being structurally refactored.

Required outcome:

- diff the final candidate against the frozen Plan 031 baseline contract;
- identify every MCP wire-behavior change that came from pre-existing working-tree edits or refactor commits;
- either revert/separate those changes from the structural refactor, or explicitly approve and verify them as a distinct compatibility change;
- do not call a wire behavior change a refactor-only change;
- keep the reviewed `2026-07-28` contract and security ordering authoritative unless a separately reviewed change supersedes it.

---

## P2 — Structural quality and truthfulness

### K. `AppSidebar.vue` remains over-broad

`default.vue` is materially thinner, but much of its former complexity is concentrated in `AppSidebar.vue`.

Review and split only independent reasons-to-change, likely around:

- sidebar navigation/list rendering;
- workspace actions/dialog orchestration;
- conversation actions;
- shell account/search/shortcut composition where still mixed.

Do not create one component per button or a flag-heavy generic sidebar framework.

### L. Frontend feature foldering is incomplete

Feature directories exist for new extracted components, while many older chat/workspace/settings components remain in the root component namespace.

Required outcome:

- group clearly feature-owned components under stable feature folders (`chat/`, `workspace/`, `settings/`, `shell/`) when doing so improves navigation/ownership;
- preserve Nuxt auto-import names/contracts or migrate all callers deliberately;
- leave genuinely cross-feature primitives at the appropriate common level;
- do not move files merely for visual symmetry.

### M. Rust execution acceptance text does not match implementation shape

`execution.rs` still combines tool-specific invocation translation with the shared Bubblewrap/process lifecycle, while Plan 031 text claims a stronger focused-handler decomposition.

Required outcome is one of:

1. keep one authoritative common process runner and extract small tool-specific invocation builders/handlers where that makes security ownership clearer; or
2. explicitly document that the retained dispatcher is intentionally cohesive and correct the acceptance wording.

Do not create a generic arbitrary execution backend or weaken the single process-safety path.

### N. Plan/checklist truth must match source

Any completed checkbox or architecture statement contradicted by source must be corrected in Plan 031A final notes. Completion evidence must distinguish:

- source inspection;
- local static gates;
- deterministic security/contract scripts;
- browser/runtime smoke;
- checks not run or unavailable.

A GitHub mergeable state is never evidence of these checks.

---

# Execution phases

## Phase 0 — Freeze 031A baseline and reproduce blockers

**Risk: low**

### Work

- [ ] Record the exact starting commit and changed-file inventory relative to the Plan 031 implementation baseline.
- [ ] Re-run source-level audit for every P0/P1 finding before editing so the fix targets current code, not stale review text.
- [ ] Record current `pnpm verify:commit` blockers without bypassing them.
- [ ] Record current MCP contract diff against the frozen Plan 031 baseline.
- [ ] Confirm no product feature is being folded into 031A.

### Acceptance

- each finding has a concrete current source path and reproduction/static proof;
- no fix has started from an unverified assumption;
- unrelated working-tree/branch changes are identified before modification.

---

## Phase 1 — Tenant isolation as one authoritative invariant

**Risk: critical / security**

### Work

- [ ] Introduce narrow ownership-resolution functions for user-owned model/provider/workspace references.
- [ ] Enforce model ownership on conversation create and model changes.
- [ ] Enforce workspace ownership on conversation create and any workspace change path.
- [ ] Enforce model ownership on `defaultModelId` settings updates.
- [ ] Make chat context loading reassert conversation → model → provider same-user ownership rather than trusting stored IDs.
- [ ] Make workspace context loading require the user/authorized workspace context rather than a naked workspace ID.
- [ ] Audit existing rows/behavior for invalid cross-owner references and define fail-closed behavior for them.
- [ ] Keep error responses non-enumerating where appropriate.

### Acceptance

- no authenticated caller can persist or consume another user's model, provider, or workspace by supplying an ID;
- one server-owned rule defines each same-tenant relationship;
- client filtering is not relied on for authorization;
- chat fails closed if existing data violates ownership.

### Verification

- `pnpm verify:commit` once policy blockers are removed;
- deterministic API acceptance script or manual authenticated two-user matrix covering create/update/chat/settings cross-owner IDs;
- no unit-test framework.

---

## Phase 2 — Outbound provider security and secret policy

**Risk: critical / security**

### Work

- [ ] Define the compatible-provider network trust model.
- [ ] Apply authoritative SSRF validation at provider model-discovery connection boundaries.
- [ ] Apply equivalent protection to actual provider chat SDK paths where technically enforceable.
- [ ] Keep public HTTP(S) provider use working.
- [ ] Decide and implement secure storage/projection semantics for secret custom headers.
- [ ] Ensure logs/errors do not expose API keys or secret header values.
- [ ] Document any accepted DNS-rebinding/SDK redirect residual risk precisely.

### Acceptance

- arbitrary authenticated users cannot make the server connect to loopback/private/link-local/cloud-metadata endpoints through provider configuration under the default trust model;
- intentional private provider support, if retained, requires explicit trusted/operator opt-in;
- secret header values never round-trip to ordinary client DTOs in plaintext.

### Verification

- deterministic SSRF acceptance script with public allow + loopback/private/link-local rejects;
- provider discovery/chat smoke for each compatible provider path;
- `pnpm audit` if dependencies change.

---

## Phase 3 — Restore repository gate integrity

**Risk: high**

### Work

- [ ] Remove all Plan 031 Rust `#[cfg(test)]` modules prohibited by repository policy.
- [ ] Move security/protocol assertions that still provide value into deterministic local acceptance scripts.
- [ ] Correct the `@opentelemetry/sdk-node` manifest/lock mismatch to the intended published dependency line.
- [ ] Keep dependency changes minimal and regenerate the lockfile with pnpm only.
- [ ] Make `pnpm check:architecture` part of `pnpm verify:commit`.
- [ ] Ensure the pre-commit hook still invokes only the canonical `pnpm verify:commit` gate.

### Acceptance

- `scripts/check-repo-policy.sh` and source tree agree on no-unit-test policy;
- clean dependency resolution no longer depends on an impossible/stale manifest requirement;
- architecture violations cannot pass the canonical commit gate merely because the standalone checker was forgotten.

### Verification

- `pnpm verify:commit`;
- `pnpm audit` for dependency changes;
- `cargo audit` because Rust/security files are touched;
- run the moved deterministic Rust/MCP acceptance scripts.

---

## Phase 4 — Repair server dependency direction

**Risk: high / architecture-sensitive**

### Target direction

```text
server/api (transport)
  -> server/application (use cases/policies)
      -> narrow capability contracts
          <- server/infrastructure (Drizzle / AI SDK / LangGraph / MCP / filesystem)
```

### Work

- [ ] Move trigger-specific submit/regenerate/resume decisions out of database infrastructure into application turn logic.
- [ ] Move direct Drizzle workspace/device/persistence access out of application modules into narrow infrastructure adapters.
- [ ] Place AI SDK/LangGraph stream implementations under explicit infrastructure/integration ownership if application imports currently depend on SDK details.
- [ ] Keep pure prompt/value policies in application/domain modules where appropriate.
- [ ] Establish `executeChatTurn()` as the authoritative H3-independent application orchestration entrypoint.
- [ ] Reduce `server/api/chat.post.ts` to auth/input/event-cancellation/dependency composition/response adaptation.
- [ ] Preserve exact resource-close, abort, persistence, context-compaction, approval, token-accounting, and stream semantics.
- [ ] Remove obsolete facades only after every caller migrates.

### Acceptance

- `server/application/**` does not import Drizzle schema/database adapters, H3 event objects, or provider SDK implementation modules;
- `server/infrastructure/**` does not own submit/regenerate/resume business decisions;
- `executeChatTurn()` is callable with plain input/dependencies and no H3 event;
- the route is primarily transport composition;
- no generic service locator/DI framework appears.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- browser/API smoke: send, regenerate, approval allow/deny/remember, local terminal offline/error, MCP tool call, reasoning model, stop/abort, context usage/persistence.

---

## Phase 5 — Enforce the architecture that actually exists

**Risk: medium**

### Work

- [ ] Expand `check:architecture` only for finalized boundaries from Phase 4.
- [ ] Add targeted ESLint restricted imports where path rules are clearer there.
- [ ] Protect `shared/**` runtime neutrality.
- [ ] Protect `server/domain/**` from H3/Drizzle/provider SDK/filesystem implementations if domain modules exist.
- [ ] Protect `server/application/**` from forbidden infrastructure implementation imports except through intentionally defined ports/contracts.
- [ ] Protect migrated API routes from direct Drizzle schema imports where applicable.
- [ ] Keep `relay_agent::mcp` transport-independent.
- [ ] Add a small negative fixture/source probe only when the rule cannot otherwise be shown to fail deterministically.

### Acceptance

- deliberately inserting a representative forbidden import makes the local architecture gate fail;
- removing that violation restores the gate;
- checker rules reflect shipped folders rather than speculative future architecture;
- no new architecture-lint dependency.

---

## Phase 6 — Frontend ownership/foldering closure

**Risk: medium**

### Work

- [ ] Re-audit `AppSidebar.vue` by reasons-to-change, not line count.
- [ ] Extract workspace/conversation/shell responsibilities only where contracts are stable.
- [ ] Move clearly feature-owned chat components into `components/chat/` with caller-safe naming.
- [ ] Move clearly feature-owned workspace components into `components/workspace/`.
- [ ] Group settings/provider/model presentation under a coherent settings boundary when it improves navigation.
- [ ] Keep truly common primitives common.
- [ ] Audit all Nuxt auto-import names after moves; avoid duplicate/ambiguous generated names.
- [ ] Remove dead old paths after all callers move.

### Acceptance

- `default.vue` remains a thin shell;
- `AppSidebar.vue` no longer owns unrelated CRUD/dialog/navigation/account concerns without an explicit composition reason;
- root component namespace is not a dumping ground for feature-specific UI;
- file count does not explode into trivial wrappers;
- no UI behavior regression.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- `pnpm preview` + browser smoke for sidebar/workspace/conversation/search/account flows;
- new chat/existing chat/settings smoke after auto-import-affecting moves.

---

## Phase 7 — Rust/MCP scope and execution closure

**Risk: very high / security-critical**

### Work

- [ ] Audit candidate MCP behavior against the frozen 031 baseline and current canonical relay contract.
- [ ] Separate/revert unapproved legacy compatibility behavior or explicitly record it as a separately approved behavior change with deterministic proof.
- [ ] Keep auth/admission/JWKS/trusted-proxy/security ordering unchanged unless a specific security bug requires change.
- [ ] Decide whether tool-specific invocation builders should leave `execution.rs`; extract only when this makes policy ownership clearer without duplicating process safety.
- [ ] Keep one authoritative Bubblewrap/process/timeout/output/kill path.
- [ ] Correct Plan 031A final architecture text to describe the implementation that actually remains.
- [ ] Do not reintroduce unit-test modules while refactoring Rust files.

### Acceptance

- no accidental MCP wire change is hidden under a refactor label;
- protocol/security acceptance scripts prove the reviewed behavior;
- execution policy is auditable and has no bypass around the shared sandbox/process lifecycle;
- Rust module descriptions and checklist claims match source.

### Verification

- `pnpm verify:commit`;
- `cargo audit`;
- deterministic MCP black-box/contract/zero-bypass/security scripts;
- representative terminal/http/search invocation smoke inside allowed boundaries.

---

## Phase 8 — Final integrated hardening pass

**Risk: high because completion is repository-wide**

### Work

- [ ] Re-run the original deep-review categories: authorization, SSRF/secrets, dependency direction, composition-root size, duplication, folder ownership, dead facades, MCP/security ordering, verification policy.
- [ ] Reconcile every P0/P1/P2 finding in this file to fixed / deliberately accepted / separately deferred with evidence.
- [ ] Remove stale comments/checklist claims contradicted by final source.
- [ ] Update `.agents/knowledge/` only for durable architecture rules that actually shipped.
- [ ] Update canonical memory only for durable decisions/failure modes; do not add sibling memory files.
- [ ] Record exact verification evidence in the final PR/merge description.

### Final verification

Mandatory minimum:

```sh
pnpm verify:commit
pnpm build
```

Also required for the touched surfaces:

```sh
pnpm audit            # if dependency graph changed
cargo audit            # Rust/security-sensitive work
pnpm check:architecture
```

Run every applicable deterministic MCP/relay/native-tool security/contract script and browser/build-preview smoke described by the relevant phases.

Because this repository has **no CI**, do not mark Plan 031A complete if these commands were not actually run in a real checkout. Connector/GitHub source inspection is useful review evidence but is not local gate evidence.

---

## Definition of done

Plan 031A is complete only when:

1. cross-tenant model/provider/workspace/default-model references fail closed server-side;
2. provider outbound networking and custom-header secret handling have an explicit, enforced trust model;
3. source complies with the no-unit-test/no-CI repository policy;
4. manifest/lock state is installable and audited as required;
5. `pnpm verify:commit` includes architecture enforcement and passes locally;
6. server application/infrastructure dependency direction matches folder ownership in actual imports;
7. `server/api/chat.post.ts` is a thin adapter around an H3-independent `executeChatTurn()` use case;
8. frontend shell/feature foldering is cohesive without needless micro-abstractions;
9. Rust/MCP behavior is either proven unchanged from the approved contract or explicitly separated/approved as a behavior change;
10. all relevant local build, browser/runtime, audit, and deterministic security checks were actually run and recorded;
11. no P0 or P1 finding remains open;
12. any deliberately deferred P2 item has explicit rationale and does not contradict a claimed architecture invariant.

Until all twelve conditions are true, **Plan 031A remains active even if individual refactor commits compile or look cleaner.**
