# Plan 031 — Repository-wide Layered Refactor

**Status: PHASE 0 COMPLETE / IMPLEMENTATION IN PROGRESS**
**Created: 2026-08-12**  
**Baseline branch: `dev`**  
**Baseline commit: `46b926ffd103b2ec50055eb11b6a824b13642b1e`**

## Mission

Refactor the repository structure and implementation without redesigning product behavior. The resulting codebase must make **DRY, SOLID, KISS, and Layered Architecture** enforceable day-to-day, with **reusable components and reusable logic as a first-class priority**.

This is a structural refactor, not a framework rewrite. Existing user-visible behavior, security boundaries, persistence contracts, MCP contracts, and native-tool ownership are preserved unless a separate product/security change is explicitly approved.

The intended outcome is not “more abstractions.” It is **fewer duplicated rules, smaller reasons-to-change, clearer dependency direction, simpler composition roots, and reusable units with stable contracts**.

---

## Non-negotiable principles

### 1. DRY — one source of truth for one rule

Extract duplicated **knowledge**, not merely duplicated syntax.

Mandatory:

- shared product/domain constants live in one authoritative place;
- repeated model/mode/reasoning/tool option definitions are centralized when they represent the same concept;
- repeated validation/policy rules are centralized when server endpoints and application services must enforce the same invariant;
- repeated state mutation/loading/error patterns are extracted only when their lifecycle semantics truly match;
- repeated UI chrome becomes a reusable component only when props/events/interaction semantics are genuinely the same.

Forbidden:

- copying a business rule into a page, composable, API route, and persistence helper;
- extracting coincidentally similar UI into a mega-component that needs many flags/slots to support unrelated behavior;
- generic `Repository<T>`, `CrudService<T>`, or form-builder abstractions solely to remove a few lines of obvious code;
- “DRY” helpers that hide important control flow or security decisions.

Rule of thumb: **duplicate source-of-truth = extract immediately; duplicate shape = extract only when behavior and change cadence are shared.**

### 2. SOLID — pragmatic, composition-first

SOLID is applied to functions/modules/composables as well as types; this plan does not force class-heavy patterns.

- **SRP:** one module should have one primary reason to change. Large orchestration files are composition roots, not homes for persistence, policy, transport, and UI details simultaneously.
- **OCP:** real extension seams (model providers, MCP/native tools, execution backends) should accept new implementations without editing an ever-growing central branch/switch when practical.
- **LSP:** adapters implementing the same contract must preserve result, error, timeout, cancellation, ownership, and security semantics expected by their caller.
- **ISP:** prefer narrow ports/functions (`loadConversationForTurn`, `persistAssistantMessage`, `executeTool`) over mega-context/service interfaces.
- **DIP:** application orchestration depends on narrow capabilities/policies; Drizzle, H3/Nitro, AI SDK, LangGraph, filesystem, OAuth/JWKS, and subprocess details remain at infrastructure/transport boundaries.

### 3. KISS — simplest design that protects the boundary

Mandatory defaults:

- pure function before stateful object;
- composable before framework-agnostic service when the concern is genuinely Vue/Nuxt reactive state;
- module/factory before class;
- explicit imports and explicit orchestration before service locator/DI container;
- stable facade before broad call-site rewrite;
- small dependency object before global registry;
- current Nuxt/Rust idioms before custom framework machinery.

Do **not** introduce:

- a dependency-injection framework;
- event bus for ordinary parent/child or composable communication;
- speculative plugin architecture;
- inheritance hierarchies for UI or providers;
- one-file-per-function fragmentation;
- a new state-management library when `useState`/Vue state is sufficient;
- Nuxt Layers merely to satisfy the word “layered”; layers are a deployment/composition feature, not a substitute for clean dependencies.

### 4. Layered Architecture — dependency direction matters more than folders

A folder move is not an architectural improvement unless dependency direction becomes clearer.

Target direction:

```text
Presentation / Transport
        ↓
Application / Use Cases
        ↓
Domain Policies / Contracts
        ↓ through narrow ports
Infrastructure / Integrations
```

Infrastructure may implement capabilities required by application code. Domain policy must not depend on HTTP events, Vue components, Drizzle tables, provider SDKs, OAuth/JWKS clients, subprocesses, or filesystem implementations.

Cross-cutting observability may wrap boundaries, but must not become a backdoor that couples every layer to infrastructure-specific state.

---

## Current evidence and priority hotspots

The plan starts from current source, not historical checklists.

### Frontend hotspots

| Area | Current mixed responsibilities | Refactor intent |
| --- | --- | --- |
| `app/layouts/default.vue` | SSR app-data orchestration, workspace synchronization, sidebar grouping, navigation mapping, conversation actions, workspace CRUD actions, user menu, search, shortcuts, several modal states, large shell template | thin app shell + feature-focused sidebar/search/dialog components + one shell controller composable |
| `app/pages/chat/[id].vue` | loading, chat orchestration wiring, editor, model/mode/effort controls, tool approvals, send/edit/copy/feedback/shortcut behavior, large rendering template | page as composition root; extract reusable prompt/config/message actions and page controller logic |
| `app/pages/chat/index.vue` | workspace restore, editor, chat configuration, creation, pending prompt handoff; duplicates mode/effort/model concepts | share chat configuration primitives and creation controller while preserving SSR restore behavior |
| `app/composables/useConversationChat.ts` | AI SDK transport, seed lifecycle, error mapping, client terminal execution, durable attempt ledger, approval watcher, store mirror, debounce/persistence/refetch | keep a thin public chat composition root; isolate independent policies/controllers without changing approval or stream semantics |
| `ModelList.vue` / `ProviderList.vue` | list rows, edit modal lifecycle, save/delete notifications; domain forms differ | extract only shared interaction chrome/behavior that has the same contract; keep provider/model-specific form policy separate |
| flat component/composable surface | unrelated feature units share one namespace and giant files carry feature ownership | group presentation components by feature; retain Nuxt-friendly public composable entrypoints |

