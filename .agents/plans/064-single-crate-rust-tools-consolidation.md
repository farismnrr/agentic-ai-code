# Plan 064 — Single-Crate Rust Tools Consolidation

**Status:** CLOSED / LOCAL ACCEPTANCE PASSED (2026-09-01)
**Goal:** Consolidate the current five-package Rust implementation into one `ai-tools` Cargo package with one `src/`, one `tests/`, and one dependency manifest, while preserving the existing layered ownership, security boundaries, MCP contracts, CLI behavior, release artifact, and runtime behavior.

**Success Criteria:**
- `packages/rust-tools/` contains exactly one Cargo package.
- Repository root remains a lightweight Cargo workspace so existing root `cargo ...` workflows continue to work.
- One production source tree exists at `packages/rust-tools/src/`.
- One integration-test tree exists at `packages/rust-tools/tests/`.
- One native dependency manifest exists at `packages/rust-tools/Cargo.toml`.
- `ai-tools` remains the only produced native binary.
- `core`, `interfaces`, `application`, and `infrastructure` remain explicit architectural modules.
- No `relay_core`, `relay_application`, `relay_infrastructure`, or `relay_interfaces` package dependency remains.
- Existing MCP schemas, profiles, effects, catalog snapshots, sandbox/security behavior, CLI behavior, and release artifact names remain unchanged.
- Loss of Cargo crate isolation is compensated by deterministic repository architecture guards with negative fixtures.
- `pnpm guardrail` passes after the migration.
- No relay systemd action, deployment, tag, release publish, or production mutation occurs as part of this plan.

## Closure Record

Implementation completed on the isolated `refactor/064-single-crate-rust-tools` branch/worktree from `main` at `f0cf18c`. The initial pre-mutation guardrail attempt surfaced a duplicate Plan 063 key because `main` had already consumed that number for the explicit Telegram message-tool work; the consolidation plan was renumbered to 064 before source mutation. No unrelated dirty checkout changes were copied into the implementation worktree.

Verified closure evidence:

- Cargo metadata reports exactly one workspace package: `ai-tools` v0.0.13.
- `packages/rust-tools/Cargo.toml` is the only Cargo manifest below `packages/rust-tools/`.
- All 15 integration tests and all 20 examples are preserved; `cargo check --workspace --all-targets --all-features --locked` passes.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passes.
- `pnpm guardrail` passes across both web and Rust stacks after active path-contract updates.
- `pnpm audit:rust` exits successfully; the only audit note is the repository-allowed yanked `chacha20 0.10.1` warning, with no blocking RustSec vulnerability.
- Frozen MCP catalog v13 remains exact through `catalog_v13_snapshot_matches_current_static_surface` (102 tools).
- Release build succeeds, `ai-tools --version` reports `0.0.13`, and root plus `terminal`, `curl`, `searxng`, `relay`, and `telegram` help surfaces remain callable.
- Cargo.lock changes are limited to replacing the five internal package nodes with the single `ai-tools` package node; third-party versions are unchanged.
- Active legacy paths/namespaces and nested Cargo manifests are absent.
- No systemd relay action, deployment, tag, release publish, or production mutation was performed.

## Scope

### In scope
- Collapse `packages/rust-tools/{core,application,infrastructure,interfaces,cli}` into one Cargo package.
- Preserve layer directories as modules under one `src/` tree.
- Consolidate Rust integration tests into one package-local `tests/` directory.
- Consolidate examples while respecting maintainability folder budgets.
- Remove internal path dependencies and merge external dependency declarations into one package manifest.
- Rewrite internal Rust namespaces from `relay_*::` crate paths to `crate::*` / `ai_tools::*` as appropriate.
- Update architecture, test-layout, maintainability, build/release, and active documentation paths affected by the consolidation.
- Preserve the current frozen MCP catalog contract exactly unless current `main` has a newer already-reviewed contract when implementation starts.
- Remove obsolete package shells, manifests, compatibility plumbing, and orphaned internal-crate references.

### Out of scope
- Dependency upgrades or feature changes unrelated to package consolidation.
- MCP protocol, tool, schema, profile, effect, approval, task, or activity behavior changes.
- Security policy redesign or weakening.
- SSH policy expansion.
- Nuxt feature changes.
- Database or migration changes.
- Reworking business responsibilities between layers beyond the minimum required for module-path migration.
- New frameworks, proc macros, build scripts, service locators, or DI frameworks.
- Rewriting historical numbered plans solely because they reference historical paths.
- Relay systemd restart/reload/install/deploy.
- Release/tag/publish actions.

## Current State

Verified repository facts at planning time:

- Root Cargo workspace currently contains five members:
  - `packages/rust-tools/core`
  - `packages/rust-tools/application`
  - `packages/rust-tools/infrastructure`
  - `packages/rust-tools/interfaces`
  - `packages/rust-tools/cli`
- The current output is already one binary: `ai-tools`.
- Approximate Rust inventory from the current checkout:
  - 169 total Rust files.
  - 134 source/production files.
  - 15 package-local integration tests.
  - 20 examples.
  - ~110 Rust files reference an internal `relay_*::` crate namespace.
