# Plan 031B — Final Architecture, Security, and Release Closure

**Status: IN PROGRESS — Phase 0 complete; implementation phases not started**
**Created: 2026-08-13**  
**Plan family: Plan 031 — Repository-wide Layered Refactor**  
**Predecessor: Plan 031A — Refactor Hardening and Architecture Closure**  
**Third deep-review baseline branch: `refactor/031-repository-wide-layered-refactor`**  
**Third deep-review baseline commit: `b43f1fe9cc08c2ba6df69f6407f1f37e71bb0e85`**

---

## Why Plan 031B exists

Plan 031 and Plan 031A materially improved the repository, but repeated strict source-level reviews showed that some remaining gaps are systemic enough that continuing to append more remediation phases to Plan 031A would make that plan harder to execute and easier to misread.

The user explicitly chose to create a dedicated follow-up plan for the final closure pass rather than keep growing Plan 031A.

Plan 031B is **not** a cosmetic way to declare Plan 031A complete. It inherits every unresolved acceptance requirement from the third deep review and owns the final repository-wide closure work required before the Plan 031 family can be considered merge-ready.

The target is not a literal mathematical “perfect codebase.” The target is a defensible **10/10 closure standard** for this effort:

- no known P0/P1 security or tenant-isolation finding remains;
- dependency direction matches the documented layered architecture in actual source, not only folder names;
- architecture guardrails reject representative bypasses instead of matching a small set of strings;
- security acceptance scripts prove the behavior they claim to prove;
- current source, comments, plan state, canonical memory, and project guidance agree;
- all mandatory verification that can materially detect regressions is actually executed and green before closure;
- no unnecessary framework, service layer, generic repository, DI container, or micro-abstraction is introduced merely to obtain an architecture score.

---

# Mission

Finish the Plan 031 refactor family with a strict, repository-wide closure pass centered on:

1. **provider network security and credential containment**;
2. **real application/infrastructure dependency inversion**;
3. **repository-wide API/application/infrastructure ownership cleanup**;
4. **architecture enforcement that matches the source-level contract**;
5. **correct and compatible Rust OAuth/JWT pre-validation**;
6. **truthful deterministic acceptance evidence**;
7. **final folder/cohesion audit without architecture astronautics**;
8. **full local/build/runtime/security verification before merge**.

A successful compile, a passing grep, a folder move, or an error that merely fails closed does not by itself satisfy this plan.

---

# Non-negotiable constraints

1. **No CI.** Do not add GitHub Actions or another CI system.
2. **No unit-test suite.** Do not add normal `test/`, `tests/`, `__tests__/`, `*.test.*`, `*.spec.*`, package `test` scripts, or Rust `#[cfg(test)]` modules.
3. Deterministic security/contract acceptance scripts under `scripts/` remain allowed and are preferred for boundaries this repository intentionally verifies without a unit-test suite.
4. Never bypass the local git gate. Do not use `--no-verify`, do not disable `core.hooksPath`, and do not claim `pnpm verify:commit` passed unless it actually completed successfully.
5. No generic `Repository<T>`, generic `CrudService<T>`, service locator, DI framework/container, event bus, speculative plugin architecture, or abstraction introduced without a concrete current use.
6. Prefer **plain feature-local contracts and dependency objects** over framework-level inversion machinery.
7. Do not change client-visible MCP protocol/tool contracts unless a current finding explicitly requires it.
8. Do not weaken tenant isolation, SSRF restrictions, OAuth validation, Bubblewrap containment, timeout/process cleanup, approval behavior, or provider secret handling to make verification easier.
9. No opportunistic dependency upgrades.
10. Keep frontend behavior and Nuxt SSR/composable invariants intact while moving server responsibilities.
11. Preserve one authoritative Rust process-safety execution path for terminal/curl/search relay calls.
12. If a required verification cannot run because the environment is incapable of producing the required artifact, record it as **not proven**. Do not substitute a weaker command and call the requirement complete.

---

# Agent execution protocol

This plan is intentionally detailed so a coding agent can execute it step by step without silently skipping cross-cutting requirements.

## Main-agent responsibilities

The main agent must:

- read `AGENTS.md`, all relevant `.agents/knowledge/`, `.agents/memories/README.md`, Plan 031, Plan 031A, and this plan before implementation;
- treat current source/config as authoritative when plan text conflicts with source history;
- create an implementation branch from the reviewed Plan 031 family branch head rather than touching `dev` directly;
- coordinate workers/sub-agents and keep ownership of cross-cutting decisions;
- review every worker result against this plan before accepting it;
- run or obtain real evidence for each phase exit criterion;
- update this plan truthfully as work proceeds;
- never mark a phase complete merely because a worker says it is complete.

## Recommended worker/sub-agent lanes

Use workers/sub-agents actively, but partition by file ownership to avoid overlapping edits:

### Worker A — provider/network security
Own:

- `server/**/ssrf*`
- provider outbound fetch adapters;
- provider redirect/credential policy;
- Phase 031B deterministic provider-network acceptance script.

### Worker B — server architecture/dependency graph
Own:

- `server/application/**`;
- application-facing contracts/ports;
- `server/infrastructure/**` adapters needed by those ports;
- API route composition changes coordinated with the main agent.

### Worker C — architecture enforcement/folder audit
Own:

- `scripts/check-architecture.sh`;
- any deterministic architecture self-check script;
- server folder ownership inventory;
- stale facade / `server/utils/**` classification audit.

This worker should not design the final boundary independently; it implements guardrails only after Worker B/main agent freezes the desired dependency direction.

### Worker D — Rust auth/relay compatibility
Own:

- `packages/rust-tools/src/relay_agent/auth.rs`;
- relevant auth section of `transport.rs`;
- `scripts/phase4-black-box.sh` auth fixtures/assertions;
- Rust comment truthfulness touched by this plan.