### Server TypeScript hotspots

| Area | Current mixed responsibilities | Refactor intent |
| --- | --- | --- |
| `server/api/chat.post.ts` | session/request handling, DB reads/writes, turn mutation, context compaction, workspace resolution, prompt construction, MCP/native tool composition, device lookup, approval policy, token cache, assistant persistence, LangGraph branch, AI SDK branch, stream response | HTTP route becomes thin adapter; application chat-turn use case coordinates narrow repositories/policies/stream adapters |
| `server/utils/providers.ts` | persistence, secret encryption, provider policy, DTO projection, upstream model discovery/error mapping | separate provider use cases/policy from persistence and provider integration where doing so reduces coupling |
| `server/utils/models.ts` | ownership check, Drizzle CRUD, domain update field list | move ownership/data access behind feature-specific persistence functions; keep route validation separate from domain policy |
| flat `server/utils/` | DB/domain/integration/telemetry/filesystem/chat concerns share one bucket | explicit application/domain/infrastructure groupings for high-value areas; avoid moving tiny stable helpers just for aesthetics |
| `server/database/schema.ts` | all tables in one file | review after data-access boundaries stabilize; split only if schema ownership/maintenance clearly improves, with barrel compatibility and no needless migration |

### Rust relay hotspots

| Area | Current mixed responsibilities | Refactor intent |
| --- | --- | --- |
| `relay_agent/transport.rs` | router, CORS, limits, app state, correlation/audit, admission control, OAuth metadata/challenges, trusted proxy decision, JWKS fetch/cache, JWT validation, HTTP parsing, MCP header validation, dispatch, handlers | transport owns HTTP composition only; auth/JWKS, admission, observability, metadata, and request validation become focused modules |
| `relay_agent/execution.rs` | dispatch plus per-tool validation/argument translation/process invocation | shared execution runner + focused terminal/http/search request builders/handlers while preserving all security checks |
| `relay_agent/mcp.rs` | protocol types and pure MCP logic | largely preserve as the good protocol boundary; split only where cohesion measurably improves |
| `relay_agent/config.rs` | CLI/config + security validation | preserve validation authority; only extract reusable path/policy functions if they gain clear independent ownership |

Large files are **signals, not violations by themselves**. A file is split because it owns independent reasons-to-change, not because it crosses an arbitrary line-count limit.

---

## Target repository architecture

### A. Nuxt application

```text
app/
  pages/                    # route composition only
  layouts/                  # shell composition only
  components/
    shell/                  # app/sidebar/search shell UI
    chat/                   # chat presentation primitives
    workspace/              # workspace presentation primitives
    settings/               # settings presentation primitives
    ...                     # existing cohesive feature groups
  composables/              # public Nuxt reactive/application entrypoints
  utils/                    # app-only pure/simple utilities where appropriate

shared/
  types/                    # client/server contracts only
  schemas/                  # runtime-neutral shared schemas where truly shared
  utils/                    # runtime-neutral domain/pure utilities only
```

Rules:

1. Pages/layouts coordinate; they do not own reusable business logic.
2. Components receive stable props and emit intent; they should not duplicate persistence rules.
3. Composables own Vue/Nuxt reactive application state and lifecycle.
4. Pure non-reactive logic should leave components/composables when it has independent reuse/value.
5. Server-only code never moves into `shared/` merely for convenience.
6. `shared/` must stay runtime-neutral and free of secrets/server-only dependencies.
7. Nested component folders are preferred for feature organization.
8. Nuxt only auto-scans top-level application composables by default, so public composables stay top-level or are deliberately re-exported/configured; do not silently break auto-imports during folder cleanup.
9. Do not introduce Nuxt Layers unless a real independently-composable product/module boundary emerges later.

### B. Nuxt server

Proposed conceptual layering:

```text
server/
  api/                      # HTTP/H3 adapters: session, input, output
  application/              # use cases/orchestration
    chat/
    models/
    providers/
    workspaces/
    ...
  domain/                   # pure policies/value transformations where warranted
  infrastructure/
    database/               # feature-specific Drizzle persistence adapters
    ai/                     # AI SDK/LangGraph/provider adapters
    mcp/                    # MCP client/tool adapters
    filesystem/             # filesystem/workspace adapters
    security/               # encryption/auth integration adapters as appropriate
  utils/                    # small truly cross-cutting Nitro helpers only
  database/                 # Drizzle connection/schema/migrations remain authoritative
```

This is a **target direction, not a mandatory bulk move**. Exact directories are introduced only when a phase has enough cohesive code to justify them.

Rules:

- `server/api/**` may depend on application functions and transport helpers, not directly own multi-step DB/provider orchestration.
- application functions receive narrow dependencies where isolation matters; use plain functions/factories, not a DI container;
- domain policy has no H3 event, Drizzle table, provider SDK, or filesystem dependency;
- infrastructure may depend on Drizzle/provider SDKs and implement application-required ports;
- current joined server response patterns remain intact: one screen needing related data should still receive one server-shaped response rather than client-side orchestration;
- use `#server` explicit imports for new non-auto-imported server modules where that makes boundaries readable;
- existing small, already-thin CRUD routes are not rewritten just to match a diagram.

### C. Rust relay/native tools

Conceptual dependency direction:

```text
bin/relay-agent
   ↓
relay_agent router/composition
   ├── transport HTTP adapters
   ├── auth/JWKS policy + adapter
   ├── admission/observability middleware
   └── MCP request application dispatcher
               ↓
         protocol (`mcp`)
               ↓
         execution policies
               ↓
       subprocess/tool adapters
```

