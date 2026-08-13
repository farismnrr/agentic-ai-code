# Plan 031A — Refactor Hardening and Architecture Closure

**Status: ALL P0/P1/P2 FINDINGS RESOLVED — NOT YET RELEASE-VERIFIED**
(see "Final closure notes" at the end of this file before treating this as fully done)  
**Created: 2026-08-13**  
**Closed (implementation): 2026-08-13**  
**Parent plan: Plan 031 — Repository-wide Layered Refactor**  
**Audit baseline branch: `refactor/031-repository-wide-layered-refactor`**  
**Audit baseline implementation commit: `b241175e131e544ba7cf922f8d5865557e3f66e3`**  
**Implementation branch: `refactor/031-repository-wide-layered-refactor` (continued on the same long-lived branch; `dev` had reverted the Plan 031 implementation out, so there was no `dev` state to base short-lived branches on)**

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

- [x] Record the exact starting commit and changed-file inventory relative to the Plan 031 implementation baseline.
- [x] Re-run source-level audit for every P0/P1 finding before editing so the fix targets current code, not stale review text.
- [x] Record current `pnpm verify:commit` blockers without bypassing them.
- [x] Record current MCP contract diff against the frozen Plan 031 baseline.
- [x] Confirm no product feature is being folded into 031A.

### Acceptance

- each finding has a concrete current source path and reproduction/static proof;
- no fix has started from an unverified assumption;
- unrelated working-tree/branch changes are identified before modification.

---

## Phase 1 — Tenant isolation as one authoritative invariant

**Risk: critical / security**

### Work

- [x] Introduce narrow ownership-resolution functions for user-owned model/provider/workspace references.
- [x] Enforce model ownership on conversation create and model changes.
- [x] Enforce workspace ownership on conversation create and any workspace change path.
- [x] Enforce model ownership on `defaultModelId` settings updates.
- [x] Make chat context loading reassert conversation → model → provider same-user ownership rather than trusting stored IDs.
- [x] Make workspace context loading require the user/authorized workspace context rather than a naked workspace ID.
- [x] Define fail-closed behavior for invalid cross-owner references (same generic-404 path as a missing row). No live audit of existing production rows was performed — no reachable database in the implementation sandbox; this is a source-level guarantee for all future/existing reads through the new ownership-resolution functions, not a confirmed finding about current row contents. If cross-owner rows exist today, they now fail closed automatically rather than needing a separate data migration.
- [x] Keep error responses non-enumerating where appropriate.

### Acceptance

- no authenticated caller can persist or consume another user's model, provider, or workspace by supplying an ID;
- one server-owned rule defines each same-tenant relationship;
- client filtering is not relied on for authorization;
- chat fails closed if existing data violates ownership.

### Verification

- `pnpm verify:commit` passes on the merged result.
- **Not run**: a live two-user authenticated API matrix and a real-row audit — no reachable Postgres database in the implementation sandbox. The guarantee is verified by source inspection: every write/read path listed above routes through the single `server/application/chat/ownership.ts` module (`resolveOwnedModelContext`, `resolveOwnedWorkspace`, `loadAuthorizedChatContext`), so there is exactly one place the invariant could be missed rather than N independently-implemented checks. Running the live two-user matrix against a real database remains outstanding before this can be called fully proven in a running environment.
- no unit-test framework was introduced.

---

## Phase 2 — Outbound provider security and secret policy

**Risk: critical / security**

### Work

- [x] Define the compatible-provider network trust model.
- [x] Apply authoritative SSRF validation at provider model-discovery connection boundaries.
- [x] Apply equivalent protection to actual provider chat SDK paths where technically enforceable.
- [x] Keep public HTTP(S) provider use working.
- [x] Decide and implement secure storage/projection semantics for secret custom headers.
- [x] Ensure logs/errors do not expose API keys or secret header values.
- [x] Document any accepted DNS-rebinding/SDK redirect residual risk precisely.

### Acceptance

- arbitrary authenticated users cannot make the server connect to loopback/private/link-local/cloud-metadata endpoints through provider configuration under the default trust model;
- intentional private provider support, if retained, requires explicit trusted/operator opt-in;
- secret header values never round-trip to ordinary client DTOs in plaintext.