### Worker E — final verification/docs consistency
Own after implementation stabilizes:

- plan/source/comment consistency audit;
- canonical memory/project guidance updates;
- deterministic script inventory;
- final verification evidence collection.

## Worker coordination rule

Workers may audit in parallel, but implementation that changes a shared boundary must be sequenced:

1. security policy and architecture target are frozen first;
2. application contracts and infrastructure adapters are implemented;
3. API callers are migrated;
4. architecture checker is tightened only after the new boundary exists;
5. deterministic acceptance scripts are repaired/expanded against the final implementation;
6. final docs and verification happen last.

Do not let two workers independently invent competing ports, provider redirect semantics, or architecture exceptions.

---

# Third deep-review findings owned by Plan 031B

## P0 — Provider credential containment across redirects

### AA. Cross-origin provider redirects can still forward secrets

At baseline `b43f1fe9`, `createSsrfSafeFetch()` manually follows redirects and re-validates destination addresses, which fixes the earlier private-target redirect SSRF hole. However, when a redirect changes origin, it strips only a small denylist of headers (`authorization`, `cookie`, `proxy-authorization`).

That is not sufficient for this repository because provider credentials are not limited to those names:

- Anthropic-compatible requests use `x-api-key`;
- users may configure arbitrary `customHeaders` that can themselves contain gateway tokens or other credentials;
- future provider SDKs may add additional authentication header names.

Therefore a same-safe-public-network redirect can still leak credentials to another public origin even though the private-address SSRF rule itself is enforced.

### Required policy

Prefer the simplest strong rule:

- authenticated provider requests **must not automatically follow cross-origin redirects**;
- same-origin redirects may be followed only within the bounded redirect count and after validating the next target with the same SSRF address policy;
- HTTPS-to-HTTP downgrade must be rejected, not merely have selected headers stripped;
- provider credential safety must not depend on maintaining a denylist of known secret header names;
- if cross-origin redirect support becomes a real product requirement later, it must use an explicit reviewed allowlist/trust policy rather than implicit forwarding.

### Redirect semantics requirements

If same-origin redirects remain supported:

- preserve standard redirect method/body semantics deliberately rather than accidentally replaying every method/body unchanged;
- account for 301/302/303 vs 307/308 behavior;
- do not replay a non-replayable body stream after a redirect;
- do not silently send a request body where the standard redirect behavior would switch to `GET`;
- re-run URL scheme and DNS/address validation before every actual follow-up connection;
- keep a hard redirect hop bound;
- preserve the documented DNS-rebinding residual risk unless connection IP pinning is deliberately introduced.

### Acceptance

The deterministic provider-network acceptance must prove all of the following:

- initial private/loopback/link-local/metadata targets are rejected;
- mapped/compatible IPv6 representations of blocked IPv4 ranges are rejected;
- a safe-looking initial target whose redirect target resolves private is rejected **before the redirected fetch implementation is invoked**;
- a cross-origin public redirect is rejected before credentials can be forwarded;
- `x-api-key`, `Authorization`, and arbitrary configured secret headers are not observable at an untrusted redirected origin;
- same-origin public redirect behavior succeeds when allowed;
- redirect hop limit is enforced;
- HTTPS downgrade is rejected;
- provider discovery and actual chat/model SDK paths all use the same authoritative provider fetch policy.

---

## P0 — Provider redirect acceptance currently proves the wrong thing

### AB. `phase9-ssrf-redirect-guard.sh` does not exercise a real redirect hop

The current deterministic redirect test starts with a loopback URL. The initial `assertSafeUrl()` rejects that URL before the fixture server can return a redirect. The script therefore reports a redirect test pass without actually traversing the redirect code path.

This is a verification defect because the test name and comments overclaim what was proven.

### Required outcome

Create a deterministic test seam without weakening production behavior. Preferred shape:

- production defaults continue to use real `dns.lookup` and real `fetch`;
- the SSRF fetch constructor may accept narrow injected resolver/fetch capabilities for deterministic acceptance only, or expose an internal policy function that the script can exercise without live internet access;
- fixture resolution can map a fake public hostname to a known public test address for policy purposes while the fetch stub returns controlled redirect responses;
- the script must assert whether the follow-up fetch function was or was not called, so “rejected before connection” is actually demonstrated.

Do not add a production environment variable or hidden bypass that disables the real SSRF policy.

---

# P1 — Real Layered Architecture closure

## AC. Application contracts are still owned by infrastructure

At the third-review baseline, `server/application/chat/execute-chat-turn.ts` imports `ChatTurnDependencies` from `server/infrastructure/ai/chat-turn-dependencies.ts`.

Even though the import is type-only, the dependency direction remains application → infrastructure. The contract is also defined using `typeof` concrete AI/provider/MCP/LangGraph implementations, so application-facing signatures are derived from infrastructure implementation types.

This is not strict dependency inversion.

### Target rule

**Application owns the contracts it needs. Infrastructure implements those contracts.**

Recommended feature-local shape:

```text
server/application/chat/
  contracts.ts
  execute-chat-turn.ts
  history.ts
  ownership.ts
  persistence.ts
  workspace-context.ts
  local-terminal-policy.ts

server/infrastructure/
  database/
  ai/
  mcp/
  filesystem/
  network/

server/api/chat.post.ts
  transport + composition only
```

`server/application/chat/contracts.ts` should contain plain application-facing types/interfaces/capabilities. It must not import:

- `server/infrastructure/**`;
- Drizzle/schema/useDb;
- H3/Nitro event types;
- provider SDKs;
- `ai`, `@ai-sdk/*`, `@langchain/*`, or MCP SDK implementation types.

Use existing shared DTO types such as `UIMessage` when they are genuinely application/shared contracts.

Do not create a global ports framework. Keep contracts feature-local unless another feature truly reuses them.

---

