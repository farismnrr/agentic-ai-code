# Plan 039A — Maintainability and Layered Refactor Foundation

**Status:** CLOSED / VERIFIED — maintainability foundation complete  
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 038 closed/verified  
**Blocks:** Plans 039B through 039J

## Goal

Refactor the current repository **before** adding the broader Plan 039 coding-agent capability surface so future work lands on a codebase that is measurably easier to understand, extend, review, and verify.

The refactor must make these engineering principles enforceable in day-to-day work:

1. **DRY** — one authoritative source for one rule or piece of domain knowledge;
2. **SOLID** — pragmatic single-responsibility, narrow interfaces, dependency inversion, and real extension seams without class-heavy ceremony;
3. **Layered Architecture** — dependency direction and ownership are explicit and mechanically protected;
4. **YAGNI** — no speculative abstractions, frameworks, adapters, registries, or extension points without a current requirement;
5. **KISS** — prefer the simplest design that preserves behavior and security;
6. **Foldering / cohesion** — avoid large flat directories containing unrelated implementation files; group code by feature, capability, or responsibility;
7. **File-size discipline** — avoid oversized source files that accumulate unrelated reasons to change.

This is a **structural maintainability refactor**, not a product redesign. Existing user-visible behavior, MCP contracts, OAuth/resource-server behavior, sandbox boundaries, workspace containment, persistence semantics, telemetry confidentiality, and Plan 038 tool behavior must remain stable unless a current defect is independently proven and explicitly handled.

## Why this must precede the rest of Plan 039

Plan 039 will add Git intelligence, LSP adapters, richer policy enforcement, lifecycle hooks, subagents, background worktrees, task/context management, extension interoperability, and new UX/telemetry surfaces. Adding those capabilities directly into the current largest modules would increase coupling and create new maintenance debt.

Verified baseline on `dev` before this plan was written includes these maintained-source hotspots:

| File | Approx. lines | Refactor concern |
| --- | ---: | --- |
| `packages/rust-tools/application/src/execution.rs` | 1,319 | process execution, jobs, sandbox construction, tool request translation, multiple execution concerns |
| `packages/rust-tools/application/src/workspace.rs` | 1,239 | directory listing, file search/read/write/edit, secure traversal, result shaping |
| `packages/rust-tools/infrastructure/src/transport.rs` | 1,173 | HTTP/MCP transport composition, request handling, security/auth/admission orchestration |
| `packages/rust-tools/interfaces/src/mcp.rs` | 666 | protocol types, tool catalog, schemas, validation, result types |
| `packages/rust-tools/core/src/config.rs` | 539 | CLI/config surface plus validation/security-oriented configuration rules |
| `app/pages/chat/[id].vue` | 344 | route composition plus substantial chat-page interaction/rendering responsibility |
| `server/infrastructure/ai/langgraph/langgraph-chat.ts` | 341 | agent/model orchestration and LangGraph-specific integration logic |
| `app/pages/settings/mcp.vue` | 325 | settings-page composition and MCP interaction UI |
| `server/infrastructure/mcp/client.ts` | 316 | MCP client lifecycle, transport, configuration, and compatibility concerns |
| `app/pages/settings/local-terminal.vue` | 302 | settings-page composition and local relay interaction UI |

The flattest maintained-source folder observed during planning is `app/composables/` with 16 direct source files. This is not automatically wrong, but it is large enough to require a cohesion/foldering review before more agent features add additional composables.

Large files and busy folders are **signals, not crimes**. Splits must follow cohesive responsibility boundaries; do not create one-file-per-function fragmentation merely to satisfy a metric.

---

## Non-negotiable engineering rules

### 1. DRY — deduplicate knowledge, not coincidental syntax

Extract when multiple locations encode the same rule, policy, schema, mapping, limit, capability metadata, or business invariant.

Do not extract merely because two small functions happen to look similar.

Mandatory:

- one authoritative source for shared tool/capability metadata where the same concept is consumed by multiple layers;
- one authoritative implementation for filesystem containment/protected-path policy per runtime boundary;
- one authoritative validation/policy rule when multiple entrypoints must enforce the same invariant;
- shared UI primitives only when interaction semantics are genuinely identical;
- duplicate constants and enum-like option lists must be reviewed for a real shared owner.

Forbidden:

- generic `Repository<T>` / `CrudService<T>` abstractions with no current polymorphic need;
- generic component factories that replace clear feature-specific components with flag-heavy mega-components;
- helper layers that merely rename another function without owning policy or reducing coupling.

### 2. SOLID — pragmatic and composition-first

Apply SOLID to modules, functions, composables, traits, and contracts, not only classes.

- **SRP:** a module/file has one primary reason to change. Large composition roots may coordinate many collaborators but should not implement their internals.
- **OCP:** extension seams must exist where current requirements already need multiple implementations or future Plan 039 capabilities clearly plug into an existing contract; do not create speculative plugin systems.
- **LSP:** adapters implementing the same contract preserve error, timeout, cancellation, security, and result semantics.
- **ISP:** prefer small capability-specific ports/contracts over mega service objects.
- **DIP:** application/use-case code depends on application-owned contracts; concrete DB, provider, MCP, HTTP, filesystem, LSP, Git, and process implementations remain at infrastructure boundaries.

### 3. Layered Architecture — dependency direction over folder aesthetics

Current server direction remains:

```text
server/api / transport
        ↓
server/application
        ↑ implemented by
server/infrastructure
```

For the Rust relay, preserve the workspace-level crate roles and make internal module ownership clearer:

```text
interfaces     protocol/schema/public contract
      ↓
application    use cases / orchestration / capability behavior
      ↓
core           policy/config/domain-safe primitives
      ↑
infrastructure transport/auth/OS/network implementations
      ↑
cli            composition/bootstrap
```

The exact Rust dependency graph must remain consistent with Cargo and current architecture; do not force this diagram where it conflicts with a deliberately inverted port/adapter boundary. The important rule is that concrete infrastructure does not leak into core/application contracts simply because a file move is convenient.