- Current direct source-file counts by layer are approximately:
  - core: 22
  - application: 74
  - infrastructure: 23
  - interfaces: 8
  - cli: 7
- Six current files use `CARGO_MANIFEST_DIR` and need explicit path re-evaluation after package-root movement.
- The repository intentionally has no hosted CI; `pnpm guardrail` is the mandatory local quality gate.
- Current Rust tests are required to live in package-local `tests/` directories; production inline tests are forbidden.
- `scripts/check-maintainability.mjs` enforces >400-line review / >500-line hard file thresholds and 13–15 direct-file review / >15 hard folder thresholds.
- `scripts/check-architecture.sh` currently relies partly on existing Cargo package boundaries and needs stronger Rust module-direction enforcement after consolidation.
- `ops/release/build-artifacts.sh` currently references `packages/rust-tools/cli/Cargo.toml` explicitly and must be updated.
- Several package skills currently use the old CLI manifest path.
- Plan 032 historically introduced the multi-crate layered structure. Its layered responsibility model remains valuable, while its multi-crate mechanism is intentionally superseded by this plan.

## Constraints & Decisions

### D-01 — One Cargo package, not one giant module

The target is one Cargo package containing layered Rust modules, not a flattened source folder.

Target shape:

```text
packages/rust-tools/
├── Cargo.toml
├── package.json
├── README.md
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── commands/
│   ├── core/
│   ├── interfaces/
│   ├── application/
│   └── infrastructure/
├── tests/
└── examples/
```

Existing cohesive subfolders such as `application/execution`, `application/workspace`, `application/git`, `application/lsp`, `infrastructure/transport`, `core/ssh_policy`, and `interfaces/mcp` remain intact.

### D-02 — Keep the repository-root virtual Cargo workspace

Root `Cargo.toml` remains a virtual workspace with one member:

```toml
[workspace]
members = ["packages/rust-tools"]
resolver = "2"
```

This preserves root-level Cargo ergonomics and the repository-root `target/` directory while keeping all actual package metadata/dependencies in `packages/rust-tools/Cargo.toml`.

### D-03 — Package identity becomes `ai-tools`

The consolidated package should use:

```toml
[package]
name = "ai-tools"
```

The binary remains:

```toml
[[bin]]
name = "ai-tools"
path = "src/main.rs"
```

The library crate name therefore becomes `ai_tools` for integration tests, examples, and the binary target.

### D-04 — Preserve the current layer dependency direction

Allowed module dependency graph:

| Module | Allowed dependencies |
| --- | --- |
| `core` | external crates only |
| `interfaces` | `core` |
| `application` | `core`, `interfaces` |
| `infrastructure` | `core`, `interfaces`, `application` |
| `main.rs` / `commands` | all layers as composition edge |
| tests/examples | public surfaces needed for verification |

Forbidden examples:

```text
core -> interfaces/application/infrastructure
interfaces -> application/infrastructure
application -> infrastructure
```

Do not simultaneously redesign the architectural direction during this migration.

### D-05 — Replace lost Cargo isolation with deterministic guardrails

The current multi-crate graph provides compiler-level dependency isolation. A single crate weakens that isolation. Plan 064 is not complete unless `scripts/check-architecture.sh` explicitly enforces Rust module direction and proves the guard with negative fixtures.

### D-06 — No compatibility wrapper crates

Do not keep stub packages such as `relay-core` that merely re-export `ai_tools::core`. The goal is to remove package-level plumbing, not recreate it as indirection.

### D-07 — No dependency churn

Merge the exact union of current external dependencies/features into the new package manifest. No dependency upgrade, new dependency, or feature broadening should occur unless a build blocker is separately justified and reviewed.

### D-08 — Current public contracts remain frozen

The refactor is internal. MCP catalog/schema/profile/effect behavior, CLI commands, release artifact names, OAuth/security behavior, sandbox behavior, task lifecycle, activity behavior, and SSH behavior remain unchanged.

## Phase Overview

| Phase | Goal | Depends On | Exit Criteria |
| --- | --- | --- | --- |
| PHASE-00 | Freeze a known-good baseline and isolate work | none | Current Rust gates/build/contracts are known-good; dedicated task branch/worktree used when Git tooling is healthy |
| PHASE-01 | Establish one-package Cargo topology | 00 | Cargo metadata reports one `ai-tools` workspace package |
| PHASE-02 | Consolidate production source and namespaces | 01 | One `src/`; no legacy package source trees; code compiles |
| PHASE-03 | Consolidate tests and examples | 02 | One `tests/`; all tests/examples compile; manifest-root assumptions fixed |
| PHASE-04 | Restore architecture enforcement | 02 | Invalid Rust layer dependencies fail deterministic guard fixtures |
| PHASE-05 | Update build/release/tooling paths | 01–04 | No active tooling references removed manifests |
| PHASE-06 | Reconcile docs and durable guidance | 01–05 | Current documentation matches the one-crate architecture |
| PHASE-07 | Full behavioral/security validation | 01–06 | Rust gates, audit, catalog parity, and binary parity pass |
| PHASE-08 | Orphan sweep and closure | 07 | No active legacy package plumbing remains; plan truthfully closed |