## AD. Application still calls concrete database and AI infrastructure directly

The third review confirmed direct dependencies including:

- `application/chat/history.ts` → `infrastructure/database/chat`;
- `application/chat/persistence.ts` → `infrastructure/database/chat`;
- `application/chat/ownership.ts` → concrete model/provider/chat database modules;
- `application/chat/local-terminal-policy.ts` → concrete infrastructure AI tool builder;
- workspace context reaches a mixed `server/utils/workspaces.ts` module that itself owns DB/filesystem access.

These imports mean current folder names overstate the dependency inversion.

### Required outcome

Move these calls behind application-owned capabilities.

A practical non-generic contract may expose cohesive methods such as:

```text
ChatTurnDataPort
  findOwnedConversationContext(...)
  loadHistory(...)
  insertUserMessage(...)
  findLastMessage(...)
  insertAssistantMessage(...)
  updateAssistantMessage(...)
  cacheLastMeasuredTokens(...)

WorkspaceContextPort
  resolveOwnedWorkspaceContext(...)

DevicePort
  hasActivePairedDevice(...)

ChatModelPort / ChatStreamPort
  resolveModelConfig(...)
  compactMessages(...)
  streamChat(...)
  streamAgent(...)
  create/close tool integrations as needed
```

Exact names may change if a smaller cohesive contract is clearer, but the following rules are mandatory:

- no generic CRUD abstraction;
- no interface per single trivial function unless it improves actual ownership;
- no application type depends on an infrastructure implementation type;
- no application file imports `server/infrastructure/**` after this phase;
- infrastructure adapters may import application contracts to implement them;
- composition happens at the API/composition edge with plain objects/functions.

### Important design guidance

Do not expose SDK-specific `ToolSet`, `ToolApprovalConfiguration`, `LanguageModel`, LangGraph model types, MCP client handles, or concrete Drizzle row types through application contracts merely to make imports compile.

If application does not need to inspect an SDK object, keep that object inside infrastructure and provide a higher-level capability.

Examples:

- application should decide **whether** the turn is chat vs agent and which business policy applies;
- infrastructure should decide **how** AI SDK/LangGraph objects are constructed;
- application should decide approval semantics (`always` / `never` / user approval) as plain values/policy;
- infrastructure should translate that policy into SDK-specific tool approval structures;
- application should coordinate close-once/persistence behavior, while persistence operations are provided through a port;
- application should not receive a raw Drizzle row when a small plain context object is enough.

---

# P1 — Repository-wide API/application/infrastructure ownership audit

## AE. Layering must be checked beyond `chat.post.ts`

Plan 031 was repository-wide. At the third-review baseline, several API routes still directly import Drizzle schema/useDb or mixed `server/utils/**` modules that perform persistence and business logic. `server/application/**` contains only the chat feature, so “Layered Architecture” is not yet a repository-wide source truth.

Plan 031B must perform an explicit route ownership audit rather than assume the chat route is representative.

### Phase requirements

Inventory every file under `server/api/**` and classify it as one of:

1. pure transport/validation/response adaptation;
2. transport + direct persistence;
3. transport + business rule/orchestration;
4. transport + mixed utility that hides persistence/orchestration.

For every route in categories 2–4 that belongs to a user-facing business feature touched by Plan 031 (conversations, workspaces, settings, providers/models, chat, MCP configuration, sidebar aggregation, device/local-terminal policy as applicable), move business/persistence ownership into cohesive application use cases + infrastructure adapters.

### KISS rule for CRUD routes

Do **not** create one class/file/interface per HTTP verb.

Prefer cohesive feature modules, for example:

```text
server/application/conversations.ts
server/application/workspaces.ts
server/application/providers.ts
server/application/settings.ts
```

or feature directories only where the feature is large enough to justify them.

Each module may expose multiple use-case functions. Infrastructure repository modules may similarly group related persistence operations.

### API target

A business API route should read approximately as:

```text
authenticate
validate/parse HTTP input
call application use case with userId + plain input
map framework response/error if needed
return
```

Direct `useDb()`, Drizzle schema imports, provider SDK construction, filesystem access, and tenant authorization policy should not live in API transport files.

### Exception policy

If any direct route-to-infrastructure dependency is intentionally retained because adding an application use case would be objectively ceremony with no business meaning, document the exact exception in the plan and architecture checker with rationale.

The default is **no exception**. Do not create broad exception patterns such as “all database adapters are allowed from application” or “type-only infrastructure imports are always fine.”

---

# P1 — `server/utils/**` ownership cleanup

## AF. Mixed utilities hide layer violations

The third review found `server/utils/**` being used as a mixed dumping ground for several responsibility types:

- persistence-backed workspace logic;
- settings logic with DB access and application ownership calls;
- provider use-case orchestration;
- cryptography;
- SSRF/network policy;
- framework-independent HTTP/domain helpers.

This makes transitive dependency auditing difficult and lets architecture violations hide behind a neutral `utils` path.

### Required audit

Classify every material `server/utils/**` file into:

- **pure/shared server utility** — small framework/server helper with no business/persistence/integration ownership;
- **application policy/use case** — move to `server/application/**`;
- **database/filesystem/network/crypto/provider integration** — move to `server/infrastructure/**`;
- **transport helper** — keep near server/transport usage if appropriate.

Do not move files solely to make the tree look symmetric. Move only when current responsibility has an obvious owner.

### Explicit files to re-evaluate

At minimum inspect:

- `server/utils/workspaces.ts`;
- `server/utils/settings.ts`;
- `server/utils/providers.ts` and provider descendants/facades if any remain;
- `server/utils/crypto.ts`;
- `server/utils/ssrf-guard.ts`;
- context-compaction/provider/MCP facades remaining after 031A;
- any utility imported by `server/application/**` that itself reaches DB, filesystem, AI/provider/MCP SDKs, or Nitro/H3.