Rules:

- `mcp.rs` remains transport-independent;
- HTTP headers/status/CORS stay transport concerns;
- OAuth challenge/JWKS/token validation gets one focused ownership boundary;
- rate/admission logic does not become intertwined with auth or execution;
- execution-root, privilege, SSRF, timeout, output-limit, and process-group rules stay fail-closed and authoritative;
- module extraction may not change ordering of security gates;
- no dynamic generic backend abstraction until there is an actual second backend;
- no Docker backend or host socket access is introduced by this refactor.

---

## Reusability rules

### Reusable UI components

Create a reusable component when at least one is true:

- the same interaction appears in multiple screens/features;
- the component is a stable visual primitive with a clear prop/event contract;
- extracting it materially simplifies a composition root and keeps domain behavior outside the component.

Prefer:

- feature components such as shell/sidebar sections, chat prompt configuration controls, entity action rows, confirmation/edit dialog shells;
- slots only for genuine extension points;
- domain-named props/events instead of generic bag/config props.

Reject components that:

- only wrap one Nuxt UI component with no stable behavior;
- need many booleans to switch unrelated modes;
- hide side effects that should remain in a composable/application controller.

### Reusable application logic

Prefer reusable logic in this order:

1. pure function/value mapping;
2. focused reactive composable;
3. factory with a small dependency object;
4. stateful class only when object lifecycle/identity genuinely makes it clearer.

Candidate reusable logic discovered in baseline:

- canonical chat `modeItems` and reasoning-effort options;
- model capability lookup/config option mapping;
- safe chat error-message extraction;
- local-tool attempted-call ledger abstraction;
- chat message mirror/debounce coordinator;
- stable editable-entity modal lifecycle only if Model/Provider implementations prove the same behavior;
- provider response projection and base-URL requirement policy;
- feature-specific persistence helpers used by multiple server use cases;
- Rust subprocess output/timeout/process cleanup runner reused by tool handlers.

### No accidental cross-layer reuse

A helper is not reusable if reusing it requires importing a higher layer into a lower layer. Duplicate tiny adapter glue is preferable to reversing dependency direction.

---

## Architectural guardrails to add during implementation

The refactor should leave lightweight enforcement behind.

1. Use existing ESLint flat config and core import restrictions where they can encode clear forbidden dependency directions without type-aware linting.
2. Prefer targeted `no-restricted-imports` rules over a new architecture-lint dependency.
3. Keep the current repository-policy and agent-doc checks intact.
4. If a structural check cannot be expressed cleanly in ESLint, add a very small deterministic local policy script only when it protects a critical boundary and can run inside `pnpm verify:commit`.
5. Do not add CI or a unit-test framework.
6. Rust module privacy and explicit module dependencies should do most architectural enforcement; add custom tooling only if a real recurring violation remains.

Potential import rules after the target paths exist:

- app/shared code cannot import `server/**`;
- server domain code cannot import H3/Nitro handlers, Drizzle schema/adapters, AI/provider SDKs, or filesystem implementations;
- server API routes should not import Drizzle schema directly once the applicable feature has migrated;
- presentation components should not import server infrastructure or perform direct DB concerns;
- shared runtime-neutral modules cannot import Vue/Nuxt server runtime internals.

Guardrails are introduced **after** call sites migrate so they protect the new architecture instead of blocking the migration itself.

---

## Migration strategy

### Incremental strangler refactor

No “move everything, then fix compilation” branch.

For each responsibility:

1. identify current contract and behavior;
2. extract the new focused module alongside the old call site;
3. keep an old facade/export temporarily if needed;
4. move callers in a bounded batch;
5. verify behavior and local gates;
6. remove obsolete path/facade once no caller needs it.

### Change boundaries

- each phase should be independently mergeable;
- prefer one focused PR per phase or cohesive sub-phase;
- do not mix broad formatting/rename churn with behavior-sensitive extraction;
- no database migration unless a real data model change is separately justified (none is currently planned);
- no dependency upgrades bundled into structural work unless required for the refactor;
- preserve exported route shapes, shared types, tool names, MCP wire behavior, exit codes, and security semantics throughout.

### Refactor correctness rule

A refactor phase is not complete merely because it compiles. The relevant runtime flow must be exercised at the boundary it changed.

---

# Execution phases

## Phase 0 — Freeze baseline and architectural contracts

**Risk: low**

### Work

- [x] Inventory current feature boundaries/import graph for app, server, shared, packages, and Rust relay.
- [x] Record public contracts that must not change: API response shapes, shared chat/provider/workspace types, MCP tool catalog, MCP protocol/version behavior, native CLI behavior, security ordering.
- [x] Build a duplication ledger grouped into **knowledge duplication**, **behavior duplication**, and **presentation-only duplication**.
- [x] Mark current hotspots by responsibilities/reasons-to-change, not only file size.
- [x] Define the first set of dependency-direction rules that can later be enforced by ESLint/module layout.
- [x] Confirm no pending product feature is being silently folded into the refactor.

### Acceptance

- no application/runtime behavior changed;
- each later phase has a named contract to preserve;
- every proposed abstraction has at least one concrete source responsibility/caller;
- no speculative framework is introduced.

### Verification

- docs/structure inspection;
- `pnpm verify:commit` before the implementation commit.

---

## Phase 1 — Canonical shared concepts and low-risk DRY cleanup

**Risk: low**

### Targets

- `app/pages/chat/index.vue`
- `app/pages/chat/[id].vue`
- model/reasoning configuration helpers
- shared runtime-neutral types/constants where appropriate

### Work