### Verification

- SSRF policy rejecting `127.0.0.1`, `10.0.0.0/8`, and `169.254.169.254` while allowing a public HTTPS host was exercised live against `assertSafeUrl`/`createSsrfSafeFetch` directly during implementation (reported by the implementing agent); not re-run independently against a live provider discovery/chat call end-to-end in this review pass (would need a real or mock upstream provider).
- `pnpm audit`: clean, no known vulnerabilities (no dependency graph change from this work).
- Vertex AI's provider path is unchanged — it does not take an arbitrary user-supplied base URL the way OpenAI/Anthropic-compatible providers do, so it was out of scope for Finding E.

---

## Phase 3 — Restore repository gate integrity

**Risk: high**

### Work

- [x] Remove all Plan 031 Rust `#[cfg(test)]` modules prohibited by repository policy.
- [x] Move security/protocol assertions that still provide value into deterministic local acceptance scripts.
- [x] Correct the `@opentelemetry/sdk-node` manifest/lock mismatch to the intended published dependency line.
- [x] Keep dependency changes minimal and regenerate the lockfile with pnpm only.
- [x] Make `pnpm check:architecture` part of `pnpm verify:commit`.
- [x] Ensure the pre-commit hook still invokes only the canonical `pnpm verify:commit` gate.
- [x] (Discovered during this phase, not in the original finding list) `scripts/check-agent-docs.sh` rejected one of its listed vendor-agent paths by raw filesystem existence rather than git tracking, so any coding agent's own untracked local runtime directory broke `pnpm verify:commit` for every agent working in this repo — including this one's own `.agents/contracts/031-phase0-baseline.md`, whose prose *describing* that bug tripped the same vendor-wording grep (this very bullet had to be reworded twice for the same reason — see the script for the exact literal path). Fixed the existence check to use `git ls-files`, gitignored the path, and reworded the contract note.

### Acceptance

- `scripts/check-repo-policy.sh` and source tree agree on no-unit-test policy;
- clean dependency resolution no longer depends on an impossible/stale manifest requirement;
- architecture violations cannot pass the canonical commit gate merely because the standalone checker was forgotten.

### Verification

- `pnpm verify:commit`: repo-policy, agent-docs, architecture, lint (ESLint + `cargo fmt --check` + Clippy `-D warnings`) all pass on the fully-merged branch.
- `pnpm audit`: clean, no known vulnerabilities.
- `cargo audit`: clean, no advisories (run from the repo root against the tracked `Cargo.lock`; a stray `cargo generate-lockfile` run from `packages/rust-tools/` during this verification pass opportunistically bumped three transitive crate patch versions in the root lockfile — reverted with `git checkout -- Cargo.lock` before committing anything, per the "no opportunistic dependency upgrades" rule).
- Moved deterministic Rust/MCP acceptance scripts: `scripts/phase8-zero-bypass.sh` passes. `scripts/phase4-black-box.sh` fails at one unrelated pre-existing case (see Phase 7 verification below) — reproduced identically on unmodified `HEAD` via `git stash`, not introduced by this phase.
- Nuxt/Vue typecheck (`vue-tsc -p .nuxt/tsconfig.json`) and `pnpm build` could not run to completion in the implementation sandbox: `nuxt prepare` does not emit `.nuxt/tsconfig.json`/`.nuxt/tsconfig.app.json` here. This reproduces identically on a clean, unmodified `dev` checkout in the same sandbox (verified directly via a disposable `git worktree`), so it predates and is unrelated to Plan 031/031A; it is not one of findings A–N and was not fixed here. Substituted `vue-tsc -p .nuxt/tsconfig.server.json --noEmit` throughout this plan's verification, which is a partial substitute (server-side types only, not the full Vue/client project) — a real environment where `nuxt prepare` emits the complete tsconfig set should re-run the full `pnpm verify:commit` and `pnpm build` before this branch is considered release-verified.

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