# PHASE-00 — Baseline and isolation

**Goal:** Establish a reproducible known-good baseline before structural mutation.
**Dependencies:** none

## TASK-001 — Revalidate repository and Git state

**Outcome:** Implementation starts from the correct repository and does not mix with unrelated work.

**Steps:**
- [x] Re-resolve repository root and origin using current source, not memory.
- [x] Read current `main`; Plan 063 was already occupied by the explicit Telegram message-tool work, so this consolidation was renumbered to Plan 064 before implementation.
- [x] Confirm the task branch/worktree is based on current `main`.
- [x] Do not reuse an unrelated feature/docs branch for implementation.
- [x] Record current `Cargo.toml`, current package list, current catalog snapshot, and current version before mutation.

**Validation:**
- `cargo metadata --locked --no-deps --format-version 1` successfully reports the current baseline packages.
- `git status --short` is clean in the implementation worktree before source changes.

**Commit boundary:** none; baseline only.

## TASK-002 — Prove baseline Rust behavior

**Outcome:** Later failures can be classified as migration regressions rather than pre-existing failures.

**Steps:**
- [x] Run the current Rust guardrail.
- [x] Build the current release binary.
- [x] Run current catalog/surface tests.
- [x] Capture only command/pass-fail evidence in the plan; do not create plan-numbered verification scripts.

**Validation:**
- `pnpm guardrail:rust`
- `cargo check --workspace --all-targets --all-features --locked`
- `cargo build --release --locked --bin ai-tools`
- `target/release/ai-tools --version`
- `target/release/ai-tools --help`

**Phase exit criteria:**
- [x] Known-good current Rust baseline established.
- [x] Implementation work isolated from unrelated changes.

# PHASE-01 — Single Cargo package topology

**Goal:** Make `packages/rust-tools` the only Rust package without changing runtime behavior.
**Dependencies:** PHASE-00

## TASK-003 — Create consolidated package manifest

**Outcome:** `packages/rust-tools/Cargo.toml` owns package metadata, features, targets, and external dependencies.

**Files:**
- Create: `packages/rust-tools/Cargo.toml`
- Modify: root `Cargo.toml`
- Regenerate/reconcile: `Cargo.lock`

**Steps:**
- [x] Define `[package] name = "ai-tools"`, current version, edition, and MSRV.
- [x] Define the `ai-tools` binary at `src/main.rs`.
- [x] Preserve `test-gh-provider` as a non-default feature.
- [x] Merge the exact union of existing external dependencies and features.
- [x] Preserve explicit feature-gated example declarations used by existing GitHub fixture examples.
- [x] Add explicit example declarations where nested example paths require them.
- [x] Do not upgrade or add dependencies.

**Validation:**
- `cargo metadata --no-deps --format-version 1` parses the new package.
- Cargo metadata reports package name `ai-tools`.

**Commit boundary:** part of the coherent structural refactor commit; do not commit a broken intermediate state.

## TASK-004 — Reduce root workspace to one member

**Outcome:** Root Cargo workspace has one member and no internal `relay-*` path dependencies.

**Steps:**
- [x] Change `[workspace].members` to only `packages/rust-tools`.
- [x] Keep root release profile unchanged.
- [x] Move actual package dependencies out of `[workspace.dependencies]` into the package manifest unless a root-level declaration remains necessary for a proven reason.
- [x] Remove `relay-core`, `relay-application`, `relay-infrastructure`, and `relay-interfaces` path dependency declarations.

**Validation:**
- `cargo metadata --no-deps --format-version 1` reports exactly one workspace package.
- No internal `relay-*` package is present in metadata.

**Phase exit criteria:**
- [x] One Cargo package owns all native code.
- [x] Root remains a virtual one-member workspace.

# PHASE-02 — Production source consolidation

**Goal:** Move all production Rust code into one source tree while preserving responsibility boundaries.
**Dependencies:** PHASE-01

## TASK-005 — Move layer source trees

**Outcome:** Current layer implementations become modules under `packages/rust-tools/src/`.

**Mapping:**

```text
core/src/*            -> src/core/*
application/src/*     -> src/application/*
infrastructure/src/*  -> src/infrastructure/*
interfaces/src/*      -> src/interfaces/*
```

Convert current crate roots:

```text
core/src/lib.rs            -> src/core/mod.rs
application/src/lib.rs     -> src/application/mod.rs
infrastructure/src/lib.rs  -> src/infrastructure/mod.rs
interfaces/src/lib.rs      -> src/interfaces/mod.rs
```

**Steps:**
- [x] Preserve existing nested responsibility folders.
- [x] Create `src/lib.rs` exposing only the module surfaces required by the binary, tests, and examples.
- [x] Prefer `pub(crate)` for internals where external test/example access is not required.
- [x] Do not expand public visibility merely to make imports convenient.
- [x] Avoid algorithmic edits while moving files.

**Validation:**
- Source tree contains one copy of every production module.
- No behavior-specific diff is introduced beyond path/module plumbing.

## TASK-006 — Move CLI composition into the consolidated package

**Outcome:** The current CLI becomes the package binary/composition edge.

**Mapping:**