### Exit condition

After cleanup, a future reviewer should be able to infer ownership from the path without recursively opening a “utility” facade to discover that it actually performs persistence or external integration.

---

# P1 — Architecture checker must enforce the real final boundary

## AG. Current checker contains loopholes matching current violations

At the third-review baseline, `scripts/check-architecture.sh` still allows:

- type-only imports from `server/application/**` into infrastructure AI/MCP paths;
- application → infrastructure/database imports;
- an explicit `local-terminal-tool` infrastructure exception;
- some transitive infrastructure access patterns by construction.

That means the gate can be green while the intended dependency inversion is still false.

### Final guardrail target

Once Phases AC–AF establish the real architecture, the checker must enforce at least:

#### `server/application/**`

Must not import:

- `server/infrastructure/**`;
- `server/database/**`;
- `drizzle-orm`;
- direct `useDb` implementation paths;
- H3/Nitro request/event types;
- `ai`;
- `@ai-sdk/*`;
- `@langchain/*`;
- provider SDK implementations;
- MCP SDK implementation packages;
- mixed `server/utils/**` facades that transitively own forbidden infrastructure.

#### `server/domain/**` if such a folder exists after final cleanup

Must be stricter than application:

- no DB;
- no framework;
- no AI/provider/MCP SDK;
- no infrastructure;
- no filesystem/network integration.

Do not create `server/domain/**` just to satisfy this checklist.

#### `shared/**`

Must remain runtime-neutral across client/server:

- no server-only imports;
- no Vue/Nitro/provider SDK coupling where the shared contract does not genuinely need it.

#### `server/api/**`

For feature routes migrated under this plan, block direct DB/schema/useDb imports and concrete provider/AI SDK construction.

### Deterministic negative-probe proof

Do not merely run the checker on already-clean source.

Provide deterministic proof that representative violations fail. Preferred approach:

- allow the checker to scan a supplied root/path fixture, or
- create a separate shell acceptance script that constructs temporary fixture files outside tracked source and runs the same pattern logic.

Required negative probes include:

- application → infrastructure type-only import;
- application → infrastructure value import;
- application → Drizzle/schema import;
- application → `ai` value import;
- application → forbidden transitive utility facade;
- migrated API route → database/schema import.

Required positive probes include:

- application → feature-local contract;
- infrastructure → application contract implementation;
- shared pure type import.

Do not mutate tracked source during the normal gate just to prove the checker fails.

---

# P1 — Rust remote OAuth/JWT pre-validation correctness

## AH. Cheap JWT precheck incorrectly requires `typ: JWT`

The 031A fix moved malformed-token rejection before expensive OIDC discovery/JWKS work, which is directionally correct. However, the current `is_structurally_plausible_jwt()` requires the protected header to contain `typ: "JWT"`.

That requirement is too strict for interoperability. A valid signed JWT/JWS access token may omit `typ`; omission must not cause an otherwise valid token to be rejected before full validation.

### Required outcome

Keep cheap rejection without narrowing the accepted valid-token set.

Preferred implementation sequence:

1. ensure the token has exactly three non-empty compact-serialization segments;
2. perform cheap base64url/JSON/header parsing without network access;
3. reuse `jsonwebtoken::decode_header()` where practical rather than maintaining duplicate JWT-header semantics;
4. require only fields actually required by the downstream verifier, such as a supported algorithm and `kid` where this relay's JWKS lookup contract requires it;
5. do **not** require optional `typ`;
6. carry the already-decoded header forward so it is not parsed twice unnecessarily;
7. only then perform OIDC discovery/JWKS cache work;
8. preserve full issuer/audience/signature/time/owner/scope validation unchanged.

### Required black-box coverage

Extend `scripts/phase4-black-box.sh` to prove at least:

- obvious malformed bearer token → 401, no dispatch;
- malformed header/base64 → 401;
- structurally valid token with bad signature → 401;
- valid signed token **with** `typ: JWT` → accepted where authorized;
- valid signed token **without `typ`** → also accepted;
- wrong issuer → rejected;
- wrong audience → rejected;
- wrong owner subject → rejected;
- insufficient scope → rejected;
- unknown `kid` follows the bounded refresh policy and remains fail closed;
- trusted proxy / HTTPS ordering is unchanged.

If the fixture can count discovery/JWKS requests cheaply, assert malformed tokens do not trigger those outbound fixture calls. Otherwise keep the source-level ordering check plus black-box status/non-dispatch proof and document exactly what is and is not measured.

---

# P2 — Rust execution truthfulness

## AI. Execution comment names a helper that does not exist

The Rust execution decomposition is otherwise acceptable: tool-specific invocation preparation feeds one authoritative Bubblewrap/process lifecycle in `dispatch_tool_call()`.

A comment currently claims that lifecycle lives in `run_sandboxed`, but there is no such helper.

Required outcome:

- correct the comment/documentation to describe the actual single shared process-safety path;
- do **not** extract a wrapper solely to make old prose true;
- preserve Bubblewrap mounts, explicit execution root, environment clearing, safe PATH, process-group handling, output limits, timeout grace/kill behavior, and sibling-binary resolution.

---

# P1/P2 — Final tenant and secret boundary re-audit

## AJ. Re-verify all user-owned references after architectural moves

Previous 031A passes materially fixed tenant ownership for model/provider/workspace/default-model/active-workspace/chat context. Moving those operations behind new application contracts can accidentally reintroduce trust in stored/client IDs.

After the architecture refactor, perform a fresh ownership matrix over every user-controlled or persisted reference touched by this family:

- conversation → user;
- conversation → model;
- model → provider;
- conversation → workspace;
- settings → default model;
- user → last active workspace;
- MCP server configuration → user;
- local paired device access → user;
- provider model discovery → provider owner.

Acceptance:

- no route relies on frontend filtering as authorization;
- foreign IDs fail using non-enumerating semantics where appropriate;
- chat context reasserts model/provider ownership even for legacy/bad stored rows;
- architecture moves do not turn authoritative ownership checks into duplicated per-route checks.

## AK. Re-verify provider secret lifecycle

Preserve and re-check:

- API keys encrypted at rest;
- custom provider headers encrypted/redacted;
- legacy plaintext custom headers still have a safe idempotent upgrade path;
- ordinary provider DTOs expose only key names / presence metadata, never decrypted values;
- editing unrelated provider fields does not clear existing secret headers;
- logs/errors do not include decrypted API keys or custom header values;
- corrupt encrypted values fail safely;
- redirect behavior cannot forward secrets outside the trusted provider origin.

Do not redesign encryption format unless a real blocker is found during this re-audit.

---

# P2 — Frontend/foldering final audit

## AL. Verify, do not churn, the frontend decomposition

The third review found the frontend materially improved:

- `app/components/chat/`;
- `app/components/workspace/`;
- `app/components/settings/`;
- `app/components/shell/`;
- a much thinner `default.vue`;
- `AppSidebar` responsibilities reduced relative to the pre-031 implementation.

Plan 031B should audit this area for regressions but should **not** manufacture extra components merely to increase a structure score.

Acceptance:

- root-level components are genuinely cross-feature/landing/shared primitives;
- feature-specific components remain under the correct feature directory;
- `AppSidebar` has no newly re-accumulated unrelated settings/provider/chat orchestration;
- reusable chat/settings/workspace presentation logic is not duplicated;
- Nuxt component auto-import names remain stable;
- no SSR/composable-context regression is introduced by server refactoring.

Only refactor frontend files if the audit identifies an actual cohesion/duplication problem.

---

# P2 — Documentation and source truth

## AM. Remove stale “031A closed architecture” claims and stale open findings

At the third-review baseline, repository guidance contains statements that no longer match current truth, including:

- project guidance describing architecture boundaries as “closed by Plan 031A” while Plan 031A is still open and source still violates strict direction;
- canonical memory entries describing Phase 4 and Phase 7 issues as still-open findings even though 031A implemented fixes;
- Plan 031A Phase 9–12 checkboxes marked complete even though the third review found Q/S/T/V follow-up deficiencies;
- Rust comments that overstate decomposition.

Required outcome after implementation:

- Plan 031A becomes a truthful historical predecessor with an explicit handoff to Plan 031B;
- Plan 031B becomes the only active plan in the 031 family;
- `.agents/memories/README.md` records durable current decisions, not stale audit chronology;
- `.agents/knowledge/project.md` describes the architecture that actually ships after 031B;
- architecture-checker description matches its real checks;
- verification evidence is tied to the exact final commit where practical;
- no doc says “closed”, “green”, “verified”, or “10/10” for a requirement that was not actually proven.

---

# Execution phases

## Phase 0 — Freeze baseline and build a gap matrix

**Risk: high because later phases depend on accurate ownership**

### Steps

- [x] Confirm implementation starts from the exact intended parent branch/head after this plan is committed.
- [x] Create a dedicated 031B implementation branch from that head; do not touch `dev` directly.
- [x] Read current `AGENTS.md`, `.agents/knowledge/`, canonical memory, Plan 031, Plan 031A, and Plan 031B.
- [x] Re-run a source inventory of `server/api/**`, `server/application/**`, `server/infrastructure/**`, `server/utils/**`, provider adapters, and relay auth files.
- [x] Build a written dependency/ownership matrix before moving files.
- [x] Mark each third-review finding AA–AM as confirmed, superseded, or already resolved by a newer commit, with source evidence.
- [x] Record current verification state; do not inherit old green results as evidence for the new branch.

### Exit criteria

- every file to be moved/refactored has one intended owner;
- no worker is assigned overlapping authority without main-agent coordination;
- no checkbox in later phases is pre-checked from assumption.

The completed source inventory and finding disposition are recorded in
[`.agents/contracts/031b-gap-matrix.md`](../contracts/031b-gap-matrix.md).

---

## Phase 1 — Close provider redirect credential leakage

**Risk: critical / security**

### Steps

- [x] Define and document the provider redirect trust rule before editing code.
- [x] Reject cross-origin provider redirects by default.
- [x] Reject HTTPS → HTTP downgrade redirects.
- [x] Revalidate every same-origin redirect target with the authoritative SSRF URL/address policy before connecting.
- [x] Preserve the bounded redirect count.
- [x] Implement deliberate 301/302/303/307/308 method/body semantics or explicitly reject unsupported replay cases.
- [x] Ensure the policy is used by OpenAI-compatible discovery + chat SDK paths.
- [x] Ensure the policy is used by Anthropic-compatible discovery + chat SDK paths.
- [x] Ensure LangGraph/OpenAI/Anthropic client hooks use the same policy.
- [x] Confirm Vertex path is unaffected unless it actually uses user-controlled base URLs.
- [x] Do not leak `Authorization`, `x-api-key`, arbitrary custom headers, cookies, or future unknown secret headers to another origin.

### Acceptance script work

- [x] Replace the false redirect proof in `phase9-ssrf-redirect-guard.sh` with a deterministic resolver/fetch fixture that actually enters the redirect branch.
- [x] Assert private redirect target is rejected before follow-up fetch.
- [x] Assert cross-origin public redirect is rejected.
- [x] Assert same-origin allowed redirect succeeds.
- [x] Assert redirect loop/hop exhaustion fails.
- [x] Assert mapped IPv4/IPv6 blocked ranges.
- [x] Assert HTTPS downgrade fails.
- [x] Assert secret headers are never presented to an untrusted redirected request fixture.

### Exit criteria

Finding AA and AB are closed only if code and deterministic acceptance agree.

---

## Phase 2 — Freeze application-facing contracts