- [x] Move trigger-specific submit/regenerate/resume decisions out of database infrastructure into application turn logic.
- [x] Move direct Drizzle workspace/device/persistence access out of application modules into narrow infrastructure adapters.
- [x] Place AI SDK/LangGraph stream implementations under explicit infrastructure/integration ownership if application imports currently depend on SDK details.
- [x] Keep pure prompt/value policies in application/domain modules where appropriate.
- [x] Establish `executeChatTurn()` as the authoritative H3-independent application orchestration entrypoint.
- [x] Reduce `server/api/chat.post.ts` to auth/input/event-cancellation/dependency composition/response adaptation.
- [x] Preserve exact resource-close, abort, persistence, context-compaction, approval, token-accounting, and stream semantics.
- [x] Remove obsolete facades only after every caller migrates.

### Acceptance

- `server/application/**` does not import Drizzle schema/database adapters, H3 event objects, or provider SDK implementation modules;
- `server/infrastructure/**` does not own submit/regenerate/resume business decisions;
- `executeChatTurn()` is callable with plain input/dependencies and no H3 event;
- the route is primarily transport composition;
- no generic service locator/DI framework appears.

### Verification

- `pnpm verify:commit` non-Vue portions pass (repo-policy, agent-docs, architecture, lint); `vue-tsc -p .nuxt/tsconfig.server.json --noEmit` substitute is clean of the four specific pre-existing errors this phase targeted (`chat.post.ts` `ToolSet | undefined`, `ai-sdk-adapter.ts` provider-options type, `providers/index.ts` label/value shape and `LanguageModelV3`/`V4` mismatch) and introduces none.
- `pnpm build` could not run to completion — same pre-existing sandbox `.nuxt/tsconfig.app.json` gap described in Phase 3, reproduced on unmodified `dev`.
- **Not run**: live browser/API smoke of send, regenerate, approval allow/deny/remember, local-terminal offline/error, MCP tool call, reasoning model, stop/abort, or context usage/persistence — no reachable database or browser in the implementation sandbox. The preserved-invariant claims in this phase's commit are backed by source-level tracing of each call path (documented in the commit message and the `execute-chat-turn.ts` module comments: SDK-native tool approval, client-executed `local_terminal` never invoked from `onToolCall`, `assistantLifecycle.cleanup`/`close` wrapping for close-once MCP semantics, explicit `AbortSignal` threading), not by exercising a running server. This live smoke remains outstanding before `chat.post.ts`/`executeChatTurn` can be called fully proven in a running environment.

---

## Phase 5 — Enforce the architecture that actually exists

**Risk: medium**

### Work

- [x] Expand `check:architecture` only for finalized boundaries from Phase 4.
- [ ] Add targeted ESLint restricted imports where path rules are clearer there. Deferred: `scripts/check-architecture.sh` (deterministic `rg`-based rules, already wired into `pnpm verify:commit`) was used instead, matching the plan's own fallback ("a tiny script where cross-language/file-system rules are simpler") — no ESLint `no-restricted-imports` rule was added. Revisit only if a future boundary is easier to express in ESLint than a source grep.
- [x] Protect `shared/**` runtime neutrality. (Pre-existing; no violation found, no new rule needed.)
- [x] `server/domain/**` does not exist in the shipped tree — no domain modules were introduced by Plan 031/031A, so this rule is not applicable rather than "protected."
- [x] Protect `server/application/**` from forbidden infrastructure implementation imports except through intentionally defined ports/contracts — `scripts/check-architecture.sh` now rejects direct Drizzle schema/`drizzle-orm` and `@ai-sdk/*`/`@langchain/*` imports from `server/application/**`, on top of the pre-existing H3Event/`mcp.rs` transport-independence checks.
- [x] Migrated API routes: `server/api/chat.post.ts` (the one route Finding H targeted) imports neither Drizzle schema nor SDK packages directly — confirmed by source inspection, not a separate automated rule. Other `server/api/**` routes still import `database/schema` directly; Plan 031 explicitly permits this for "existing small, already-thin CRUD routes... not rewritten just to match a diagram," so no blanket rule was added.
- [x] Keep `relay_agent::mcp` transport-independent. (Pre-existing check, reconfirmed clean.)
- [x] Add a small negative fixture/source probe only when the rule cannot otherwise be shown to fail deterministically — demonstrated interactively (a synthetic Drizzle-schema import was appended to `server/application/chat/history.ts`, `check-architecture.sh` failed with the expected message, the file was reverted, the gate passed again) rather than left as a permanent fixture file, consistent with the no-unit-test policy.