```text
cli/src/main.rs      -> src/main.rs
cli/src/commands/*   -> src/commands/*
```

**Steps:**
- [x] Preserve command names and argument behavior.
- [x] Preserve commands `curl`, `relay`, `searxng`, `terminal`, and `telegram`.
- [x] Keep telemetry bootstrap/shutdown behavior unchanged.
- [x] Keep `main.rs` composition-oriented rather than absorbing layer logic.

**Validation:**
- `cargo check --bin ai-tools --locked`

## TASK-007 — Rewrite internal namespace imports correctly

**Outcome:** Active code no longer references removed package names.

**Library-source rule:**
Inside `src/core/**`, `src/application/**`, `src/infrastructure/**`, and `src/interfaces/**`, convert cross-layer references to `crate::...`.

Examples:

```text
relay_core::...           -> crate::core::...
relay_application::...    -> crate::application::...
relay_infrastructure::... -> crate::infrastructure::...
relay_interfaces::...     -> crate::interfaces::...
```

**Separate-target rule:**
Inside `src/main.rs`, `src/commands/**`, `tests/**`, and `examples/**`, use `ai_tools::...` when referring to the package library.

**Steps:**
- [x] Treat binary/tests/examples separately from library-source rewrites.
- [x] Do not blindly replace all old paths with `crate::...` because each integration target has a different crate root.
- [x] Remove obsolete `use`/re-export plumbing created only for cross-crate consumption.
- [x] Preserve visibility boundaries wherever possible.

**Validation:**
- `rg 'relay_(core|application|infrastructure|interfaces)::' packages/rust-tools --glob '*.rs'` returns zero active-source hits.
- `cargo check --workspace --all-targets --all-features --locked` succeeds before old package shells are deleted.

## TASK-008 — Delete obsolete package shells

**Outcome:** No nested Cargo package remains.

**Remove after successful namespace migration:**
- `packages/rust-tools/core/Cargo.toml`
- `packages/rust-tools/application/Cargo.toml`
- `packages/rust-tools/infrastructure/Cargo.toml`
- `packages/rust-tools/interfaces/Cargo.toml`
- `packages/rust-tools/cli/Cargo.toml`
- Empty legacy package directories after their source/tests/examples have moved.

**Steps:**
- [x] Do not create compatibility crates.
- [x] Do not leave duplicate copies of moved sources.
- [x] Verify Git recognizes moves where practical rather than delete/recreate churn.

**Phase exit criteria:**
- [x] Exactly one production `src/` tree exists.
- [x] No old internal crate manifest exists.
- [x] Active production Rust compiles.

# PHASE-03 — Test and example consolidation

**Goal:** Make one package-local test tree and a maintainable example tree.
**Dependencies:** PHASE-02

## TASK-009 — Merge all integration tests into one `tests/`

**Outcome:** All current Rust integration tests live under `packages/rust-tools/tests/`.

**Expected current test files include:**
- `activity.rs`
- `continuation.rs`
- `protected_paths.rs`
- `redaction.rs`
- `relay_config.rs`
- `resources.rs`
- `security.rs`
- `ssh_catalog.rs`
- `ssh_diagnostics.rs`
- `ssh_policy.rs`
- `ssh_transport.rs`
- `task_notifications.rs`
- `task_progress.rs`
- `terminal_policy.rs`

**Steps:**
- [x] Move all current package-local integration tests into the single package test root.
- [x] Convert imports to `ai_tools::...`.
- [x] Do not move tests inline into production modules.
- [x] Do not introduce another Rust test root.

**Validation:**
- `cargo test --workspace --tests --all-features --locked`

## TASK-010 — Audit every `CARGO_MANIFEST_DIR` assumption

**Outcome:** Tests/config fixtures continue resolving the intended package/repository paths after manifest relocation.

**Known current users requiring explicit review:**
- `application/tests/ssh_diagnostics.rs`
- `core/tests/relay_config.rs`
- `infrastructure/tests/ssh_transport.rs`
- `infrastructure/tests/task_notifications.rs`
- `interfaces/tests/ssh_catalog.rs`

**Steps:**
- [x] Recalculate repository-root-relative paths from the new package root.
- [x] Ensure the MCP catalog snapshot test still resolves `.agents/contracts/063-tool-catalog-v13.json` (or its newer current successor when implementation starts).
- [x] Ensure workspace fixtures still point to intended safe test roots.
- [x] Do not preserve historical `../../..` assumptions blindly.

**Validation:**
- All affected tests pass from repository root and through `pnpm test:rust`.

## TASK-011 — Consolidate examples without violating folder budgets

**Outcome:** Existing examples remain buildable without creating a >15 direct-file maintainability violation.

**Recommended mapping:**

```text
application/examples/*.rs      -> examples/*.rs
application/examples/plan044a/ -> examples/plan044a/
core/examples/*                 -> examples/core/*
```

Current planning inventory indicates 15 direct application examples plus 2 core examples. Flattening all 17 would exceed the hard direct-file limit, so keep the two core examples nested.

**Steps:**
- [x] Preserve feature-gated example behavior.
- [x] Add explicit Cargo `[[example]]` entries for nested paths when required.
- [x] Do not add a broad maintainability exception merely to flatten examples.