- [x] Centralize chat mode options and reasoning-effort options that currently have duplicated source-of-truth.
- [x] Centralize model capability/config mapping used by both new-chat and existing-chat controls.
- [x] Move pure error/format/value transformations out of large presentation/composable files when they have independent reuse.
- [x] Audit duplicated provider-type/base-URL requirements and establish one authoritative server policy while keeping request-schema validation user-friendly.
- [x] Reconcile duplicated types currently declared inside composables with shared contracts only when both client and server truly need them.

### Guardrails

- shared modules stay runtime-neutral;
- do not centralize UI copy that may intentionally diverge by screen;
- do not create a generic settings schema for unrelated settings.

### Acceptance

- one canonical definition for same-concept mode/effort/provider policy;
- no behavior change in chat creation/edit controls;
- no new circular imports;
- existing API payloads unchanged.

### Verification

- `pnpm verify:commit`;
- `pnpm build` because shared imports can affect SSR/client bundling;
- browser smoke: new chat and existing chat model/mode/reasoning controls.

---

## Phase 2 — App shell/sidebar presentation decomposition

**Risk: medium**

### Primary target

- `app/layouts/default.vue`

### Work

- [x] Keep `default.vue` as a thin dashboard composition root.
- [x] Extract workspace/sidebar rendering into cohesive feature components.
- [x] Extract conversation row actions/dialog state from layout markup.
- [x] Extract workspace create/confirm/rename/details interaction into workspace-focused presentation/controller units.
- [x] Extract dashboard search UI/group mapping from unrelated workspace mutation logic.
- [x] Keep account menu/logout as a small shell concern or focused component depending on resulting cohesion.
- [x] Introduce one shell/sidebar controller composable only if it meaningfully coordinates shared state; avoid a giant replacement composable.

### Critical preserved invariants

- deep-link conversation load must still set active workspace before sidebar render where required;
- request-bound Nuxt composables must still be invoked before unsafe async boundaries;
- `/api/sidebar` remains the joined server fetch; do not regress to multi-fetch client orchestration;
- workspace loaded/active restoration timing must remain unchanged;
- shortcut/search behavior remains functional.

### Acceptance

- layout primarily composes components/controllers rather than implementing CRUD/UI details;
- extracted components have focused props/events;
- no duplicated workspace/conversation action rules introduced;
- SSR deep-link/sidebar behavior unchanged.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- `pnpm preview` when practical;
- browser smoke: login shell, sidebar load, deep-link chat refresh, workspace switch/create/rename/delete/confirm/details, conversation rename/delete, search, keyboard shortcuts, logout.

---

## Phase 3 — Reusable chat presentation and page controllers

**Risk: medium**

### Targets

- `app/pages/chat/index.vue`
- `app/pages/chat/[id].vue`
- chat-related components

### Work

- [x] Extract reusable chat configuration controls for model/mode/reasoning/tool selection without hiding persistence ownership.
- [x] Reuse the same option/config mapping between new-chat and existing-chat pages.
- [x] Extract message action UI (copy/edit/regenerate/feedback) into focused components where contracts are stable.
- [x] Extract edit-message modal/dialog behavior if it materially simplifies the page.
- [x] Isolate new-conversation creation/handoff orchestration in a focused composable.
- [x] Isolate existing-conversation page state/config mutation logic from template concerns.
- [x] Keep `useChatEditor` as the shared editor behavior boundary; improve it rather than duplicating editor keyboard/mention logic.

### Acceptance

- both chat pages use common reusable configuration primitives;
- pages remain readable composition roots;
- feature-specific side effects stay in composables/controllers, not dumb presentation components;
- pending first prompt remains one-shot across navigation/reload;
- tool approval and mode-specific mention behavior unchanged.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- browser smoke: create chat, workspace target, model/mode/effort selection, tool picker, send, edit/resend, regenerate, copy, approval UI, Esc stop.

---

## Phase 4 — Client state/data composable responsibilities

**Risk: medium-high because SSR state semantics are sensitive**

### Targets

- `useConversations.ts`
- `useWorkspaces.ts`
- `useModels.ts`
- `useModelProviders.ts`
- related loading composables

### Work

- [x] Separate state mutation helpers from remote API concerns where doing so improves reuse/readability.
- [x] Normalize repeated collection replacement/upsert/remove logic with small pure helpers only when semantics match.
- [x] Preserve per-entity differences in optimistic update/rollback; do not force one generic CRUD composable.
- [x] Audit narrow API adapter/fetcher factories; no extraction was justified without weakening explicit SSR request-fetch context.
- [x] Keep public Nuxt composable entrypoints stable during migration.
- [x] Audit loaded/pending/error semantics; no common lifecycle exists across these composables beyond the existing workspace contract.

### Critical preserved invariants

- `useState` remains request-safe shared state; no module-scope request data;
- SSR-authenticated fetches keep forwarded request context;
- no request-bound composable is newly called after arbitrary `await`;
- workspace active restoration finishes before consumers treat workspace state as loaded;
- conversation `loadOne` still handles direct/deep-link insertion correctly;
- optimistic mutations retain current rollback/error behavior per feature.

### Acceptance

- no mega generic CRUD abstraction;
- repeated collection/persistence mechanics are reused where identical;
- public composable contracts are smaller/clearer or intentionally preserved behind facades;
- SSR behavior remains correct.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- browser smoke on hard refresh/deep link plus CRUD settings/workspace/chat flows.

---

## Phase 5 — Server CRUD/application/data-access boundaries

**Risk: medium**

### Initial targets

- models
- providers
- workspaces/conversations where repetition justifies it
- small API routes that already provide clear migration examples

### Work