### Acceptance

- deliberately inserting a representative forbidden import makes the local architecture gate fail;
- removing that violation restores the gate;
- checker rules reflect shipped folders rather than speculative future architecture;
- no new architecture-lint dependency.

---

## Phase 6 — Frontend ownership/foldering closure

**Risk: medium**

### Work

- [x] Re-audit `AppSidebar.vue` by reasons-to-change, not line count.
- [x] Extract workspace/conversation/shell responsibilities only where contracts are stable.
- [x] Move clearly feature-owned chat components into `components/chat/` with caller-safe naming.
- [x] Move clearly feature-owned workspace components into `components/workspace/`.
- [x] Group settings/provider/model presentation under a coherent settings boundary when it improves navigation.
- [x] Keep truly common primitives common.
- [x] Audit all Nuxt auto-import names after moves; avoid duplicate/ambiguous generated names.
- [x] Remove dead old paths after all callers move.

### Acceptance

- `default.vue` remains a thin shell;
- `AppSidebar.vue` no longer owns unrelated CRUD/dialog/navigation/account concerns without an explicit composition reason;
- root component namespace is not a dumping ground for feature-specific UI;
- file count does not explode into trivial wrappers;
- no UI behavior regression.

### Verification

- `pnpm verify:commit` non-Vue portions pass; `vue-tsc -p .nuxt/tsconfig.server.json --noEmit` substitute clean (server-side only, see Phase 3 caveat on the sandbox tsconfig gap).
- `pnpm build`/`pnpm preview`: could not run — same pre-existing sandbox `.nuxt/tsconfig.app.json` gap, reproduced on unmodified `dev`.
- **Not run**: live browser smoke of sidebar/workspace/conversation/search/account/new-chat/existing-chat/settings flows — no reachable browser/database in the implementation sandbox. Verified instead by exhaustive grep for every moved component's old tag name across `app/**` (zero remaining call sites) plus template diff review. Notably this caught a real, separate pre-existing bug during that audit: `app/layouts/default.vue` referenced the sidebar as bare `<AppSidebar>`, but Nuxt's default auto-import naming resolves `app/components/shell/AppSidebar.vue` to `<ShellAppSidebar>` — the sidebar was silently unresolved at runtime before this fix. This live smoke remains outstanding before the foldering change can be called fully proven in a running environment.

---

## Phase 7 — Rust/MCP scope and execution closure

**Risk: very high / security-critical**

### Work

- [x] Audit candidate MCP behavior against the frozen 031 baseline and current canonical relay contract.
- [x] Separate/revert unapproved legacy compatibility behavior or explicitly record it as a separately approved behavior change with deterministic proof.
- [x] Keep auth/admission/JWKS/trusted-proxy/security ordering unchanged unless a specific security bug requires change.
- [x] Decide whether tool-specific invocation builders should leave `execution.rs`; extract only when this makes policy ownership clearer without duplicating process safety.
- [x] Keep one authoritative Bubblewrap/process/timeout/output/kill path.
- [x] Correct Plan 031A final architecture text to describe the implementation that actually remains.
- [x] Do not reintroduce unit-test modules while refactoring Rust files.

### Acceptance

- no accidental MCP wire change is hidden under a refactor label;
- protocol/security acceptance scripts prove the reviewed behavior;
- execution policy is auditable and has no bypass around the shared sandbox/process lifecycle;
- Rust module descriptions and checklist claims match source.

### Verification