### 4. YAGNI

Do not add during this refactor:

- a new DI framework/container;
- an event bus;
- a new state-management framework;
- a generic plugin framework;
- a custom AST/indexing framework;
- a repository-wide base-class hierarchy;
- a new package/crate merely to move a few files;
- abstractions whose only consumer is hypothetical future work.

Plan 039B–039J may introduce new capabilities later, but 039A should create only the seams that are justified by **current** responsibility splits or required to prevent immediate duplication.

### 5. KISS

Prefer, in order where appropriate:

- pure function;
- cohesive module;
- narrow trait/interface only when multiple implementations or inversion genuinely require one;
- explicit composition;
- existing framework/runtime idioms;
- small dependency objects rather than service locators.

The refactor should reduce conceptual count, not increase it.

### 6. Foldering / directory cohesion

A directory should represent a coherent feature, layer, or capability rather than becoming a dumping ground.

Initial repository maintainability budget:

- **Target:** no more than 10–12 direct maintained implementation files per cohesive folder.
- **Review threshold:** 13–15 direct implementation files requires an explicit cohesion review.
- **Violation threshold:** more than 15 direct maintained implementation files must be grouped into meaningful subfolders or carry a narrow documented exception.

Do not create artificial folders containing one trivial file solely to satisfy the count.

Appropriate grouping examples include:

```text
application/src/
  execution/
    mod.rs
    jobs.rs
    terminal.rs
    process.rs
  workspace/
    mod.rs
    list.rs
    search.rs
    read.rs
    mutate.rs

server/infrastructure/
  mcp/
  ai/
  database/
  observability/

app/composables/
  # preserve Nuxt public auto-import entrypoints where required
  # place internal feature implementation/controllers in cohesive subfolders
```

The implementation must verify Nuxt auto-import behavior before moving public composables. Do not break framework conventions merely for folder aesthetics.

### 7. File-size discipline

There is no universal industry-standard line count that makes a file good or bad, so this repository will use a pragmatic maintainability budget as a **guardrail** rather than a substitute for design review.

Initial budget for hand-maintained production source:

- **Target:** ≤ 300 physical lines per source file where cohesion permits.
- **Review threshold:** > 400 lines requires an explicit responsibility/cohesion review.
- **Violation threshold:** > 500 lines must be split or have a narrowly documented exception with a concrete cohesion reason.
- **Function/method review target:** functions around 60 logical lines or less are preferred; functions beyond roughly 100 logical lines require manual decomposition review. Do not build a custom parser merely to automate function-length counting.

Line thresholds exclude or separately classify files where physical length is not a useful maintainability proxy, such as:

- generated code/output;
- lockfiles;
- database migrations;
- frozen protocol/evidence fixtures;
- intentionally declarative registries/data tables where splitting would reduce discoverability;
- vendored third-party content.

An exception may never be justified only by “it already existed.” Exceptions must name the cohesive reason and be reviewed as part of repository policy.

---

## Scope

### In scope

- audit and refactor oversized/mixed-responsibility Rust relay modules;
- review server TypeScript modules for layered ownership, duplicated policy, and large mixed-responsibility files;
- review Vue/pages/components/composables for composition-vs-feature responsibility and flat-folder hotspots;
- introduce repository maintainability checks that prevent regression after the baseline is clean;
- preserve public APIs/contracts through stable facades/re-exports where useful during decomposition;
- update architecture documentation, development documentation, agent knowledge, and relevant skills/guides to reflect the actual final architecture;
- update deterministic architecture/maintainability verification so the rules are not prompt-only guidance;
- perform a fresh DRY/SOLID/Layered/YAGNI/KISS review after mechanical decomposition to catch architecture debt that simple line/folder metrics cannot detect.

### Out of scope

- implementing any Plan 039B–039J coding-agent capability;
- redesigning product UX;
- schema/business behavior changes unrelated to safe refactoring;
- dependency upgrades unrelated to the refactor;
- new CI;
- a normal unit-test suite;
- broad package/crate proliferation;
- changing client-visible MCP behavior merely to make files smaller.

---

# PHASE-01 — Freeze baseline and classify responsibilities

**Goal:** Establish a truthful baseline and a responsibility map before moving code.

## TASK-001 — Verify current source and behavior

**Outcome:** The refactor starts from current `dev`, not historical Plan 031 assumptions.

**Steps:**

- [x] Reverify repository identity, branch, HEAD, and worktree state.
- [x] Read `AGENTS.md`, current `.agents/knowledge/`, canonical memory, `docs/architecture.md`, `docs/development.md`, and relevant relay/package skills.
- [x] Run `pnpm verify:commit` before implementation and record the baseline result.
- [x] Run relevant current MCP/relay acceptance scripts that cover the surfaces to be moved.
- [x] Inventory maintained source line counts and direct source-file counts per folder.
- [x] Record every >500-line maintained production source file and every >15-file implementation folder.
- [x] Distinguish true cohesive exceptions from mixed-responsibility hotspots.

**Validation:**

- baseline commit gate is green or any pre-existing failure is explicitly recorded before refactor;
- inventory is reproducible by deterministic commands/scripts.

## TASK-002 — Build responsibility/dependency map

**Outcome:** Each major hotspot has a proposed decomposition based on reasons-to-change.

For each target module, identify:

- public API / callers;
- domain/application responsibility;
- infrastructure responsibility;
- shared policy/validation ownership;
- security boundaries;
- telemetry ownership;
- likely stable facade/re-export required during migration.

Do not move a file until its target owner is clear.

**Phase exit criteria:**

- [x] baseline is verified;
- [x] responsibility map covers every hard-threshold violation;
- [x] proposed folder/module boundaries are reviewed for DRY/YAGNI/KISS.

**Commit boundary:** documentation/evidence-only baseline commit only if repository convention needs persistent evidence; otherwise proceed without a mechanical checkpoint commit.

### Phase-01 reconciliation evidence — 2026-08-17