- [x] Establish thin route pattern: authenticate → parse/validate → call use case → return result.
- [x] Move multi-step ownership/business rules into application/domain functions.
- [x] Move Drizzle feature queries into feature-specific infrastructure/persistence functions when reused or when they currently pollute orchestration.
- [x] Keep DTO projection explicit at the boundary where secrets/internal columns are removed.
- [x] Separate provider upstream discovery from provider persistence.
- [x] Keep encryption as infrastructure/security capability, not model/provider domain state.
- [x] Use narrow feature-specific persistence APIs rather than a generic repository base class.

### Acceptance

- migrated routes no longer embed persistence orchestration;
- ownership rules have one authoritative implementation per feature;
- provider secret fields cannot leak through shared projections;
- no public API shape changes;
- no DB schema migration required for this phase.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- manual API/UI smoke for provider/model/workspace CRUD;
- `pnpm audit` only if dependencies changed (dependencies are not expected to change).

---

## Phase 6 — Chat application use case and persistence extraction

**Risk: high / critical**

### Primary target

- `server/api/chat.post.ts`

### Desired decomposition

The route should end near this conceptual shape:

```text
HTTP adapter
  -> authenticate + parse request
  -> build request-scoped dependencies
  -> executeChatTurn(input, dependencies)
  -> return stream response
```

Application orchestration should coordinate focused capabilities such as:

- load authorized conversation/model/provider;
- load bounded message history;
- apply submit/regenerate/resume turn mutation;
- resolve context compaction;
- resolve workspace context;
- build tool set/approval policy;
- select chat-mode or agent-mode stream adapter;
- persist assistant message/usage;
- close resources and propagate cancellation.

### Work

- [x] Extract authorized chat-turn data loading/repository functions.
- [x] Extract history query + trigger-specific message mutation into a cohesive turn-input stage.
- [x] Extract workspace-context resolution and system-prompt policy.
- [x] Extract paired-device/local-terminal availability and approval policy from the HTTP route.
- [x] Extract assistant persistence/continuation/token-cache behavior.
- [x] Extract AI SDK agent streaming adapter.
- [x] Extract LangGraph chat streaming adapter.
- [x] Keep one application use case deciding mode and coordinating common lifecycle.
- [x] Ensure MCP resource close happens exactly once on all appropriate completion paths.
- [x] Preserve abort/cancellation wiring from request close through provider stream.

### Critical preserved invariants

- bounded history after context-summary cutoff;
- submit/regenerate/tool-approval resume semantics;
- context compaction and measured-token cache behavior;
- local terminal is agent-mode-only and client-executed;
- no server shell execution is accidentally reintroduced;
- tool approval uses AI SDK-native flow;
- current timeouts/step budget/reasoning middleware semantics;
- assistant continuation persistence and provider metadata diagnostics;
- errors remain user-visible/logged at current boundaries;
- joined/server-owned data behavior stays server-side.

### Acceptance

- `chat.post.ts` is a thin transport/composition adapter;
- application chat logic is callable without an H3 event object;
- Drizzle and SDK details sit behind focused infrastructure functions/adapters;
- no new generic chat framework or DI container;
- chat/agent behavior and stream protocol stay compatible.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- browser smoke across chat and agent mode;
- exercise send, regenerate, approval allow/deny/remember, local-terminal unavailable/error path, MCP tool call, reasoning model, stop/abort, context usage refresh;
- inspect persisted DB behavior where practical.

---

## Phase 7 — `useConversationChat` split without stream/approval regression

**Risk: high / critical**

### Primary target

- `app/composables/useConversationChat.ts`

### Work

- [x] Keep `useConversationChat()` as the public composition root expected by chat pages.
- [x] Extract chat error normalization as pure reusable logic.
- [x] Extract local client-tool execution controller from AI SDK construction.
- [x] Extract durable attempted-tool-call storage behind a tiny storage adapter/policy.
- [x] Extract message mirror/debounce/flush coordination.
- [x] Keep transport request preparation in one focused adapter/factory.
- [x] Keep status/seed lifecycle logic visible enough that future changes cannot casually reintroduce chat-instance recreation.

### Critical preserved invariants

- `useChat` options must not become reactively invalidated per streamed chunk;
- conversation ID and seed snapshot strategy stays stable unless a verified equivalent replaces it;
- `local_terminal` **must not execute from `onToolCall`**;
- local terminal executes only after `approval-responded` with approved=true;
- durable attempt ledger prevents accidental duplicate execution after reload/crash window;
- denied calls never execute;
- SDK message watching remains non-deep to avoid per-token full traversal;
- message mirror remains debounced and flushes at end-of-turn;
- server-owned conversation fields refresh once per completed turn, not per chunk.

### Acceptance

- public composable is materially smaller and primarily composes focused responsibilities;
- each extracted unit has one clear reason to change;
- approval/execution ordering is obvious in code review;
- no performance regression on long streamed conversations.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- browser/manual approval matrix: ask / always / never / deny;
- reload with historical approved tool-call state;
- streaming long conversation sanity check;
- local relay offline/error reporting path.

---

## Phase 8 — Provider, MCP, and tool integration extension seams

**Risk: medium-high**

### Targets

- provider resolution/factories
- provider model discovery
- MCP tool construction
- native/local tool registration
- related server integration utilities

### Work

- [x] Clarify provider contract vs provider-specific adapters.
- [x] Ensure adding a provider implementation primarily adds an adapter/registration entry instead of modifying unrelated chat orchestration.
- [x] Keep provider configuration validation/domain policy independent from SDK construction.
- [x] Separate MCP server persistence/config from live MCP client/tool building.
- [x] Audited tool metadata/approval policy; MCP and native approval identities remain intentionally separate because their ownership and lifecycle differ.
- [x] Keep native/local-terminal tool behavior separate from remote MCP tools while sharing only compatible AI SDK boundary helpers.
- [x] Clarified provider ownership with the adapter registry in `server/utils/providers/index.ts` and the persistence facade in `server/utils/providers.ts`; no ambiguous duplicate was removed because the facade remains a compatibility boundary.