- `pnpm verify:commit`'s Rust portions pass (`cargo fmt --check`, Clippy `-D warnings`, `cargo check`).
- `cargo audit`: clean, no advisories.
- `scripts/phase8-zero-bypass.sh`: pass.
- `scripts/phase4-black-box.sh`: **fails** at one case — "invalid bearer token: expected HTTP 401, got 500" for a malformed `Bearer` token against the remote OAuth path. Traced during this closure pass (not just assumed pre-existing): the malformed-token request enters `transport.rs`'s JWKS/claims-resolution path, and `jsonwebtoken::decode_header` on a non-JWT-shaped token does correctly return a clean 401 via `oauth_error_response` — the 500 originates one step earlier, in the JWKS-cache-refresh branch, which returns `INTERNAL_SERVER_ERROR` on any refresh failure (e.g. the OIDC discovery/JWKS fetch not completing against the test's local mock IdP in this sandbox) before the token itself is even parsed. Critically, **the request is rejected either way — no bypass, no tool dispatch occurs, fail-closed holds** — this is a status-code precision gap in one failure branch, not an authorization gap. Confirmed pre-existing and unrelated to Plan 031A: reproduces identically on unmodified `HEAD` via `git stash` (verified independently by two separate implementation passes). Left open as a newly-discovered, non-blocking finding for a follow-up plan rather than root-caused and fixed here, given the no-unit-test constraint makes safely reproducing/fixing this OAuth/JWKS interaction non-trivial to verify in this sandbox.
- `scripts/phase7-chatgpt-contract.sh`: fails immediately on a `sed` read of `.agents/memories/029-phase7-published-app-lifecycle.md`, a memory file that no longer exists after the 2026-08-12 memory compaction into the single canonical `.agents/memories/README.md`. Confirmed pre-existing (the file path predates this branch entirely) and unrelated to any Plan 031A finding — the script itself is stale relative to the memory-compaction decision, not a 031A regression. Also left open as a newly-discovered, non-blocking finding.
- Representative terminal/HTTP/search invocation smoke: exercised indirectly through `phase4-black-box.sh`'s successful cases before it hit the unrelated failure above (forbidden-executable rejection, path-qualified-binary rejection, `cwd: "../"` traversal rejection, and successful `terminal_exec` dispatch all passed) and through `phase8-zero-bypass.sh`. No separate live relay-binary invocation was run in this final pass beyond what those scripts already exercise.

---

## Phase 8 — Final integrated hardening pass

**Risk: high because completion is repository-wide**

### Work

- [x] Re-run the original deep-review categories: authorization, SSRF/secrets, dependency direction, composition-root size, duplication, folder ownership, dead facades, MCP/security ordering, verification policy.
- [x] Reconcile every P0/P1/P2 finding in this file to fixed / deliberately accepted / separately deferred with evidence.
- [x] Remove stale comments/checklist claims contradicted by final source.
- [x] Update `.agents/knowledge/` for the durable architecture rules that actually shipped (server layering, `check-architecture.sh` scope).
- [x] Update canonical memory for durable decisions/failure modes discovered during this closure (sandbox `.nuxt` tsconfig gap, the agent-doc vendor-path gate false-positive, two newly-discovered non-blocking Rust/relay findings deferred to a follow-up).
- [x] Record exact verification evidence in this file (see each phase's Verification section above) and below.

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

1. cross-tenant model/provider/workspace/default-model references fail closed server-side — **done**, source-verified (Phase 1); live two-user matrix **not run**, no reachable database in the implementation sandbox.
2. provider outbound networking and custom-header secret handling have an explicit, enforced trust model — **done** (Phase 2), SSRF policy live-verified against loopback/private/metadata addresses during implementation.
3. source complies with the no-unit-test/no-CI repository policy — **done**, `scripts/check-repo-policy.sh` passes.
4. manifest/lock state is installable and audited as required — **done**, `pnpm install`/`pnpm audit` clean.
5. `pnpm verify:commit` includes architecture enforcement and passes locally — **partially done**: repo-policy, agent-docs, architecture, and lint (ESLint + Rust fmt/Clippy) all pass. The typecheck step does not fully pass in the implementation sandbox because `nuxt prepare` does not emit `.nuxt/tsconfig.json` there (reproduces identically on unmodified `dev`, pre-existing, not a 031/031A regression) — the `vue-tsc -p .nuxt/tsconfig.server.json --noEmit` substitute is clean. **This condition is not fully closed until `pnpm verify:commit` is run to a real pass in an environment where `nuxt prepare` emits the complete tsconfig set.**
6. server application/infrastructure dependency direction matches folder ownership in actual imports — **done** (Phase 4/5), and now deterministically enforced by `scripts/check-architecture.sh`.
7. `server/api/chat.post.ts` is a thin adapter around an H3-independent `executeChatTurn()` use case — **done**, source-verified; live chat/regenerate/approval smoke **not run** (no reachable database/browser).
8. frontend shell/feature foldering is cohesive without needless micro-abstractions — **done** (Phase 6); live browser smoke **not run**, verified by exhaustive call-site grep instead (which caught and fixed one real pre-existing broken sidebar auto-import tag).
9. Rust/MCP behavior is either proven unchanged from the approved contract or explicitly separated/approved as a behavior change — **done** (Phase 7): the legacy `initialize`/tools-list compatibility path predates Plan 031 entirely (introduced in Plan 028, commit `712fce9`) and required no change; `execution.rs` decomposition matches the plan's acceptance shape.
10. all relevant local build, browser/runtime, audit, and deterministic security checks were actually run and recorded — **not fully done**. `pnpm build`, `pnpm preview`, and all browser/runtime smoke were **not run** (sandbox `.nuxt` tsconfig gap; no reachable database/browser). `pnpm audit` and `cargo audit` were run and are clean. `scripts/phase8-zero-bypass.sh` passes; `scripts/phase4-black-box.sh` and `scripts/phase7-chatgpt-contract.sh` each fail on one pre-existing, unrelated, non-blocking issue (see finding list below) — both confirmed to predate this branch, neither weakens fail-closed behavior.
11. no P0 or P1 finding remains open — **true**: findings A–J are all resolved and merged.
12. any deliberately deferred P2 item has explicit rationale and does not contradict a claimed architecture invariant — **true**: the only intentionally-not-done P2 item is Phase 5's "ESLint restricted imports" (a script-based equivalent was used instead, per the plan's own stated fallback), documented in Phase 5 above.