- Repository identity reverified at the implementation checkout: canonical root `/home/farismnrr/Projects/MasihAwam/ai-code`, origin `https://github.com/farismnrr/ai-code.git`, implementation branch `refactor/039-maintainability-foundation`, and no active Git mutation lock or competing repository writer. The branch was created from the verified `dev` baseline before Phase-02 work.
- Mandatory guidance was read from current source before implementation: `AGENTS.md`, `.agents/knowledge/*`, `.agents/memories/README.md`, `docs/architecture.md`, `docs/development.md`, `packages/rust-tools/README.md`, `packages/relay-agent/SKILL.md`, and the relevant `ai-self` policies/skills. A later documentation drift in `packages/relay-agent/SKILL.md` (stale Docker wording) is explicitly queued for mandatory Phase-08 synchronization rather than treated as source truth.
- The initial pre-implementation `pnpm verify:commit` run on verified `dev` passed. Phase-02 subsequently re-ran Rust formatting/check/Clippy/tests and the applicable Plan-038 security/black-box scripts, all green; the detailed results are recorded in the Phase-02 closure evidence below.
- Reproducible baseline inventory identified five maintained production Rust files above 500 physical lines: `application/src/execution.rs` (1,319), `application/src/workspace.rs` (1,239), `infrastructure/src/transport.rs` (1,173), `interfaces/src/mcp.rs` (666), and `core/src/config.rs` (539). The only direct maintained implementation folder above 15 files was `app/composables/` with 16. Generated `target/`, lockfiles, migrations, evidence/fixtures, and vendored content are not maintained-source budget inputs.
- Responsibility classification found no cohesive exception among those five >500 production files: `execution.rs` mixed job lifecycle/process/sandbox/request translation; `workspace.rs` mixed traversal/search/read/mutation; `transport.rs` mixes router/bootstrap with MCP request orchestration; `mcp.rs` mixes protocol/result types with the concrete tool catalog/schema; `config.rs` mixes CLI declaration, runtime config, and fail-closed validation. `app/composables/` is a folder-density review hotspot rather than an automatic violation-by-design; Phase-05 owns the cohesion fix.
- Responsibility/dependency map for every hard-threshold file:

  | Baseline hotspot | Public callers / stable facade | Target responsibility boundary | Security / policy owner preserved |
  | --- | --- | --- | --- |
  | `application/src/execution.rs` | application dispatch + relay transport | facade/job lifecycle; `execution/process`; `execution/sandbox`; `execution/requests` | `relay_core::terminal_policy`, one Bubblewrap builder, one process runner |
  | `application/src/workspace.rs` | application dispatch | facade + list/search/read/mutate modules sharing `workspace/secure` | core path containment + no-follow/atomic secure workspace primitives |
  | `infrastructure/src/transport.rs` | CLI `create_router_with_jobs` | router/bootstrap composition separate from MCP method/request orchestration | existing `auth`, `security`, admission, telemetry modules remain authoritative |
  | `interfaces/src/mcp.rs` | transport/application | protocol/result types separate from one concrete catalog/schema owner behind stable re-exports | catalog remains the single owner of names, annotations, OAuth schemes, schemas |
  | `core/src/config.rs` | CLI/application/infrastructure | CLI shape/runtime config separated from validation only where it reduces independent reasons-to-change | fail-closed config validation remains single-source and API-compatible |

- DRY/YAGNI/KISS review: the map uses cohesive Rust modules and stable facades only; it does not introduce new crates, DI containers, repositories/services, plugin frameworks, event buses, or one-file-per-function fragmentation.

**Phase-01 status:** CLOSED / VERIFIED. Phase-02 remains CLOSED / VERIFIED; Phase-03 is the next implementation phase.

---

# PHASE-02 — Refactor Rust application workspace/execution modules

**Goal:** Split the largest Rust application modules into cohesive capability modules while preserving one execution/security path.

## TASK-201 — Decompose `application/src/execution.rs`

**Outcome:** Job lifecycle, command/process execution, sandbox invocation, and tool-specific request translation no longer share one 1,300+ line module without boundaries.

**Steps:**

- [x] Identify the stable public application API used by transport/CLI callers.
- [x] Keep one authoritative process execution/sandbox path; do not duplicate command spawning between tools.
- [x] Separate job lifecycle/state retention from synchronous invocation construction where ownership differs.
- [x] Separate terminal/http/search request translation only when it reduces independent reasons-to-change.
- [x] Preserve timeout, cancellation, process-group cleanup, bounded output, Docker/Tailscale opt-ins, credential masking, and Bubblewrap behavior exactly.
- [x] Use a small `mod.rs`/facade rather than forcing all callers to know internal files.

## TASK-202 — Decompose `application/src/workspace.rs`

**Outcome:** list/search/read/mutate capabilities have focused modules and share one secure path/mutation foundation.

**Steps:**

- [x] Group directory listing and file traversal separately from mutation logic where responsibilities differ.
- [x] Keep shared secure directory/no-follow/path-containment primitives authoritative and DRY.
- [x] Keep file write/edit atomicity and stale-entry protections unified rather than duplicated.
- [x] Preserve current schemas/result behavior and hard output limits.
- [x] Avoid one source file per tiny tool handler when multiple handlers share one cohesive capability module.

**Validation:**

- `cargo fmt --all -- --check`;
- warnings-denied Cargo check/Clippy through repository commands;
- current deterministic Plan 038 workspace/security acceptance checks;
- diff review confirms no weakened no-follow/atomic/sandbox behavior.

**Phase exit criteria:**

- [x] Rust application hard file-size violations are removed or narrowly justified;
- [x] common security/execution primitives remain single-source;
- [x] no new behavior/API regression is observed.

**Commit boundary:** `refactor(relay): split execution and workspace capabilities`


### Phase-02 closure evidence — 2026-08-17