### Acceptance

- extension points are explicit but not plugin-framework-heavy;
- adding a provider/tool does not require touching unrelated UI/server layers;
- secrets remain server-only;
- AI SDK-native approval semantics remain authoritative;
- frozen MCP/native tool identities/contracts do not change accidentally.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- provider/model discovery smoke;
- MCP tool list/call smoke;
- dependency audit only if a dependency changes.

---

## Phase 9 — Database/schema organization review

**Risk: medium if moved; low if left intact**

### Work

- [x] Review `server/database/schema.ts` only after application/persistence ownership is clear; the current application/infrastructure boundaries are sufficient for this review.
- [x] Evaluate splitting table groups by cohesive domain; leave the schema as one authoritative module because its tightly interdependent foreign keys (including circular references) make a split add indirection without clearer ownership or navigation benefit.
- [x] Keep Drizzle migration schema semantics identical; `drizzle-kit check` reports no migration changes.
- [x] Keep database initialization/connection centralized.
- [x] Do not create one repository class per table; persistence APIs follow use cases/aggregates, not table count.

### Decision gate

If splitting the schema adds indirection without improving ownership/navigation, **leave it as one file**. KISS wins over visual symmetry.

### Acceptance

- no unintended generated migration;
- existing imports remain stable or migrate cleanly through a barrel;
- ownership boundaries are clearer, not merely more files.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- run applicable Drizzle schema/migration inspection; no migration should appear for a source-only split.

---

## Phase 10 — Rust relay transport/auth modularization

**Risk: very high / security-critical**

**Status: complete** — boundary review and Rust verification passed; bearer-auth
HTTP orchestration intentionally remains in `transport.rs` as the composition
root.

### Primary target

- `packages/rust-tools/src/relay_agent/transport.rs`

### Work

> Micro-step 10A complete: pure OIDC-discovered `jwks_uri` validation is
> centralized in `auth.rs`; fetch, cache, token validation, and request flow
> remain in `transport.rs`.

> Boundary decision: keep bearer-auth orchestration in `transport.rs` as the
> HTTP/security composition root. It owns request middleware, trusted transport
> gates, ordering, `Next` continuation, and HTTP response/challenge mapping;
> `auth.rs` owns the focused policy, fetch, cache, refresh, and token helpers.
> Moving orchestration again would add indirection without clearer ownership
> and would risk the established security ordering. No runtime change is
> required for this boundary.

- [x] Keep router construction and HTTP middleware composition in transport ownership.
- [x] Extract correlation/audit helpers into an observability-focused module.
- [x] Extract request-admission token bucket into a focused admission module.
- [x] Extract OAuth challenge/protected-resource metadata helpers into auth/metadata ownership.
- [x] Extract JWKS fetch/cache/refresh/token-validation flow into focused auth helpers; keep HTTP bearer orchestration in transport by the boundary decision above.
- [x] Micro-step 10D: OIDC discovery/JWKS HTTP helpers now live in `auth.rs`; cache ownership and JWT validation remain in `transport.rs`.
- [x] Micro-step 10E: pure owner/scope claim policy now lives in `auth.rs`; cache refresh and JWT signature/issuer/audience/expiry validation remain in `transport.rs`.
- [x] Micro-step 10F: JWKS cache TTL/URI/key lookup and missing-key single-refresh decision operations now live in `auth.rs`; lock ownership and refresh orchestration remain in `transport.rs`.
- [x] Micro-step 10G: JWT signature/algorithm/issuer/audience/expiry/nbf validation now lives in `auth.rs`; HTTP/cache/lookup orchestration remains in `transport.rs`.
- [x] Micro-step 10J: initial/stale and unknown-`kid` JWKS cache writes/refresh operations now use focused async helpers in `auth.rs`; transport retains auth gate/order and error mapping.
- [x] Micro-step 10K: cache snapshot/URI reads and cached-`kid` lookup now use lock-owning async helpers in `auth.rs`; transport retains auth gate/order and refresh/error mapping.
- [x] Extract trusted-proxy HTTPS decision with explicit security ownership (micro-step 10C: pure, tested policy helper; transport retains the same gate position and inputs).
- [x] Extract MCP HTTP header/body validation from router composition while keeping HTTP-specific validation outside pure `mcp.rs` (Phase 10H complete; all call sites use `transport_validation.rs`).
- [x] Extract MCP route handler dispatch once dependencies are explicit (Phase 10I complete; all method selection uses `dispatcher.rs`).
- [x] Keep `mcp.rs` protocol-pure and avoid dragging Axum/auth state into it.

### Security ordering that must remain true

1. body/concurrency/admission bounds protect expensive work;
2. access policy executes before MCP tool dispatch;
3. remote mode requires trusted HTTPS termination and explicit proxy trust;
4. OAuth/JWT/JWKS validation fails closed;
5. owner subject and `relay.coding` authorization remain enforced;
6. local mode still enforces exact local access policy;
7. sensitive token/subject/arguments remain excluded from logs;
8. MCP header/protocol validation remains exact;
9. unauthenticated/insufficient-scope challenge behavior remains spec-compatible.

### Acceptance

- `transport.rs` no longer owns router + OAuth/JWKS + admission + observability + protocol dispatch all at once;
- module APIs are narrow and security gate order is visible from composition code;
- no wire-contract changes;
- no weakened fail-closed behavior.

### Verification

- `pnpm verify:commit` (includes Rust fmt/Clippy/check);
- `cargo audit`;
- existing deterministic relay/MCP/security acceptance scripts applicable to the touched boundaries, including black-box, ChatGPT contract, and zero-bypass checks;
- manual/local health + MCP request smoke where available.

---

## Phase 11 — Rust execution/tool-handler decomposition