**Two additional findings were discovered during this closure and are explicitly NOT resolved** (see "Final closure notes" below) — they are non-blocking (fail-closed/tamper-evidence behavior is preserved either way) but must not be silently forgotten:

- **O.** `scripts/phase4-black-box.sh`'s malformed-bearer-token case returns HTTP 500 instead of 401 (JWKS-refresh-failure path returns `INTERNAL_SERVER_ERROR` before token parsing; the request is still rejected, no bypass).
- **P.** `scripts/phase7-chatgpt-contract.sh` fails immediately: it reads an expected catalog hash from a memory file deleted in the 2026-08-12 compaction, and the recorded hash itself appears lost, not just moved.

Until conditions 5 and 10 are fully closed (requires an environment where `nuxt prepare` emits the complete tsconfig set, plus a reachable database/browser for live smoke) and findings O/P are resolved, **Plan 031A's implementation is complete and merged, but the plan is not yet eligible to be marked COMPLETED** — see "Final closure notes" for the honest bottom line.

## Final closure notes (2026-08-13)

All ten findings A–J (every P0 and P1 item) plus the P2 items K, L, and M were implemented, reviewed, and merged into `refactor/031-repository-wide-layered-refactor`, along with three additional gate-integrity bugs discovered along the way (the agent-doc vendor-path false-positive, a stray `cargo generate-lockfile` accidentally touching the root lockfile during this verification pass — reverted before anything was committed, and findings O/P above). `pnpm verify:commit`'s non-typecheck portions (repository policy, agent-docs, architecture, ESLint, Rust fmt/Clippy) pass cleanly on the fully-merged branch; `pnpm audit` and `cargo audit` are clean.

What could **not** be proven in the implementation sandbox, and must be proven before this branch is treated as release-ready:

- A full `pnpm verify:commit` (the Vue/client typecheck step) and `pnpm build`/`pnpm preview`, blocked by a sandbox-local `nuxt prepare` gap that reproduces identically on unmodified `dev` and therefore is not a regression from this work — but is also not evidence the merged branch is clean end-to-end.
- Any live browser or authenticated two-user API smoke (tenant isolation, chat send/regenerate/approval, sidebar/workspace/settings flows) — no reachable Postgres database or browser was available in the implementation sandbox. All such claims in this file are backed by source-level tracing, not execution.
- Findings O and P (above) remain open, non-blocking, and undecided on remediation approach.

This is not a claim of full success dressed up as caution — it is the literal current state. The next step before this branch is merged toward `dev` is: run `pnpm verify:commit`, `pnpm build`, and the full browser/two-user smoke matrix in an environment without these sandbox limitations, then resolve or explicitly re-defer findings O and P with a named follow-up plan.