- Independent diff review found and fixed one material sandbox-centralization drift: the shared read-only `text_search` profile had inherited `/opt` and relay-binary-directory mounts that were not present before centralization. The shared Bubblewrap builder now preserves the narrower historical read-only mount surface while writable execution retains its existing mounts and Docker/Tailscale socket opt-ins.
- Application Rust hard-size threshold is satisfied: the largest files in `application/src` are 454 lines; `execution/requests.rs` is 412 lines (<= 500).
- `cargo fmt --all -- --check` — PASS.
- `RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked` — PASS.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — PASS.
- `cargo test --workspace --locked` — PASS (3 relay-core terminal-policy tests, 11 relay-infrastructure security tests, all remaining workspace/doc test targets clean).
- `scripts/verify-workspace-path-security.sh` — PASS.
- `scripts/verify-workspace-v1-integration.sh` — PASS.
- `scripts/phase4-black-box.sh` — PASS.
- Targeted local Phase-02 sandbox/security black-box — PASS: protected `.ssh`/`.npmrc` handling, read-only text-search no-mutation behavior, localhost network visibility, bounded stdout/stderr retention with omitted-byte accounting, running-job cancellation with descendant cleanup, Docker socket absent by default and exposed only with explicit opt-in, and Tailscale explicit opt-in failing closed when the host socket is unavailable.

**Phase-02 status:** CLOSED / VERIFIED. Phase-03 and later 039A work remain untouched by this closure.

---

# PHASE-03 — Refactor Rust transport/interface/core hotspots

**Goal:** Make protocol, transport, auth/security composition, catalog/schema, and configuration responsibilities easier to change independently.

## TASK-301 — Decompose relay transport

- [x] Keep HTTP/router/bootstrap composition separate from MCP method handling when responsibilities are independently changeable.
- [x] Preserve the current ordering of host/origin validation, trusted proxy handling, OAuth/authentication, admission/rate controls, schema validation, and tool dispatch.
- [x] Reuse existing focused `auth.rs`, `security.rs`, telemetry, and admission modules rather than recreating their policy in transport submodules.
- [x] Keep external error text bounded/sanitized.

## TASK-302 — Decompose MCP interface/catalog ownership

- [x] Separate reusable MCP protocol/result types from the concrete tool catalog/schema declarations if this reduces coupling.
- [x] Keep tool names, annotations, OAuth schemes, and JSON schemas in one obvious catalog owner.
- [x] Prevent schema definitions from being copied into application execution code.
- [x] Preserve client-visible `tools/list` contract unless an independently reviewed correction is required.

## TASK-303 — Review configuration ownership

- [x] Split `core/src/config.rs` only along real cohesive boundaries such as CLI shape, validated server configuration, or reusable path/toolchain validation.
- [x] Do not create a configuration framework.
- [x] Preserve all fail-closed validation and current environment/CLI compatibility.

**Validation:**

- existing relay black-box/security/protocol verification;
- `pnpm verify:commit` before commit;
- compare discovered MCP tool catalog against pre-refactor contract.

**Phase exit criteria:**

- [x] no unreviewed Rust maintained-source file remains above the hard line threshold;
- [x] transport remains composition rather than policy dumping ground;
- [x] protocol/catalog ownership is discoverable and DRY.

**Commit boundary:** `refactor(relay): clarify transport protocol and config ownership`

### Phase-03 closure evidence — 2026-08-17

- `infrastructure/src/transport.rs` is now a 241-line router/state/composition facade. HTTP access/OAuth admission lives in `transport/access.rs` (448), MCP request/protocol routing in `transport/mcp_http.rs` (326), and tool-call adaptation in `transport/tools.rs` (214). Existing `auth.rs`, `security.rs`, admission, observability, and telemetry remain the authoritative policy implementations.
- Security-order review confirmed admission precedes trusted-proxy/OAuth work; JWT structural/signature/claim validation remains in the access layer; content-type/body parsing and routing-header validation remain before application dispatch; schema validation remains before tool execution. External auth/process errors remain bounded/sanitized.
- `interfaces/src/mcp.rs` is now a 307-line protocol/result module with one `mcp/catalog.rs` owner (369) for tool names, descriptions, annotations, OAuth schemes, JSON schemas, lookup, and argument validation. Existing public re-exports preserve callers and `tools/list`.
- `core/src/config.rs` is now 394 lines; Clap CLI declarations/security-mode/stop-command ownership moved cohesively to `config/cli.rs` (151) behind stable re-exports. `ServerConfig` defaults, conversion, execution-root validation, trusted-proxy/host/OAuth/socket/toolchain fail-closed checks remain single-source.
- No maintained Rust production file remains above 500 lines after Phases 02–03.
- Validation PASS: `cargo fmt --all -- --check`; warnings-denied workspace Cargo check; workspace Clippy; `cargo test --workspace --locked`; `scripts/phase4-black-box.sh`; `scripts/phase7-external-mcp-contract.sh`; `scripts/phase8-zero-bypass.sh`; `scripts/verify-workspace-v1-integration.sh`; and `pnpm verify:commit`.
- Adversarial review found no broadened authorization, config, catalog, or execution boundary. No new crate/framework/dependency was introduced.

**Phase-03 status:** CLOSED / VERIFIED.


---

# PHASE-04 — Refactor server TypeScript by layer and feature

**Goal:** Ensure server code stays aligned with the documented layered architecture before new agent orchestration is added.

## TASK-401 — Audit dependency direction

- [x] Re-run and inspect `scripts/check-architecture.sh` coverage against current source.
- [x] Find application imports of infrastructure/Drizzle/H3/provider/MCP concrete types that evade intended ownership.
- [x] Find API routes that own business/persistence logic rather than request adaptation.
- [x] Find infrastructure modules that duplicate application policy instead of implementing an application-owned contract.

## TASK-402 — Split mixed-responsibility server hotspots

Prioritize verified hotspots such as:

- `server/infrastructure/ai/langgraph/langgraph-chat.ts`;
- `server/infrastructure/mcp/client.ts`;
- any newly measured >400/>500-line files;
- any feature directory whose direct implementation count violates the foldering budget.

Rules:

- keep provider/MCP/LangGraph SDK-specific code in infrastructure;
- keep use-case/approval/orchestration semantics in application-owned modules/contracts;
- keep transport parsing/auth/response shaping at API boundary;
- preserve one owner for error classification and telemetry sanitation.

## TASK-403 — Remove real duplication

- [x] consolidate repeated policy/metadata/constants only when they represent the same rule;
- [x] remove dead wrappers and pass-through modules created by historical refactors where they no longer create a boundary;
- [x] avoid moving stable tiny helpers simply for symmetry.

**Validation:**

- architecture checker with negative fixtures where new dependency rules need enforcement;
- lint/typecheck;
- relevant server deterministic acceptance checks;
- production build if server bundling/auto-import boundaries moved.

**Phase exit criteria:**

- [x] current server layering matches documentation in actual imports;
- [x] no oversized mixed-responsibility server source remains without justification;
- [x] no new service/DI abstraction was introduced without a concrete need.

**Commit boundary:** `refactor(server): tighten layered feature ownership`

### Phase-04 closure evidence — 2026-08-17

- `scripts/check-architecture.sh` PASS and its current checks cover application→infrastructure/Drizzle leakage plus API→database/infrastructure bypasses; direct source grep found no application imports of infrastructure, Drizzle, H3, provider SDK, or MCP concrete implementation types.
- API-route inventory found no oversized route (`telemetry.post.ts` is the largest at 149 lines); routes remain request adapters rather than hidden persistence/service layers under the current mechanical ownership checks.
- `server/infrastructure/ai/langgraph/langgraph-chat.ts` (341) was reviewed as one LangGraph-specific streaming adapter: SDK message conversion, event translation, provider/tool invocation, cancellation/error sanitation are all integration concerns for the same adapter and remain below the >400 review threshold. Splitting it now would add indirection without a hard/cohesion violation.
- `server/infrastructure/mcp/client.ts` (316) was reviewed as one outbound MCP infrastructure adapter: first-party protocol compatibility, SSRF-safe transport selection, credential owner binding, and generic SDK fallback are coupled at the transport boundary. Its security rules remain infrastructure-owned and it is below the >400 review threshold.
- No server implementation folder exceeds the >15 hard budget; `server/infrastructure/database` has 12 direct TypeScript files and remains a cohesive database-adapter folder within the target range.
- DRY review found no duplicated application policy introduced by this refactor; no service container, DI framework, or new generic abstraction was added.
- `pnpm verify:commit` PASS after the Rust refactor, including architecture, ESLint, Vue TypeScript, warnings-denied Rust Clippy/check.

**Phase-04 status:** CLOSED / VERIFIED with no source move required beyond the already-clean layered architecture.


---

# PHASE-05 — Refactor frontend composition, foldering, and large views

**Goal:** Keep route/layout files as composition roots and prevent flat feature folders from becoming maintenance bottlenecks.

## TASK-501 — Review page/component responsibility

Prioritize:

- `app/pages/chat/[id].vue`;
- `app/pages/settings/mcp.vue`;
- `app/pages/settings/local-terminal.vue`;
- large settings/chat components reported by the fresh line-count inventory.

For each page:

- [x] keep routing/loading/page composition in the page;
- [x] extract reusable UI into feature components only when it has a stable UI contract;
- [x] extract reactive behavior into cohesive composables/controllers only when it has independent lifecycle/state ownership;
- [x] keep pure transformations outside Vue files where they have independent reuse/value;
- [x] avoid flag-heavy “universal” components.

## TASK-502 — Review `app/composables` foldering

- [x] classify composables by feature and public/internal status;
- [x] preserve Nuxt auto-import semantics for public `use*` entrypoints;
- [x] group internal controllers/helpers under feature subfolders where it materially reduces the flat namespace;
- [x] do not create extra wrapper files unless they preserve a framework/public API boundary.

## TASK-503 — Review component folder density

- [x] ensure chat/settings/workspace/shell feature folders remain cohesive;
- [x] create deeper subfolders only when a feature has distinct sub-capabilities with multiple files;
- [x] keep cross-feature primitives clearly separate from feature-owned implementation.

**Validation:**

- `pnpm verify:commit`;
- `pnpm build` where Nuxt file moves/auto-imports are involved;
- browser/runtime smoke verification of affected pages;
- no SSR composable-context regressions.

**Phase exit criteria:**

- [x] frontend pages above the review threshold have been assessed and split where responsibility warrants;
- [x] flat-folder violations are removed or justified;
- [x] user-visible behavior remains unchanged.

**Commit boundary:** `refactor(app): improve feature cohesion and composition boundaries`

### Phase-05 closure evidence — 2026-08-17

- Priority pages were reviewed directly: `app/pages/chat/[id].vue` (344), `app/pages/settings/mcp.vue` (325), and `app/pages/settings/local-terminal.vue` (302) remain below the >400 review threshold and act as route/form/UI composition roots while chat, MCP, and relay behavior remains delegated to existing feature composables/controllers.
- Component-folder density remains cohesive (`chat` 8 direct files; `settings`, `workspace`, and `shell` 2 each), so no metric-only subfolder split was introduced.
- `app/composables/` has 16 direct files, all public Nuxt `use*` auto-import entrypoints. A controlled feature-folder experiment proved nested movement removed those entrypoints from generated `.nuxt/imports.d.ts`; the experiment was fully reverted. The final tree keeps the 16-file public framework surface as one exact, reasoned cohesion exception while existing internal chat controllers remain under `app/composables/chat/`.
- `pnpm exec nuxt prepare --dotenv .env.example` regenerated types and explicitly proved all 16 public composables remain auto-imported.
- `pnpm verify:commit` PASS and `pnpm build` PASS after the frontend audit.
- Production preview smoke on `127.0.0.1:31333`: `/` returned 200; `/chat/phase05-smoke`, `/settings/mcp`, and `/settings/local-terminal` returned the expected 302 login redirect. No SSR/composable-context regression was observed.

**Phase-05 status:** CLOSED / VERIFIED. No user-visible frontend behavior change was retained.