**Validation:**
- `cargo check --workspace --all-targets --all-features --locked`

**Phase exit criteria:**
- [x] One Rust integration-test root exists.
- [x] All tests and examples remain buildable.
- [x] Manifest-root path assumptions are explicitly corrected.

# PHASE-04 — Architecture and repository guardrails

**Goal:** Replace lost compile-time crate isolation with deterministic repository enforcement.
**Dependencies:** PHASE-02

## TASK-012 — Add single-package topology checks to `check-architecture.sh`

**Outcome:** Repository guard rejects reintroduction of nested Rust packages.

**Files:**
- Modify: `scripts/check-architecture.sh`

**Steps:**
- [x] Assert `packages/rust-tools/Cargo.toml` is the only Cargo manifest beneath `packages/rust-tools/`.
- [x] Reject nested `core/Cargo.toml`, `application/Cargo.toml`, `infrastructure/Cargo.toml`, `interfaces/Cargo.toml`, `cli/Cargo.toml`, or future nested package manifests.
- [x] Reject active source references to removed `relay_*` namespaces.
- [x] Update the existing MCP transport-independence path from old `interfaces/src/...` to new `src/interfaces/...`.

## TASK-013 — Enforce Rust layer dependency direction

**Outcome:** Invalid single-crate cross-layer dependencies fail the architecture guard.

**Required negative rules:**
- [x] `src/core/**` cannot depend on `crate::interfaces`, `crate::application`, or `crate::infrastructure`.
- [x] `src/interfaces/**` cannot depend on `crate::application` or `crate::infrastructure`.
- [x] `src/application/**` cannot depend on `crate::infrastructure`.
- [x] `src/infrastructure/**` may depend inward on `core`, `interfaces`, and `application`.
- [x] `src/main.rs` and `src/commands/**` remain the composition edge and may depend on all layers.

Cover imports, direct fully-qualified references, and re-exports at minimum. Do not pretend a narrow `use` grep is sufficient if common bypass syntax remains untested.

## TASK-014 — Add deterministic Rust architecture fixtures

**Outcome:** The guard proves it can reject invalid dependencies instead of merely passing the current tree.

**Required negative fixtures:**
- `core -> infrastructure` fails.
- `core -> application` fails.
- `interfaces -> infrastructure` fails.
- `application -> infrastructure` fails.

**Required positive fixtures:**
- `interfaces -> core` passes.
- `application -> core` passes.
- `application -> interfaces` passes.
- `infrastructure -> application` passes.

Use the existing temporary-fixture pattern in `scripts/check-architecture.sh`; do not create plan-numbered verifier scripts.

**Validation:**
- `bash scripts/check-architecture.sh`
- Deliberate negative self-fixtures are rejected.

## TASK-015 — Update test-layout policy

**Outcome:** The repository recognizes only the consolidated Rust test root.

**Files:**
- Modify: `scripts/check-test-layout.mjs`

**Steps:**
- [x] Change the approved Rust test location to `packages/rust-tools/tests/`.
- [x] Reject old subcrate `*/tests/` locations.
- [x] Preserve the ban on inline Rust tests in production files.
- [x] Update error messages/docs from plural package-local directories to the actual single package test root.

**Validation:**
- `node scripts/check-test-layout.mjs rust`
- Existing consolidated tests pass the layout check.
- A deterministic fixture/guard self-case proves an old-style location would fail if the guard has a fixture mechanism; otherwise enforce through the direct path matcher and source review.

## TASK-016 — Update maintainability exact paths only

**Outcome:** Maintainability guard reflects file moves without broad new exceptions.

**Files:**
- Modify: `scripts/check-maintainability.mjs`

**Steps:**
- [x] Move the exact existing `hooks_acceptance.rs` exception path to `packages/rust-tools/examples/hooks_acceptance.rs` while preserving its concrete reason.
- [x] Do not add broad exceptions for `src`, `tests`, or `examples`.
- [x] Confirm `tests/` and `examples/` stay at or below the hard direct-file limit.
- [x] Keep thresholds unchanged.

**Validation:**
- `node scripts/check-maintainability.mjs`
- `node scripts/check-maintainability.mjs --self-test`

## TASK-017 — Review `guardrail.sh` without unnecessary churn

**Outcome:** Stack detection remains correct after path consolidation.

**Decision:** Current Rust path matching already covers `packages/rust-tools/*`, root `Cargo.toml`, and `Cargo.lock`. Do not modify `scripts/guardrail.sh` unless actual validation proves stack detection is broken.

**Phase exit criteria:**
- [x] Single-crate topology is repository-enforced.
- [x] Layer dependency direction is repository-enforced.
- [x] Architecture guard has deterministic negative fixtures.
- [x] Test and maintainability policies reflect the new tree without weakening thresholds.

# PHASE-05 — Build, release, and active tooling paths

**Goal:** Remove active references to deleted manifests while preserving binary/release behavior.
**Dependencies:** PHASE-01 through PHASE-04

## TASK-018 — Update native release manifest path

**Outcome:** Release artifact builder compiles the consolidated package.