**Risk: very high / security-critical**

**Status: complete** — ownership audit confirms the remaining execution path is
cohesive and behavior-sensitive; no unsafe generic extraction is justified.

> Micro-step 11A-terminal-policy complete: terminal cwd containment and executable privilege/path policy are isolated in `terminal_policy.rs`; execution translation and subprocess lifecycle remain in `execution.rs`.

> Boundary decision 11B: HTTP/SSRF and redirect policy remains owned by the
> authoritative native `curl-tool` path. `execution.rs` has no duplicate HTTP
> security policy to extract; retaining this boundary is the KISS/DRY layered
> choice and requires no runtime change.

> Micro-step 11C reviewed: no extraction justified. Sibling-binary resolution
> and subprocess mechanics occur in one shared post-dispatch safety path; a
> terminal-only extraction would create a partial/generic abstraction and risk
> process-group, timeout, and output-limit semantics.

### Primary target

- `packages/rust-tools/src/relay_agent/execution.rs`

### Work

- [x] Separate terminal policy ownership via `terminal_policy.rs`; HTTP/SSRF remains authoritative in native `curl-tool`, and existing search/translation logic remains cohesive in `execution.rs` (no duplicate or unsafe split).
- [x] Review common sibling-binary resolution and subprocess mechanics; retain the single shared post-dispatch path because a terminal-only extraction would be partial and a generic abstraction is prohibited.
- [x] Retain output limit, timeout, termination/process-group handling in the single authoritative execution path; semantics are already centralized and unchanged.
- [x] Keep tool-specific security policy next to its owner: terminal policy in `terminal_policy.rs`, HTTP/SSRF in native `curl-tool`, and search semantics in their existing native path.
- [x] Avoid a generic “execute arbitrary tool” abstraction that can bypass per-tool policy.

### Critical preserved invariants

- sibling binaries resolved relative to relay executable;
- explicit execution root containment;
- privilege escalation command rejection;
- Bubblewrap/non-root startup and runtime boundaries remain untouched;
- timeout and kill/process cleanup semantics remain stable;
- output/argument/header limits remain enforced;
- curl/HTTP SSRF and redirect protections remain in the authoritative native tool path;
- stable MCP result/error semantics preserved.

### Acceptance

- dispatcher chooses focused handlers; handlers cannot bypass common process safety;
- each tool policy remains obvious and auditable;
- no supported JS CLI fallback reappears;
- no new execution backend is added.

### Verification

- `pnpm verify:commit`;
- `cargo audit`;
- native CLI deterministic/zero-bypass/security scripts relevant to terminal/curl/search and relay;
- representative manual command/http/search invocations inside allowed boundaries.

---

## Phase 12 — Cross-package reuse and naming cleanup

**Risk: medium**

> Micro-step 12A audit decision: TS packages remain schema/API/tool-factory
> facades only; native Rust binaries remain the executable source of truth.
> Tool names differ intentionally by runtime boundary (MCP native IDs versus
> application LangChain/AI SDK names), so consolidating them would change
> contracts. Existing package skills already document the Rust ownership and
> removed npm CLI bins. No low-risk duplicate source of truth or stale facade
> with a concrete caller was found; no runtime change is justified.

### Targets

- TS wrapper packages
- shared tool schemas/contracts
- package docs/skills
- ambiguous/stale internal names discovered by earlier phases

### Work

- [x] Ensure TS packages expose reusable schema/API concerns without reclaiming executable ownership from Rust (12A audit).
- [x] Audit duplicated tool identifiers/schemas; retain intentionally distinct runtime-boundary contracts because consolidation would alter callers.
- [ ] Remove dead facades/compatibility exports left by migration phases.
- [ ] Normalize naming around application/domain/infrastructure responsibilities.
- [ ] Remove obsolete comments pointing at pre-refactor paths while preserving important historical/security rationale in canonical docs/memory where durable.

### Acceptance

- no duplicate executable implementation;
- package public APIs remain intentional;
- no stale compatibility layer remains without a caller;
- docs match final ownership boundaries.

### Verification

- `pnpm verify:commit`;
- `pnpm build`;
- `cargo audit` if Rust/package dependency surface changed;
- applicable deterministic tool checks.

---

## Phase 13 — Architecture enforcement, final cleanup, and regression pass

**Risk: medium**

### Work

- [ ] Add finalized import-direction restrictions to ESLint where useful.
- [ ] Add only minimal deterministic architecture policy checks that cannot be expressed clearly by existing lint/tooling.
- [ ] Remove unused imports/files/helpers/facades discovered after all callers migrated.
- [ ] Review every remaining large file for cohesion; do not split cohesive files for line-count aesthetics.
- [ ] Review all reusable components/composables for stable naming/props/contracts.
- [ ] Re-run duplication search for canonical rules/constants and remove accidental forks.
- [ ] Update `.agents/knowledge/` only where new architectural conventions are durable and repository-wide.
- [ ] Append only durable lessons/invariants to `.agents/memories/README.md`; do not create sibling memory files.
- [ ] Mark this Plan 031 complete only after all accepted phases/gates are actually done.

### Final acceptance

- presentation/transport layers primarily compose;
- business/application rules have one authoritative owner;
- no duplicated source-of-truth for shared chat configuration/provider policy/tool identity;
- reusable UI and logic are used in more than one real caller where expected;
- server application orchestration is not coupled to H3 event objects;
- migrated server domain/policy code is not coupled to Drizzle/SDK/filesystem implementation details;
- `chat.post.ts`, `useConversationChat.ts`, and `default.vue` no longer combine their current independent concern sets;
- Rust transport no longer owns OAuth/JWKS/admission/audit/protocol dispatch as one module;
- Rust execution no longer combines all per-tool translation with all shared process mechanics in one dispatcher;
- frozen MCP/native security and wire contracts remain intact;
- no CI and no unit-test suite introduced;
- no needless dependency/framework introduced;
- no dead compatibility layer remains;
- final docs describe the architecture that actually shipped.