---

# PHASE-06 — Repository-wide DRY / SOLID / YAGNI / KISS audit

**Goal:** Review the refactored repository for architectural quality that line/folder metrics cannot detect.

## TASK-601 — DRY audit

Search for:

- duplicated tool names/descriptions/limits;
- duplicated path/security/approval rules;
- duplicated provider/model option knowledge;
- duplicated mapping/conversion logic;
- duplicate error/result shaping.

Extract only duplicated **knowledge** with one real owner.

## TASK-602 — SOLID/layering audit

- [x] identify modules with multiple unrelated reasons-to-change after decomposition;
- [x] identify broad interfaces/dependency objects that force consumers to depend on unused methods;
- [x] identify infrastructure types leaking into application/core contracts;
- [x] identify central switches that should use an existing extension seam versus switches that are simpler and should remain explicit.

## TASK-603 — YAGNI/KISS deletion pass

- [x] remove obsolete compatibility wrappers no longer needed after callers migrate;
- [x] remove speculative abstractions with one trivial consumer;
- [x] simplify nested indirection where it does not enforce a boundary;
- [x] remove dead code/imports/comments that describe superseded architecture.

**Phase exit criteria:**

- [x] independent review finds no obvious repository-wide DRY/layering regression introduced by the refactor;
- [x] module count/indirection has not exploded merely to lower line counts;
- [x] final architecture is simpler to navigate than baseline.

**Commit boundary:** `refactor: remove post-decomposition duplication and indirection`

### Phase-06 closure evidence — 2026-08-17

- Repository-wide DRY/security/policy searches found one real duplicated piece of knowledge in the MCP catalog: the identical OAuth2 `relay.coding` security scheme was repeated for every tool. It now has one catalog-local owner (`CODING_SCOPE` + `coding_security_scheme()`), with no behavior/schema change; targeted `cargo clippy -p relay-interfaces --all-targets --all-features --locked -- -D warnings` PASS.
- Application/core dependency review found no concrete infrastructure types leaking into application/core contracts; existing application dependency objects remain capability/use-case scoped rather than service locators.
- Post-split Rust facades remain intentionally small/stable while cohesive submodules own real responsibilities; no one-file-per-function fragmentation, DI container, event bus, generic repository/service framework, or speculative plugin layer was introduced.
- Tiny API/adapter/facade modules were reviewed and retained only where they enforce a transport/layer/public boundary; no compatibility wrapper was removed merely to reduce module count.
- Independent read-only external MCP client review of the uncommitted Plan-039A diff reported no DRY/layering/sandbox/OAuth/Nuxt P0/P1 regression. It raised maintainability-check coverage concerns handled in Phase-07: stylesheet-family coverage was broadened; its claimed root `tests/` gap was not applicable because the repository has no tracked/root `tests/` tree and the checker’s contract is maintained production source.

**Phase-06 status:** CLOSED / VERIFIED. The final architecture has fewer mixed-responsibility hotspots without speculative indirection.


---

# PHASE-07 — Make maintainability rules mechanically enforceable

**Goal:** Prevent the repository from immediately regressing to giant files and dumping-ground folders.

## TASK-701 — Add deterministic maintainability policy check

Create a small repository-native checker using an already-available runtime; do not add a dependency solely for this check.

The checker must:

- [x] count physical lines for maintained source categories;
- [x] count direct maintained implementation files per relevant folder;
- [x] exclude generated/vendor/migration/evidence/lockfile categories explicitly;
- [x] fail on >500-line maintained production source unless explicitly allowlisted with a reason;
- [x] fail on >15 direct implementation files in one folder unless explicitly allowlisted with a cohesion reason;
- [x] report review-threshold (>400 lines / 13–15 files) findings clearly without hiding them;
- [x] use one authoritative configuration/source for thresholds and exceptions;
- [x] reject wildcard/broad exceptions that would make the gate meaningless.

Do not attempt cross-language AST function-length enforcement in this phase. Function length remains a review rule because building a parser framework would violate YAGNI.

## TASK-702 — Integrate with canonical local gate

- [x] integrate the maintainability checker into `pnpm verify:commit` / existing repository-policy flow in the smallest appropriate location;
- [x] preserve the repository's no-CI/no-unit-test policy;
- [x] add deterministic negative fixtures/probes only where needed to prove the checker rejects representative violations;
- [x] never create an easy bypass separate from the normal commit gate.

**Phase exit criteria:**

- [x] current refactored tree passes the new maintainability check with no unexplained hard violations;
- [x] representative oversized-file and overfull-folder fixtures are rejected;
- [x] `pnpm verify:commit` remains the single mandatory commit gate.

**Commit boundary:** `chore(repo): enforce maintainability budgets`

### Phase-07 closure evidence — 2026-08-17

- Added dependency-free `scripts/check-maintainability.mjs` with one authoritative policy object for roots/extensions/exclusions/thresholds/exceptions. It counts physical maintained-source lines and direct maintained implementation files, reports >400-line and 13–15-file review findings, and fails unexplained >500-line files or >15-file folders.
- Generated/build/vendor/migration/evidence-style paths are excluded explicitly; exact exceptions reject `*`, `?`, trailing-slash, dot, empty, or reasonless entries. The only current folder exception is exact `app/composables` with the verified Nuxt public-auto-import cohesion reason.
- Source coverage includes current maintained JS/TS/Vue/Rust/CSS plus common Nuxt stylesheet families (`.scss`, `.sass`, `.less`, `.styl`, `.stylus`) so a supported style-file change cannot trivially evade the budget. No tracked root `tests/` source exists; the production-source checker does not invent a test-suite policy the repository explicitly does not have.
- `node scripts/check-maintainability.mjs` PASS: 239 maintained source files checked; current review findings are explicit and there are no unexplained hard violations.
- `node scripts/check-maintainability.mjs --self-test` PASS, proving representative 501-line and 16-direct-file fixtures are rejected.
- Integrated the checker directly into `scripts/verify-commit.sh` between architecture and lint/type gates; no separate bypass command or second threshold source was added. Repository no-CI/no-conventional-unit-suite policy remains unchanged.