**Risk: high / architecture**

### Steps

- [x] Define the minimal application-owned contracts needed by chat orchestration.
- [x] Move `ChatTurnDependencies` ownership out of infrastructure.
- [x] Remove `typeof concreteImplementation`-derived signatures from application contracts.
- [x] Replace concrete SDK object exposure with application-level inputs/results where application does not need SDK internals.
- [x] Decide the smallest cohesive data/persistence capability shape for history, ownership, persistence, workspace, and device access.
- [x] Keep contracts feature-local; do not build a global ports framework.
- [x] Main agent reviews the contract before infrastructure/API migration begins.

### Contract review questions

For every contract method ask:

- does application actually need to know this value/type?
- is the type plain and stable, or copied from an SDK/Drizzle implementation?
- would moving this operation behind the port hide infrastructure without hiding business semantics?
- are two methods actually one cohesive operation, or are we creating ceremony?
- would a future infrastructure implementation be possible without importing the original concrete implementation type?

### Exit criteria

- `server/application/**` can compile conceptually using only application/shared types and capabilities;
- no contract requires infrastructure imports to define its public shape.

---

## Phase 3 — Migrate chat application off concrete infrastructure

**Risk: high / regression-prone**

### Steps

- [x] `execute-chat-turn.ts` imports only application/shared contracts and sibling application policies.
- [x] `history.ts` no longer imports database infrastructure directly.
- [x] `persistence.ts` no longer imports database infrastructure directly.
- [x] `ownership.ts` no longer imports concrete model/provider/chat repositories directly.
- [x] `workspace-context.ts` no longer reaches a mixed DB/filesystem utility directly.
- [x] `local-terminal-policy.ts` no longer imports an infrastructure AI tool builder directly.
- [x] Concrete database/AI/MCP/filesystem/device adapters implement the application contracts.
- [x] `chat.post.ts` remains auth + HTTP parsing + abort wiring + composition + response only.

### Preserve these chat invariants

- [x] submit-message inserts one user message with authoritative generated ID;
- [x] regenerate removes only the appropriate trailing assistant context;
- [x] resume/continuation behavior remains unchanged;
- [x] context compaction cutoff/token accounting remains correct;
- [x] workspace prompt is tenant-scoped;
- [x] chat vs agent mode selection remains application-owned;
- [x] local terminal remains client-executed and only available according to paired-device/approval policy;
- [x] MCP close runs exactly once across success/error/abort paths;
- [x] assistant persistence remains resilient and logged on failure;
- [x] stop/abort propagates correctly;
- [x] reasoning/provider options remain correct;
- [x] tool approval allow/deny/user-approval semantics remain unchanged.

### Exit criteria

No source file under `server/application/chat/**` imports `server/infrastructure/**`, DB/schema/Drizzle, H3/Nitro, or concrete AI/provider/MCP SDK packages.

---

## Phase 4 — Repository-wide server layering closure

**Risk: high / broad but bounded**

### Steps

- [x] Inventory all `server/api/**` direct DB/schema/useDb imports.
- [x] Inventory all API routes that import mixed `server/utils/**` business/persistence modules.
- [x] Group routes by feature rather than HTTP verb.
- [x] Introduce cohesive application feature modules only where needed.
- [x] Move persistence into existing/new infrastructure database modules.
- [x] Move provider/network/filesystem integration to infrastructure owners.
- [x] Keep validation/auth/session/H3 adaptation in API routes.
- [x] Centralize tenant ownership in application/use-case rules rather than per-route duplicates.

### Mandatory features to audit

- [x] conversations;
- [x] models;
- [x] providers;
- [x] workspaces;
- [x] active workspace;
- [x] settings/default model;
- [x] sidebar aggregate data;
- [x] MCP configuration;
- [x] paired/local-terminal device lookups;
- [x] chat.

Verification note: worker capacity was temporarily exhausted during Phase 3
orchestration; completed worker threads were reclaimed and Phase 3 then passed
an independent worker audit before Phase 4 continued.

### KISS exit test

Before accepting each new module, ensure it is not merely:

```text
route -> one-line application wrapper -> one-line repository wrapper -> useDb
```

without any meaningful ownership benefit.

Prefer feature-level cohesive functions and reuse authoritative policy.

### Exit criteria

The repository's claimed API → application → infrastructure direction is true for the Plan 031 business surface, not only for `chat.post.ts`.

---

## Phase 5 — Eliminate mixed utility ownership

**Risk: medium**

### Steps

- [ ] Classify every material `server/utils/**` file.
- [ ] Move persistence-backed utilities to infrastructure/application owners.
- [ ] Move provider SDK/network integration to infrastructure.
- [ ] Move cryptography and SSRF policy to a clearly named infrastructure/security/network location if that improves ownership after Phase 1.
- [ ] Keep genuinely small pure server helpers in `server/utils/**`.
- [ ] Delete obsolete facades/re-exports created only to preserve old paths once all callers are migrated.
- [ ] Search for duplicate ownership/security helpers after moves.

### Exit criteria

- no application dependency reaches infrastructure by hiding behind `server/utils/**`;
- `server/utils/**` is no longer a dumping ground for DB/provider/filesystem business code;
- no unnecessary file churn beyond ownership improvements.

---

## Phase 6 — Rebuild architecture enforcement against the final tree

**Risk: high because false-green gates are dangerous**

### Steps

- [ ] Simplify `check-architecture.sh` around the final actual rules.
- [ ] Remove temporary exceptions that existed only for pre-031B violations.
- [ ] Block all application → infrastructure imports, including type-only imports.
- [ ] Block application DB/schema/Drizzle/H3/Nitro/AI/provider/MCP implementation imports.
- [ ] Block migrated API routes from direct DB/schema access.
- [ ] Keep Rust MCP transport-independence check.
- [ ] Add shared/domain checks only where folders/contracts really exist.
- [ ] Add deterministic negative/positive fixture acceptance without modifying tracked production source.
- [ ] Keep `pnpm check:architecture` inside `pnpm verify:commit`.
- [ ] Do not add a new architecture-lint dependency unless grep/shell enforcement demonstrably cannot express the final rule safely.