**Files:**
- Modify: `ops/release/build-artifacts.sh`

**Steps:**
- [x] Replace `packages/rust-tools/cli/Cargo.toml` with `packages/rust-tools/Cargo.toml`.
- [x] Preserve target `x86_64-unknown-linux-gnu`.
- [x] Preserve output path/asset names/checksum behavior.
- [x] Preserve clean-main/tag/release fail-closed rules.
- [x] Do not publish a release during this plan.

**Validation:**
- `bash -n ops/release/build-artifacts.sh`
- Local release binary build command succeeds without publishing.

## TASK-019 — Update package skill command paths

**Outcome:** Active tool documentation no longer tells operators/agents to use the removed CLI manifest.

**Known files:**
- `packages/curl-tool/SKILL.md`
- `packages/relay-agent/SKILL.md`
- `packages/searxng-search-tool/SKILL.md`
- `packages/terminal-tool/SKILL.md`

**Steps:**
- [x] Replace old `packages/rust-tools/cli/Cargo.toml` examples with `packages/rust-tools/Cargo.toml`.
- [x] Do not change tool semantics or deployment boundaries.

## TASK-020 — Preserve root package commands unless necessary

**Outcome:** Existing `pnpm build:tools`, `pnpm lint:rust`, `pnpm typecheck:rust`, and `pnpm test:rust` continue working without cosmetic script churn.

**Steps:**
- [x] Re-run current scripts against the one-member workspace.
- [x] Modify `package.json` only if an existing command is objectively invalid after consolidation.
- [x] Do not make every ordinary guard compile unrelated extra targets unless required by current repository policy; use explicit `--all-targets` during Plan 064 acceptance.

**Phase exit criteria:**
- [x] No active build/release/tool skill depends on a removed manifest path.
- [x] Binary output naming/location remains compatible.

# PHASE-06 — Cargo lockfile and dependency parity

**Goal:** Reconcile package topology without dependency drift.
**Dependencies:** PHASE-01 through PHASE-05

## TASK-021 — Regenerate lockfile metadata without upgrades

**Outcome:** `Cargo.lock` reflects one package rather than five internal packages.

**Steps:**
- [x] Let Cargo reconcile package topology without `cargo update`.
- [x] Confirm old package nodes `relay-core`, `relay-application`, `relay-infrastructure`, `relay-interfaces`, and old `cli` disappear.
- [x] Confirm the single `ai-tools` package node appears.
- [x] Review third-party package version diffs; unexpected version changes are blockers, not acceptable churn.

## TASK-022 — Compare dependency feature parity

**Outcome:** The consolidated package retains the same effective runtime/security dependency capabilities.

**Review at minimum:**
- Tokio features.
- Reqwest TLS/JSON/charset/http2 features.
- Axum/Tower/Tower HTTP.
- rusqlite bundled mode.
- jsonschema.
- jsonwebtoken/ring/base64.
- tracing/OpenTelemetry stack.
- serde/serde_json.
- clap env/derive behavior.

**Validation:**
- Compare pre/post `cargo metadata` / `cargo tree` for unexpected external dependency additions/removals.

# PHASE-07 — Documentation and durable guidance

**Goal:** Make current human/agent documentation match the actual one-crate architecture without rewriting historical evidence.
**Dependencies:** PHASE-01 through PHASE-06

## TASK-023 — Update human documentation

**Inspect/update as needed:**
- `README.md`
- `packages/rust-tools/README.md`
- `docs/architecture.md`
- `docs/development.md`
- `docs/releases.md`

**Required reconciliations:**
- [x] Describe one Cargo package with layered modules, not independent internal crates.
- [x] Document `packages/rust-tools/tests/` as the Rust integration-test root.
- [x] Remove active commands that reference removed nested manifests.
- [x] Preserve Nuxt-vs-Rust deployment separation and all security/runtime invariants.
- [x] Correct stale test/verification wording encountered in files already being touched, but do not broaden into unrelated documentation cleanup.

## TASK-024 — Update current agent knowledge

**Inspect/update as needed:**
- `AGENTS.md`
- `.agents/knowledge/project.md`
- `.agents/knowledge/tooling.md`
- `.agents/knowledge/git.md`
- `.agents/knowledge/self-improvement.md`

**New durable invariant:**

```text
The native Rust implementation is one Cargo package. Layering remains explicit through
core/interfaces/application/infrastructure modules and is enforced by the repository
architecture guard rather than by separate Cargo package dependencies.
```

## TASK-025 — Reconcile canonical memory without rewriting history

**Outcome:** `.agents/memories/README.md` no longer requires independent Rust crates.

**Steps:**
- [x] Replace the current invariant requiring independent crates with the one-crate/module-based invariant.
- [x] State that Plan 032's layered-responsibility decision remains valid while its multi-crate enforcement mechanism is superseded by Plan 064.
- [x] Preserve historical Plan 032 itself as historical evidence.
- [x] Do not rewrite old completed plan files solely to update old file paths.

**Phase exit criteria:**
- [x] Current code, human docs, agent guidance, and canonical memory tell the same architecture story.

# PHASE-08 — Behavioral, contract, and security acceptance