**Phase-07 status:** CLOSED / VERIFIED. `pnpm verify:commit` remains the single mandatory commit gate.


---

# PHASE-08 — Mandatory documentation and agent-guide update

**Goal:** Make the final architecture and maintainability rules discoverable to humans and every coding agent.

**This phase is mandatory. Plan 039A cannot close if code changed but documentation/guidance still describes the pre-refactor structure.**

## TASK-801 — Update operator/developer documentation

At minimum review and update where applicable:

- `README.md`;
- `docs/architecture.md`;
- `docs/development.md`;
- `docs/README.md`;
- package-level relay/tool documentation when file/module ownership moved.

Document:

- final high-level architecture and module ownership;
- maintainability budgets and what they mean;
- the canonical local enforcement command;
- exemption policy and why thresholds are guardrails rather than design substitutes;
- how future contributors should choose a folder/module owner.

Do not publish stale internal paths that are not intended as stable operator interfaces unless the architecture document genuinely needs them.

## TASK-802 — Update agent-facing guidance

At minimum review and update:

- `AGENTS.md` if its concise repository entrypoint needs a new invariant;
- `.agents/knowledge/project.md`;
- `.agents/knowledge/conventions.md`;
- `.agents/knowledge/tooling.md` where gate/tooling changes apply;
- `.agents/knowledge/self-improvement.md` where maintainability review belongs in completion workflow;
- `.agents/memories/README.md` for durable architecture/maintainability decisions;
- relevant `ai-self` skill(s) if execution exposed a reusable implementation/refactoring lesson.

Agent guidance must explicitly carry forward:

1. DRY;
2. pragmatic SOLID;
3. Layered Architecture;
4. YAGNI;
5. KISS;
6. folder cohesion/file-count guardrails;
7. source-file-size guardrails;
8. mandatory docs/guide synchronization after architecture changes.

Do not duplicate long policy text into every guide. Keep one authoritative detailed policy and concise links/summaries elsewhere.

## TASK-803 — Documentation integrity validation

- [x] run `scripts/check-agent-docs.sh`;
- [x] verify plan/knowledge/docs links after file moves;
- [x] grep for superseded module paths/architecture statements;
- [x] ensure canonical memory describes only verified final state, not planned state.

**Phase exit criteria:**

- [x] operator docs match current source;
- [x] coding-agent guidance matches current source;
- [x] maintainability policy is discoverable from both developer and agent entrypoints;
- [x] no stale architecture statement found in relevant canonical docs.

**Commit boundary:** `docs: document maintainability and refactored architecture`

### Phase-08 closure evidence — 2026-08-17

- Updated human/operator guidance in `README.md`, `docs/architecture.md`, `docs/development.md`, `docs/README.md`, `packages/rust-tools/README.md`, and `packages/relay-agent/SKILL.md` for final module ownership, maintainability budgets/checker, exact-exception policy, and documentation-sync expectations.
- Corrected stale relay guidance that still called Docker unsupported: current docs now match source—Docker is default-denied and an explicit trusted local opt-in with effectively host-level authority; Tailscale remains a separate explicit socket opt-in. Relay skill tool inventory now includes the six native Plan-038 workspace tools as well as execution/network/job tools.
- Updated `AGENTS.md`, `.agents/knowledge/{project,conventions,tooling,self-improvement}.md`, and canonical `.agents/memories/README.md` with the final responsibility splits, maintainability rules, public Nuxt-composable exception, and mandatory docs/agent-guide synchronization.
- Improved existing `ai-self/skills/implementation-planning/SKILL.md` rather than creating a duplicate skill: structural-refactor planning now treats metrics as signals, maps callers/policy owners before splits, and prefers exact framework exceptions over wrapper spam.
- `bash scripts/check-agent-docs.sh` PASS; `bash scripts/verify-workspace-docs.sh` PASS; custom relative-Markdown-link validation PASS; canonical-doc stale-path sweep for moved Rust facade paths PASS; contradictory old Docker wording sweep PASS; `git diff --check` PASS.

**Phase-08 status:** CLOSED / VERIFIED. Operator and agent guidance now describes the final Plan-039A architecture rather than the pre-refactor layout.


---

# PHASE-09 — Full validation, independent review, and closure

**Goal:** Prove the refactor preserved behavior/security and leaves a sound foundation for Plan 039B.

## TASK-901 — Full repository validation

Run at minimum:

- [x] `pnpm verify:commit`;
- [x] `pnpm build`;
- [x] `pnpm build:tools`;
- [x] relevant deterministic MCP/relay workspace/security/authorization scripts affected by moved code;
- [x] `pnpm audit` / `cargo audit` if dependency manifests changed (dependency changes are not expected) — N/A: no dependency manifest or lockfile changed in 039A;
- [x] browser/runtime smoke checks for moved frontend areas;
- [x] actual MCP discovery/call smoke tests if relay catalog/dispatch files moved.

Do not claim a check passed unless it actually ran on the final state.

## TASK-902 — Independent architecture/maintainability review

Ask an independent focused reviewer/subagent to inspect the final diff/source for:

- DRY regressions;
- unnecessary abstractions;
- crossed layer dependencies;
- folders split only for metrics rather than cohesion;
- files still carrying multiple independent reasons-to-change;
- hidden security behavior changes;
- stale docs/guide references;
- maintainability-check bypasses or over-broad exceptions.

Fix P0/P1 findings before closure. Material P2 maintainability findings should also be fixed unless explicitly documented as a bounded follow-up with rationale.

**Independent-review reconciliation (2026-08-17):**