### Final verification

At minimum:

```sh
pnpm verify:commit
pnpm build
```

Plus, as applicable across the final integrated state:

- browser build/preview smoke of auth, sidebar/workspaces, new chat, existing chat, settings/provider/model, chat/agent/tool approvals, error/retry states;
- `pnpm audit` if dependencies changed;
- `cargo audit` for Rust/security-sensitive final state;
- all existing deterministic relay/MCP/native-tool security and contract scripts affected by the refactor;
- Drizzle migration/schema sanity if database source organization changed.

Because the repository has no CI, every PR must record the **exact local verification actually performed**. “Mergeable” is not evidence that this refactor is safe.

---

## Phase/PR strategy

This plan is intentionally too broad for one implementation PR.

Recommended integration sequence:

```text
031.0 baseline/contracts
  ↓
031.1 shared DRY concepts
  ↓
031.2 app shell
  ↓
031.3 chat presentation
  ↓
031.4 client state/data
  ↓
031.5 server CRUD boundaries
  ↓
031.6 server chat orchestration
  ↓
031.7 client chat orchestration
  ↓
031.8 provider/MCP seams
  ↓
031.9 DB organization decision
  ↓
031.10 Rust transport/auth
  ↓
031.11 Rust execution
  ↓
031.12 packages/naming cleanup
  ↓
031.13 final enforcement/regression
```

A phase may split into multiple PRs when the change surface is large. Every PR must be cohesive, independently buildable, and leave the repository in a working state.

Suggested branch naming during implementation:

```text
refactor/031-p1-shared-chat-config
refactor/031-p2-app-shell
refactor/031-p6-chat-application
refactor/031-p10-relay-transport
```

Do not merge a later phase just to make an earlier broken phase compile.

---

## Review checklist for every refactor PR

Before approving a phase, answer all of these:

### DRY

- Did this remove or prevent duplicated **knowledge**, not just move lines?
- Is there now one clear source of truth for each policy/constant changed?
- Did we avoid a generic abstraction whose callers actually have different semantics?

### SOLID

- Does each extracted module have one primary reason to change?
- Are extension seams open where there are real variants?
- Do equivalent adapters preserve contract/error/security semantics?
- Are dependencies narrow instead of mega service/context objects?
- Does application/domain code avoid depending directly on transport/infrastructure details?

### KISS

- Is the new code easier to trace than the old code?
- Could a pure function or direct composition replace a class/framework/registry?
- Did file count/abstraction count grow only because responsibilities became clearer?
- Is important security/async control flow still explicit?

### Layered Architecture

- Do imports flow in the intended direction?
- Is page/layout/route code primarily composition?
- Is reusable policy below presentation/transport?
- Is infrastructure kept at the edge rather than leaking upward/downward?
- Did anything get moved to `shared/` that is not truly runtime-neutral?

### Reuse

- Is the extracted component/logic actually reused or clearly isolating a critical independently-changing responsibility?
- Are props/functions narrow and domain-readable?
- Did reuse accidentally couple unrelated features?

### Safety

- Were existing runtime/security contracts explicitly exercised?
- Did `pnpm verify:commit` pass locally before the commit?
- Was the relevant build/browser/Rust/security verification recorded truthfully?

---

## Stop conditions / anti-overengineering gates

Stop extracting when any of these becomes true:

- the new abstraction has only one trivial caller and no independent reason to change;
- adding a feature requires understanding more indirection than before;
- an abstraction needs multiple boolean mode flags to serve unrelated callers;
- a “generic” interface is just a mirror of one concrete implementation;
- dependency injection plumbing becomes larger than the behavior being isolated;
- splitting a cohesive module makes security/control-flow ordering harder to audit;
- a folder restructure requires Nuxt config complexity without architectural benefit;
- a schema split produces no clearer ownership;
- reuse would cross a runtime/security boundary improperly.

When DRY and KISS appear to conflict, prioritize **one source of truth for important knowledge** while keeping obvious glue explicit.

---

## Out of scope unless separately approved

- product redesign or visual redesign;
- framework migration away from Nuxt/Vue, Nitro/H3, Drizzle, AI SDK/LangGraph, or Rust;
- new state-management framework;
- new DI framework;
- CI reintroduction;
- unit-test suite/framework introduction;
- replacing existing local verification policy;
- database model redesign/data migration merely for architecture aesthetics;
- changing MCP protocol version/tool catalog/wire contract as part of structural cleanup;
- changing relay OAuth/security policy;
- weakening filesystem/process/SSRF/approval boundaries;
- Docker execution support or host Docker socket exposure;
- restoring JavaScript executable fallbacks for native tools;
- opportunistic dependency upgrades unrelated to the refactor.

---

## Definition of done

Plan 031 is complete only when:

1. accepted phases are implemented and merged through normal feature PRs;
2. DRY/SOLID/KISS/layering rules are reflected in actual imports and ownership, not only docs;
3. reusable components/logic replace real duplicated concepts without introducing mega-abstractions;
4. current high-risk composition roots are decomposed around independent reasons-to-change;
5. architecture guardrails prevent obvious dependency regressions;
6. all required local lint/typecheck/build/runtime/security checks for the touched surfaces have actually passed and are recorded;
7. no behavioral/security contract regression is known;
8. canonical repository knowledge/memory reflects only durable final architecture;
9. this file is updated to **COMPLETED** with truthful final outcomes and any deliberately deferred items explicitly named.

Until then, **Status remains PLANNED / IN PROGRESS as appropriate; unchecked work is active only inside this Plan 031.**