### Exit criteria

The architecture checker fails on representative forbidden direct, type-only, and facade-style imports and passes on the intended clean dependency graph.

---

## Phase 7 — Fix JWT pre-validation without narrowing valid tokens

**Risk: high / auth compatibility**

### Steps

- [ ] Remove mandatory `typ: JWT` behavior.
- [ ] Reuse decoded JWT header across the auth path instead of parsing twice where possible.
- [ ] Keep cheap malformed-token rejection before discovery/JWKS work.
- [ ] Preserve supported algorithm checks.
- [ ] Preserve required `kid` semantics.
- [ ] Preserve JWKS cache TTL and single refresh-on-unknown-kid behavior.
- [ ] Preserve issuer/audience/time/signature validation.
- [ ] Preserve owner subject and scope enforcement.
- [ ] Preserve admission-before-expensive-auth ordering.
- [ ] Preserve trusted-proxy HTTPS policy.

### Phase 4 black-box updates

- [ ] valid token with `typ` passes;
- [ ] valid token without `typ` passes;
- [ ] malformed bearer returns intended 401;
- [ ] malformed header/payload/signature cases fail closed;
- [ ] wrong issuer/audience/owner/scope fail;
- [ ] unknown kid behavior remains bounded;
- [ ] fixture discovery/JWKS endpoints remain deterministic.

### Exit criteria

Finding AH is closed with both source-ordering review and black-box compatibility evidence.

---

## Phase 8 — Tenant/security regression matrix after architecture moves

**Risk: critical because refactoring authorization can reintroduce BOLA**

### Steps

Perform a two-user matrix with User A and User B where possible.

- [ ] A cannot create conversation with B's model.
- [ ] A cannot update conversation to B's model.
- [ ] A cannot create/use conversation with B's workspace.
- [ ] A cannot persist B's workspace as active workspace.
- [ ] A cannot persist B's model as default model.
- [ ] chat turn for A cannot resolve B's provider through a corrupt/legacy model reference.
- [ ] provider model-list endpoint is owner-scoped.
- [ ] provider DTO never exposes API key or custom header values.
- [ ] legacy custom headers remain usable/upgraded.
- [ ] provider secret edit semantics preserve unchanged secrets.
- [ ] MCP configs and local device lookups remain user-scoped.

Where full DB/runtime execution is unavailable, retain deterministic/source verification but leave the live item unchecked until it is actually run.

---

## Phase 9 — Frontend/cohesion and Rust execution final audit

**Risk: medium / avoid churn**

### Frontend

- [ ] confirm feature component folders remain coherent;
- [ ] confirm root components are genuinely shared/landing primitives;
- [ ] confirm no new duplicate collection/state logic;
- [ ] confirm sidebar responsibilities did not re-expand;
- [ ] do not split components solely by line count.

### Rust execution

- [ ] fix the stale `run_sandboxed` comment/reference;
- [ ] confirm all tool invocation builders feed the same Bubblewrap/process path;
- [ ] confirm no direct process-spawn bypass exists;
- [ ] confirm bwrap requirement, execution root, env clearing, safe PATH, output bounds, timeout and process-group cleanup remain intact;
- [ ] do not introduce Rust unit tests.

### Exit criteria

No known cohesion/foldering regression remains, and no cosmetic refactor was introduced solely for scoring.

---

## Phase 10 — Deterministic acceptance sweep

**Risk: high / verification truth**

Run and record every applicable deterministic script. At minimum inspect/run:

- [ ] `scripts/check-repo-policy.sh`;
- [ ] `scripts/check-agent-docs.sh`;
- [ ] `scripts/check-architecture.sh`;
- [ ] architecture negative/positive acceptance added by 031B;
- [ ] provider SSRF/redirect acceptance added/repaired by 031B;
- [ ] `scripts/phase4-black-box.sh`;
- [ ] `scripts/phase6-chatgpt-e2e.sh` applicable deterministic/static portion;
- [ ] `scripts/phase7-chatgpt-contract.sh`;
- [ ] `scripts/phase8-zero-bypass.sh`;
- [ ] any other existing Rust/native security acceptance script affected by touched code.

Requirements:

- scripts must test the behavior their names/comments claim;
- no script may pass because the request is rejected earlier than the branch being asserted unless that early rejection is the explicit test target;
- source-string checks may complement but not substitute black-box behavior when black-box deterministic verification is practical.

---

## Phase 11 — Full commit/build/audit verification

**Risk: blocking**

Run from a real checkout/environment capable of generating the complete Nuxt project:

```sh
pnpm verify:commit
pnpm build
pnpm check:architecture
pnpm audit
cargo audit
```

Also record, as applicable:

```sh
cargo fmt --all -- --check
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Do not count server-only `.nuxt/tsconfig.server.json` type checking as completion of `pnpm verify:commit`.

If the known sandbox still cannot emit `.nuxt/tsconfig.json`, run partial checks there for feedback, but final closure must happen in an environment where the canonical gate and production build actually complete.

---

## Phase 12 — Runtime/browser acceptance

**Risk: blocking / catches integration bugs static checks miss**

Using a clean build/preview where possible, verify:

### Authenticated application flows

- [ ] login/session remains functional;
- [ ] sidebar/workspace data loads under authenticated SSR/client navigation;
- [ ] create/rename/delete workspace;
- [ ] active workspace switching;
- [ ] create/rename/delete conversation;
- [ ] settings load/save;
- [ ] provider create/edit/delete/model discovery;
- [ ] custom header add/replace/delete without secret round-trip;
- [ ] model create/select/default model.

### Chat flows

- [ ] send;
- [ ] regenerate;
- [ ] stop/abort;
- [ ] continuation/resume if supported by current UI flow;
- [ ] chat mode;
- [ ] agent mode;
- [ ] approval allow;
- [ ] approval deny;
- [ ] remembered approval behavior;
- [ ] MCP tool call;
- [ ] MCP cleanup/close path;
- [ ] local terminal paired path if environment supports it;
- [ ] local terminal offline/error path;
- [ ] reasoning/provider variants that are actually configured.

### Provider network/security flow

- [ ] normal public provider target works;
- [ ] initial private target rejected;
- [ ] public-to-private redirect rejected;
- [ ] cross-origin redirect does not receive credentials;
- [ ] allowed same-origin redirect works if product behavior supports it.

Do not invent external-provider results if credentials/environment are unavailable. Mark unavailable cases explicitly.

---

## Phase 13 — Final source/dependency/folder re-review

**Risk: blocking because this is the “10/10” audit pass**

After all fixes and verification, perform a fresh review **from the final source**, not from checklist assumptions.

### Required review lenses

- [ ] DRY — no duplicated ownership/SSRF/provider/collection policy;
- [ ] SOLID — responsibilities and dependency inversion are real;
- [ ] KISS — no needless abstraction/framework ceremony;
- [ ] Layered Architecture — imports and runtime composition follow intended direction;
- [ ] reusable components/logic — feature reuse without generic mega-abstractions;
- [ ] folder structure — path matches responsibility;
- [ ] tenant isolation — every user-owned reference is server-authorized;
- [ ] secret handling — storage, DTOs, logs, redirects;
- [ ] SSRF — initial + redirect destinations and credential containment;
- [ ] Rust relay security — auth/admission/transport/sandbox ordering;
- [ ] deterministic verification quality — scripts actually exercise claimed path;
- [ ] dependency manifest/lock consistency;
- [ ] no CI/no unit-test policy;
- [ ] dead facades/re-exports/stale comments;
- [ ] source/docs/plan/memory truthfulness.

### Required output

Record findings in this plan. If a new material P0/P1 issue is discovered, **do not close Plan 031B**. Add it to this plan and fix it unless the user explicitly changes scope.

Do not create Plan 031C simply to move a newly discovered unfinished 031B blocker elsewhere.

---

## Phase 14 — Documentation, memory, and closure

Only after Phase 13 returns no unresolved P0/P1 issue:

- [ ] update `.agents/knowledge/project.md` to the architecture that actually ships;
- [ ] update `.agents/memories/README.md` to remove stale fixed findings and record durable 031B decisions;
- [ ] ensure Plan 031A has a truthful administrative handoff to Plan 031B;
- [ ] update Plan 031B checkboxes based on actual evidence;
- [ ] record final verification commands/results and final commit SHA;
- [ ] mark Plan 031B closed only after all mandatory evidence is present;
- [ ] keep next independent numeric plan number at `032`.

---

# Final Definition of Done

Plan 031B is complete only when **all** of the following are true:

1. No known P0/P1 security finding from the Plan 031/031A/031B reviews remains unresolved.
2. Provider requests cannot forward credentials to an untrusted cross-origin redirect target.
3. Provider SSRF enforcement validates every followed target and deterministic acceptance actually traverses the redirect decision path.
4. HTTPS downgrade behavior is explicitly safe and redirect count/method/body semantics are deliberate.
5. Tenant ownership remains authoritative for conversation/model/provider/workspace/default-model/active-workspace/MCP/device references touched by this refactor family.
6. Legacy and new provider secrets remain usable, encrypted/redacted, and non-leaking.
7. `server/application/**` has **zero** imports from `server/infrastructure/**` and zero direct DB/Drizzle/H3/provider/AI/MCP implementation dependencies.
8. Application-facing contracts are owned by application (or an equally inner plain contract layer), not derived from concrete infrastructure implementation types.
9. Infrastructure implements application contracts; API routes compose and call use cases rather than owning business/persistence logic.
10. The Plan 031 server business surface has been audited repository-wide; layering truth is not based only on `chat.post.ts`.
11. Mixed `server/utils/**` facades no longer hide database/provider/filesystem/network business ownership.
12. Architecture enforcement blocks representative direct, type-only, transitive-facade, and API-bypass violations while allowing legitimate dependencies.
13. JWT cheap rejection happens before expensive auth work without requiring optional `typ: JWT` or otherwise rejecting valid tokens that full verification accepts.
14. Phase 4 deterministic auth acceptance includes a valid token without `typ` and remains green.
15. Rust execution continues through one authoritative Bubblewrap/process-safety path with comments matching source.
16. Frontend feature foldering/reusability remains coherent and no micro-component churn was introduced.
17. `pnpm verify:commit` passes completely on the final implementation commit without bypass or substitute client typecheck.
18. `pnpm build` succeeds on the final implementation commit.
19. `pnpm audit` and `cargo audit` are clean, or any finding has an explicit reviewed disposition accepted by the user.
20. All applicable deterministic security/contract scripts pass and have been reviewed for false-positive/false-proof behavior.
21. Required runtime/browser flows have been executed where an environment exists; unavailable external-provider/device cases are explicitly marked unproven rather than fabricated.
22. A final fresh source-level review finds no known material P0/P1 architecture, tenant-isolation, credential, SSRF, or relay-security blocker.
23. Plan text, `.agents/knowledge/`, canonical memory, comments, source, and verification evidence tell the same story.
24. No CI, unit-test suite, generic repository/service layer, DI framework, service locator, or speculative abstraction was introduced.
25. The branch is ready to be integrated into `dev` under the repository's normal implementation workflow.

Until every required item above is satisfied, **Plan 031B remains open and the Plan 031 implementation family is not merge-ready for `dev`.**