- the original reviewer job `8dcc51fd-87c8-4f1b-b144-6e1045aeee56` became unrecoverable; replacement reviewer `d867ed10-2cb2-47a1-998b-6da6dc4a61a1` completed against a hard read-only snapshot and returned substantive findings;
- the claimed admission-gate ordering regression was rejected after direct source verification: non-`/mcp` requests return before MCP request-admission acquisition;
- the sandbox behavior finding was documentation-only after the Phase-02 security fix; the comment was clarified and the read-only search mount surface remains narrower than writable execution;
- the claimed duplicated terminal parsing was rejected after call-chain verification: synchronous execution and terminal jobs share `build_terminal_exec_invocation`;
- workspace result serialization/output-bound adaptation was a valid ownership finding and moved into `application::workspace::dispatch`;
- the review's general audit-ordering rationale did not apply to ordinary tool dispatch, but investigation exposed a real pre-existing gap for `terminal_job_start/get/cancel`; the subject-aware remote audit now runs before all terminal-job early returns;
- duplicated `relay.coding` scope knowledge was a valid DRY finding and now has one canonical MCP-interface constant reused by auth/transport;
- a proposed non-Linux abstraction was rejected under YAGNI because the production sandbox contract remains Linux-specific and no current alternate platform requires the seam;
- no unresolved P0/P1 architecture or security finding remains after remediation and final validation.

## TASK-903 — Establish Plan 039B handoff baseline

Record in the master roadmap:

- final branch/commit;
- validation evidence;
- final maintained-source line/folder budget result;
- any explicit allowed exceptions;
- the stable module ownership relevant to Git/LSP/policy work.

**Phase-09 closure evidence (2026-08-17):**

- implementation baseline committed as `1872ca6ff7bf8572e9bf91ce7ff37ca59733749b` on `refactor/039-maintainability-foundation`;
- `pnpm verify:commit` PASS on the final source before closure metadata;
- `pnpm build` PASS and `pnpm build:tools` PASS;
- `cargo test --workspace --locked` PASS (relay-core terminal-policy 3/3; relay-infrastructure security 11/11; remaining targets/doc-tests clean);
- `bash scripts/verify-workspace-path-security.sh` PASS;
- `bash scripts/verify-workspace-v1-integration.sh` PASS, including direct-argv regressions proving leading `--help`, `--locked`, and background `--job-flag` values are passed verbatim;
- `bash scripts/phase4-black-box.sh` PASS;
- `bash scripts/phase7-external-mcp-contract.sh` PASS with frozen catalog hash `ab5b01984b62362bd493052eb7e5d91be0e409172bb5bc6174bdf1c95fb6e456`;
- `bash scripts/phase8-zero-bypass.sh` PASS;
- `node scripts/verify-mcp-tool-result-error-confidentiality.mjs` PASS;
- `node scripts/check-maintainability.mjs --self-test` PASS;
- final production-preview smoke on `127.0.0.1:31333`: `/` returned 200; `/chat/phase09-final-smoke`, `/settings/mcp`, and `/settings/local-terminal` returned the expected 302 authentication redirect; preview job was then cancelled;
- maintained-source hard budgets remain green; review-threshold files are explicit, and the only 16-file folder exception remains the documented cohesive Nuxt `app/composables` public auto-import surface;
- the public MCP catalog stayed frozen while terminal direct-argv behavior was clarified in operator/agent guidance and locked by deterministic runtime regression coverage.

**Phase exit criteria:**

- [x] all hard maintainability-budget violations are eliminated or narrowly justified;
- [x] mandatory docs and agent guides are current;
- [x] full commit gate/build/relevant acceptance checks are green;
- [x] independent review finds no unresolved P0/P1 architecture/security finding;
- [x] Plan 039B can add Git read/patch capability without returning to giant central modules.

**Commit boundary:** final closure/docs correction commit only if needed after review.

---

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Mechanical splitting increases indirection | split only by independent responsibility; use small stable facades; run KISS/YAGNI deletion pass |
| File-count rules create folder spam | treat thresholds as cohesion guardrails; reject one-file folder fragmentation |
| Line-count rules encourage meaningless helper extraction | hard rule requires responsibility review; function-level size remains manual rather than parser-driven |
| Rust security behavior changes during moves | preserve one authoritative execution/path/sandbox implementation and rerun deterministic adversarial checks after each phase |
| Nuxt auto-import/SSR breaks after folder moves | preserve public composable entrypoints; run typecheck/build/browser smoke tests after moves |
| Layering becomes folder-only architecture | architecture checker plus import/dependency review must verify actual direction |
| DRY creates generic abstractions | extract shared knowledge only; YAGNI/KISS review explicitly removes speculative abstractions |
| Documentation drifts | docs/guide update is a blocking phase and agent-doc integrity remains in commit gate |

---

## Final acceptance criteria

Plan 039A is closed only when:

- [x] the refactor demonstrably applies DRY, pragmatic SOLID, Layered Architecture, YAGNI, and KISS;
- [x] no unexplained maintained production source file exceeds the 500-line hard budget;
- [x] no unexplained cohesive implementation folder exceeds 15 direct maintained implementation files;
- [x] >400-line and 13–15-file review-threshold findings have been consciously reviewed;
- [x] giant Rust relay modules have been decomposed by responsibility without duplicating security/execution paths;
- [x] server dependency direction still matches its documented layered architecture;
- [x] frontend page/composable/component ownership is clearer and Nuxt behavior is preserved;
- [x] deterministic maintainability enforcement is part of the canonical local commit gate;
- [x] operator documentation is updated;
- [x] agent-facing guides/knowledge/memory are updated;
- [x] relevant skills are updated if reusable workflow knowledge changed;
- [x] `pnpm verify:commit`, build checks, and relevant black-box/security checks pass on final source;
- [x] independent review finds no unresolved P0/P1 architecture/security regression;
- [x] the master Plan 039 roadmap marks 039A closed before 039B begins.

## Execution handoff

Execute this plan **sequentially**. Do not implement later Plan 039 capabilities in parallel with the refactor. Focused subagents may inspect/review the current phase, but one main worktree owns implementation. Validate each phase before advancing.

After Plan 039A closes, begin **Plan 039B — Git Read Intelligence and Patch Ergonomics** from the verified refactored baseline.