**Goal:** Prove the refactor is structurally complete and behaviorally neutral.
**Dependencies:** all previous phases

## TASK-026 — Compiler/lint/test closure

**Validation:**

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --lib --bins --tests --all-features --locked
pnpm guardrail:rust
pnpm guardrail
```

**Expected condition:** all applicable commands pass with no warnings treated as errors.

## TASK-027 — Explicit architecture-guard closure

**Validation:**

```bash
bash scripts/check-architecture.sh
node scripts/check-test-layout.mjs rust
node scripts/check-maintainability.mjs
node scripts/check-maintainability.mjs --self-test
```

**Expected condition:**
- Current tree passes.
- New Rust negative architecture fixtures are rejected.
- One test root is accepted.
- No unexplained hard maintainability violation exists.

## TASK-028 — MCP catalog and contract parity

**Outcome:** Internal package movement does not alter client-visible MCP behavior.

**Steps:**
- [x] Relocate and run the current catalog snapshot test.
- [x] Confirm `tool_catalog()` exactly matches the current frozen catalog contract.
- [x] Preserve current profile membership, annotations, schemas, task support, security schemes, and tool count.
- [x] If current `main` has advanced beyond v13/102 tools when implementation starts, preserve the newer current contract rather than rolling source backward to this implementation baseline.

## TASK-029 — Security regression matrix

**Outcome:** Existing security owners remain behaviorally equivalent.

Ensure existing tests continue covering at least:
- protected paths;
- workspace containment/allowlisting;
- no-follow/atomic mutation behavior;
- terminal policy;
- SSH policy and SSH transport;
- relay configuration validation;
- OAuth/transport security;
- redaction;
- activity lifecycle/encryption paths;
- task/job lifecycle;
- task notification behavior;
- MCP catalog/schema validation.

No change is expected to:
- Bubblewrap profiles;
- safe PATH;
- process-group cleanup;
- OAuth admission/verification ordering;
- SSH credential mounts;
- Docker/Tailscale authority;
- Git credential isolation;
- output bounds/redaction;
- activity journaling;
- MCP task semantics.

Any such behavioral difference is a migration bug, not an accepted Plan 064 change.

## TASK-030 — Binary/CLI parity

**Validation:**

```bash
cargo build --release --locked --bin ai-tools
target/release/ai-tools --version
target/release/ai-tools --help
target/release/ai-tools terminal --help
target/release/ai-tools curl --help
target/release/ai-tools searxng --help
target/release/ai-tools relay --help
target/release/ai-tools telegram --help
```

**Expected condition:** binary name, version reporting, and command surface remain unchanged.

## TASK-031 — Dependency security audit

**Validation:**
- `pnpm audit:rust`

Do not opportunistically upgrade dependencies inside Plan 064 to address unrelated advisories without separately reviewing scope.

# PHASE-09 — Orphan and legacy-topology sweep

**Goal:** Remove all active traces of the five-package implementation mechanism.
**Dependencies:** PHASE-08

## TASK-032 — Prove one package remains

**Expected:**

```bash
find packages/rust-tools -name Cargo.toml
```

returns exactly:

```text
packages/rust-tools/Cargo.toml
```

The old top-level package directories must not remain as package shells:

```text
packages/rust-tools/core/
packages/rust-tools/application/
packages/rust-tools/infrastructure/
packages/rust-tools/interfaces/
packages/rust-tools/cli/
```

Their architectural equivalents exist only below `packages/rust-tools/src/`.

## TASK-033 — Sweep active legacy references

Search active source/tooling/docs for:
- `relay_core::`
- `relay_application::`
- `relay_infrastructure::`
- `relay_interfaces::`
- `packages/rust-tools/cli/Cargo.toml`
- `packages/rust-tools/core/Cargo.toml`
- `packages/rust-tools/application/Cargo.toml`
- `packages/rust-tools/infrastructure/Cargo.toml`
- `packages/rust-tools/interfaces/Cargo.toml`

Historical numbered plans/contracts may retain historically correct paths. Do not rewrite them merely for grep cleanliness.

## TASK-034 — Remove compatibility/orphan wrappers

Reject migration leftovers such as:
- compatibility modules existing only to alias old crate names;
- duplicate module files plus module directories for the same responsibility without a real facade reason;
- empty legacy directories;
- unused re-exports added only to make the move compile;
- stale dependency declarations;
- stale build/release commands.

**Phase exit criteria:**
- [x] No active legacy package plumbing remains.
- [x] No orphan compatibility layer remains.

# PHASE-10 — Final review and closure

## TASK-035 — Structural diff review

A healthy Plan 064 implementation diff should be dominated by:
- Git moves/renames;
- namespace/import changes;
- Cargo topology changes;
- architecture/test-layout/maintainability guard updates;
- release/tool path updates;
- current documentation updates.

Review specifically for unexpected:
- algorithm changes;
- visibility expansion;
- dependency changes;
- feature changes;
- public schema changes;
- security behavior changes;
- command behavior changes;
- deleted tests or examples.

## TASK-036 — Commit boundaries

Because architecture/test layout will be temporarily invalid during file movement, do not create broken checkpoint commits.

Recommended coherent implementation commit:

```text
refactor(rust): consolidate ai-tools into one crate
```

It should include the structural source/test/example/Cargo changes plus the guard/build-path changes necessary for the new topology to pass the mandatory guardrail.

Recommended documentation follow-up commit:

```text
docs(rust): document single-crate architecture
```

It should include current human docs, agent knowledge, canonical memory, and Plan 064 status/closure updates.

Do not bypass pre-commit hooks.

## TASK-037 — Final plan and self-improvement closeout

**Steps:**
- [x] Update this plan checklist/status only for work actually proven.
- [x] Review `.agents/knowledge/self-improvement.md` closeout requirements.
- [x] Reconcile canonical memory once, without duplicate memory files.
- [x] Explicitly record any remaining environmental/external limitation rather than marking it verified.
- [x] Do not restart/reload/install the relay or deploy unless separately authorized.

## Risks & Rollback

| Risk | Mitigation / rollback |
| --- | --- |
| Loss of compiler-enforced crate isolation | Add deterministic layer-direction guards and negative fixtures in the same migration; do not merge without them |
| Wrong namespace rewrite in integration targets | Separate `crate::*` rules for library source from `ai_tools::*` rules for binary/tests/examples |
| `CARGO_MANIFEST_DIR` silently changes fixture paths | Audit every known use individually and run affected tests |
| Example flattening exceeds maintainability budget | Keep core examples nested under `examples/core/` rather than adding a broad exception |
| Dependency feature drift | Preserve exact current versions/features; compare metadata/tree before and after |
| Lockfile pulls unrelated updates | Do not run `cargo update`; treat unexpected version churn as a blocker |
| MCP surface changes accidentally | Require exact current catalog snapshot parity |
| Release script points to removed manifest | Update and syntax-check/build using the consolidated package manifest |
| Docs keep teaching removed paths | Sweep active docs and package skills for old manifest references |
| Single crate becomes a giant unstructured module | Preserve current cohesive layer/submodule directories; only remove package boundaries |
| Visibility becomes overly public | Prefer `pub(crate)` and narrow exports; review every visibility expansion |
| Runtime/security behavior changes during mechanical refactor | Treat any behavior difference as a migration bug; revert/refine structural change rather than accepting semantic drift |
| Implementation work begins from an unrelated branch | Re-resolve current `main` and create a dedicated task branch/worktree before source mutation |

## Final Acceptance Criteria

- [x] Root Cargo workspace has one member: `packages/rust-tools`.
- [x] `packages/rust-tools/Cargo.toml` is the single native package/dependency manifest.
- [x] Package name is `ai-tools`.
- [x] `target/release/ai-tools` remains the release binary.
- [x] One production source root exists: `packages/rust-tools/src/`.
- [x] One Rust integration-test root exists: `packages/rust-tools/tests/`.
- [x] All current integration tests are preserved.
- [x] All current examples remain discoverable/compilable.
- [x] No old internal Cargo package remains.
- [x] No active `relay_core::`, `relay_application::`, `relay_infrastructure::`, or `relay_interfaces::` reference remains.
- [x] Layer dependency direction is enforced by `scripts/check-architecture.sh`.
- [x] Architecture guard proves representative invalid Rust dependencies fail.
- [x] Test-layout guard enforces the new single test root.
- [x] Maintainability thresholds are not weakened.
- [x] Existing exact-path maintainability exceptions are moved only as necessary.
- [x] Every `CARGO_MANIFEST_DIR` user affected by package movement is reconciled.
- [x] Current MCP catalog remains exactly contract-compatible.
- [x] No unrelated dependency version changes are introduced.
- [x] `Cargo.lock` contains no legacy internal packages.
- [x] Release build script uses the new manifest path.
- [x] Active tool skills no longer reference `cli/Cargo.toml`.
- [x] Current human docs describe one Rust package with layered modules.
- [x] Current agent guidance describes module-based layering and the new test root.
- [x] Canonical memory supersedes the old independent-crate requirement without rewriting historical Plan 032.
- [x] `cargo fmt --all -- --check` passes.
- [x] `cargo check --workspace --all-targets --all-features --locked` passes.
- [x] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passes.
- [x] Rust tests pass.
- [x] `pnpm audit:rust` passes.
- [x] `pnpm guardrail` passes.
- [x] Release binary builds and reports the expected version.
- [x] All CLI subcommands remain discoverable.
- [x] No orphan compatibility wrapper remains.
- [x] No relay service operation, deployment, or release publish occurred.

## Execution Handoff

Execution order is strictly PHASE-00 through PHASE-10 because Cargo topology, source moves, and test-layout/architecture guards are coupled. The mechanical namespace/file moves can be performed in batches inside the implementation phase, but validation should occur after each coherent layer migration to constrain regressions.

The most important review boundary is PHASE-04: do not accept the single-crate architecture merely because it compiles. The migration intentionally removes Cargo package boundaries, so the replacement Rust layer-direction guard and its negative fixtures are part of the architecture, not optional cleanup.

Deployment remains a separate operator action. Completing Plan 064 source/commit/PR work does not authorize systemd relay restart, binary replacement, tag creation, release publication, or production runtime mutation.
