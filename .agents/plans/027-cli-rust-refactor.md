# 027 — Refactor tool CLIs from JavaScript to Rust

## Status: PLANNED

Target branch: `dev`.

This plan migrates the executable tool CLIs from JavaScript/TypeScript to Rust incrementally, while preserving observable behavior, security invariants, package integration, and a reversible rollout path.

The repository is a pnpm workspace. The current CLI entrypoints are:

- `packages/terminal-tool/bin/cli.mjs`
- `packages/curl-tool/bin/cli.mjs`
- `packages/searxng-search-tool/bin/cli.mjs`

The executable wrappers are not the whole migration boundary. Their `src/index.ts` implementations expose tool factories and may also be consumed by application/server code. The migration must therefore explicitly distinguish **CLI migration** from **shared tool-runtime/API migration** and must not remove TypeScript APIs that still have consumers.

## Goal

Replace the three executable JavaScript CLIs with maintainable Rust binaries while preserving the existing public CLI contract and security behavior, reducing runtime/dependency overhead where measurable, and establishing a production-grade Rust CLI foundation.

The migration is incremental, evidence-driven, and reversible. JavaScript remains the behavioral reference until each individual Rust implementation passes its parity, security, integration, release, and rollout gates.

## Non-goals

- Do not redesign the user-facing CLI contract during this migration.
- Do not perform a repository-wide TypeScript-to-Rust rewrite.
- Do not remove shared TypeScript tool APIs merely because their CLI wrappers move to Rust.
- Do not remove the JS implementations until their respective cutover gates and deprecation windows are complete.
- Do not introduce Rust solely because it is theoretically faster; benchmark the actual workloads.
- Do not weaken security controls for compatibility.
- Do not introduce an unnecessary Rust workspace/crate hierarchy before dependency and coupling analysis is complete.
- Do not make production workflows require Cargo/Rust unless that is an explicit product decision.
- Do not make unrelated Nuxt/server changes to accommodate the migration.

## Engineering principles

1. **Behavioral compatibility before optimization.** Preserve observable behavior first; optimize only after measuring.
2. **Thin CLI adapters.** Argument parsing and process exit-code mapping belong at the CLI boundary; tool behavior should be independently testable.
3. **Explicit security boundaries.** Process execution and network access are privileged boundaries and must have dedicated negative tests.
4. **Deterministic verification.** Prefer local fake services, fixtures, and controlled subprocesses over uncontrolled public services.
5. **Incremental rollout.** One tool at a time; every cutover has a rollback path.
6. **Minimal dependencies.** Add crates only when justified by an actual requirement and keep feature flags minimal.
7. **Supply-chain hygiene.** Pin/document the toolchain, audit dependencies, and make release artifacts reproducible where practical.
8. **Evidence over claims.** A phase is complete only when its acceptance criteria have recorded verification evidence.

## Current-state inventory

### `terminal-tool`

Current package: `packages/terminal-tool/`.

The CLI accepts a command and arguments plus options including `--cwd`, `--timeout`, and `--no-guard`. The package implementation uses `execa` and exposes tool factories used by the application/tooling layer.

The migration must inventory and preserve, where public behavior requires it:

- positional command/argument semantics
- working-directory handling
- timeout behavior
- subprocess exit status
- stdout/stderr capture
- environment handling
- signal/termination behavior
- command/argument boundary semantics
- guard and explicit bypass behavior

The current CLI guard behavior and the runtime safety behavior must be documented separately. Do not assume that a CLI guard is equivalent to the runtime security policy.

### `curl-tool`

Current package: `packages/curl-tool/`.

The CLI accepts a URL, request method, repeated headers, request body, and `--no-guard`.

The important distinction is that the package runtime exposes an injectable safe-URL/SSRF boundary, while the current CLI wrapper has its own guard/bypass behavior. The Rust migration must inventory the actual call paths before implementing a security policy. Do not invent a stronger or weaker policy and call it parity.

Preserve:

- URL parsing behavior
- method semantics
- repeated-header parsing
- body handling
- stdout/stderr behavior
- non-zero failure behavior
- timeout/network failure behavior
- guard/bypass semantics
- redirect behavior and URL validation policy where applicable

### `searxng-search-tool`

Current package: `packages/searxng-search-tool/`.

The CLI accepts a positional query and optional `--base-url`, with the current default pointing at the local SearXNG service.

Preserve query handling, base URL resolution, HTTP behavior, response decoding, output, errors, and timeout semantics.

## Migration boundary decision

Before production implementation, Step 1 must answer this explicitly for each package:

```text
CLI-only migration:
JS CLI -> Rust CLI -> existing TS runtime (if still required)

or

Runtime migration:
TS runtime -> Rust runtime -> Rust CLI
```

A hybrid architecture is allowed when it is the least disruptive option:

```text
Rust core/tool implementation
        ├── Rust CLI adapter
        └── compatibility adapter for remaining TS consumers, if required
```

The final decision must be recorded in the plan before the corresponding tool is migrated. No shared TypeScript API may be removed until its repository-wide import graph proves it is unused.

## Compatibility specification

For each CLI, create a compatibility matrix covering:

- command/subcommand names
- positional arguments
- flags, aliases, and repeatable options
- defaults
- environment variables
- configuration files, if any
- accepted input shapes
- stdout format
- stderr format
- exit codes
- timeout semantics
- signal/interrupt behavior
- network behavior
- URL validation and redirect policy
- subprocess behavior
- working-directory semantics
- environment inheritance/filtering
- error categories/messages where externally consumed

Human-facing incidental wording should not be snapshot-locked unless consumers depend on it. Machine-consumed output must remain stable or be explicitly versioned before changing.

### Exit-code policy

Rust business/tool modules must return typed `Result`/error values. Only the outer CLI adapter maps errors to process exit codes.

Document the mapping for each command. Do not scatter process termination calls throughout the implementation.

## Target architecture

Do not assume a final crate layout until Step 1 is complete. Evaluate whether the codebase needs:

- one cohesive Rust crate with multiple binaries
- a small Rust workspace with shared library code and binaries
- per-tool crates only where independent ownership/release/testing justify them

Prefer the smallest architecture that provides clear separation of:

1. CLI parsing and dispatch.
2. Typed input/configuration.
3. Tool/domain behavior.
4. External I/O such as HTTP and subprocesses.
5. Error types.
6. Exit-code/output mapping.
7. Test fixtures and integration harnesses.

The preferred flow is:

```text
CLI args
  -> typed input/config
  -> tool/service
  -> typed result/error
  -> CLI output + exit code
```

Avoid shell execution unless the existing public contract explicitly requires shell semantics. If shell semantics are required, document them as a compatibility requirement and test them deliberately.

## Plan

### Step 1 — Repository and dependency inventory

Before writing production Rust code, inspect the complete implementations and every consumer.

Audit:

- `packages/terminal-tool/bin/cli.mjs`
- `packages/terminal-tool/src/**`
- `packages/terminal-tool/package.json`
- `packages/curl-tool/bin/cli.mjs`
- `packages/curl-tool/src/**`
- `packages/curl-tool/package.json`
- `packages/searxng-search-tool/bin/cli.mjs`
- `packages/searxng-search-tool/src/**`
- `packages/searxng-search-tool/package.json`
- root `package.json`
- `pnpm-workspace.yaml`
- all repository imports/usages of the three packages
- existing tests and fixtures
- CI workflows
- release workflows
- documentation and package READMEs

Produce a migration matrix with columns:

`CLI behavior | JS source | shared-runtime consumer? | Rust owner | compatibility requirement | test/evidence`

Also produce a dependency matrix showing which current JS dependencies are CLI-only versus shared by application/runtime consumers.

**Exit gate:** every public CLI behavior and every package consumer is classified; no dependency or import path remains unexplained.

### Step 2 — Architecture decision record and Rust baseline

Using Step 1 evidence, record decisions for:

- migration boundary per tool
- final crate/workspace layout
- Rust edition
- pinned toolchain and MSRV
- supported operating systems/architectures
- async runtime requirement
- HTTP client choice
- error-handling model
- logging/diagnostics approach if needed
- configuration strategy
- binary naming/versioning
- distribution strategy

Candidate dependencies such as `clap`, `tokio`, `reqwest`, `serde`, `thiserror`, and `anyhow` must be selected only when justified by actual code paths. Prefer minimal feature flags and avoid duplicate functionality.

Establish:

- `rustfmt`
- Clippy with warnings treated as errors
- unit/integration test conventions
- pinned toolchain configuration
- dependency policy
- release profile
- reproducible-build expectations where practical

Do not introduce unsafe Rust without a concrete requirement and explicit review.

**Exit gate:** architecture decisions are recorded, dependencies are justified, and a clean minimal Rust build/test/Clippy pipeline passes.

### Step 3 — Define supported platform and distribution contract

Because the deliverable is a binary CLI, explicitly define supported targets before release work.

At minimum evaluate the repository's actual user/deployment needs for:

- Linux x86_64
- Linux ARM64
- macOS x86_64
- macOS ARM64
- Windows x86_64

Do not promise a target merely because Cargo can theoretically compile it.

Define:

- binary names
- release version source of truth
- artifact naming
- checksums/signatures if distributed
- GitHub Release strategy
- development installation strategy
- production installation strategy
- rollback behavior

Production installation must not require Rust/Cargo unless explicitly approved.

**Exit gate:** target matrix and distribution mechanism are documented and can be exercised from a clean checkout/release workflow.

### Step 4 — Build differential/parity test harness before migration

Create a reusable harness that runs JS and Rust with identical inputs and captures:

- exit status
- stdout
- stderr
- duration
- relevant side effects

Use deterministic fixtures and local fake HTTP services. For subprocess tests, use controlled fixtures/scripts rather than arbitrary host commands wherever possible.

For each tool create positive, negative, timeout, malformed-input, and failure cases.

**Exit gate:** each CLI has at least one known-good JS invocation and an equivalent Rust invocation producing equivalent observable behavior before cutover.

### Step 5 — Establish the Rust CLI skeleton and shared testing conventions

Implement only the minimum shared foundation justified by the ADR:

- typed CLI parsing
- typed configuration/input
- error model
- exit-code mapping
- output abstraction where needed
- test helpers
- shared HTTP/process primitives only when genuinely shared

Keep the binary adapter thin. Do not prematurely build a generic framework for three small tools.

**Exit gate:** a minimal Rust binary can parse representative inputs, return deterministic exit codes, and run through the parity harness.

### Step 6 — Migrate `searxng-search-tool`

Use SearXNG as the lowest-risk production migration after Step 1 confirms it has the lowest coupling.

Implement:

- positional query
- configurable base URL
- HTTP client behavior
- response decoding
- timeout/error mapping
- output contract

Keep the JS implementation intact as the oracle.

**Acceptance:** fixture responses, malformed responses, invalid configuration, HTTP failure, timeout, and unavailable-service cases match the documented contract.

**Cutover gate:** CI green, parity green, supported release build green, documentation updated, rollback path verified.

### Step 7 — Migrate `curl-tool` with explicit SSRF/security policy

Before implementation, document the actual existing safe-URL policy and distinguish it from the CLI's current guard/bypass behavior.

Implement and test the policy for:

- loopback/private/link-local destinations as applicable to the established policy
- IPv4 and IPv6 representations
- hostname resolution
- redirects and whether validation is re-applied
- malformed URLs
- unsafe destinations
- explicit bypass behavior
- DNS/rebinding-sensitive cases if the current policy addresses them
- request methods, repeated headers, body handling, and timeouts

Do not broaden the bypass accidentally. If existing policy is ambiguous, resolve the policy explicitly before declaring compatibility.

**Acceptance:** security negative tests prove unsafe destinations remain blocked under guarded operation; positive tests prove allowed requests work; bypass behavior is explicit and tested; Rust and JS contracts match.

### Step 8 — Migrate `terminal-tool` with process-boundary hardening

Inventory the current `execa` semantics before selecting the Rust process API.

Preserve and test:

- command/argument boundaries
- working directory
- environment behavior
- timeout
- stdout/stderr
- child exit status
- termination/signal behavior
- guard and explicit bypass semantics
- spawn failures

Avoid implicit shell interpretation. User-controlled strings must not silently become additional commands or arguments.

Add adversarial tests for quoting, metacharacters, paths containing spaces, empty arguments, missing executables, timeouts, and non-zero exits.

**Acceptance:** process behavior is compatible, security boundaries are explicit, and the CLI cannot accidentally reinterpret input as shell syntax unless that is an intentional documented contract.

### Step 9 — Integrate Rust binaries with the pnpm workspace

Use the Step 1 import graph to choose the least disruptive integration model.

Evaluate:

- package-level wrappers
- development-time Cargo invocation
- built binary artifacts
- platform-specific packaging
- release-time installation

Preserve existing package names and public TypeScript APIs for non-CLI consumers unless a separate breaking-change plan is approved.

**Exit gate:** fresh checkout can build/install the project using the documented workflow, and the application resolves the intended binary without developer-specific absolute paths.

### Step 10 — CI, release, and supply-chain hardening

Keep existing JS checks while migration is in progress.

Rust CI should include, as applicable to the final workspace:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- integration/parity tests
- release builds for supported targets
- existing `pnpm lint`
- existing `pnpm typecheck`
- relevant existing application tests

Pin/document the Rust toolchain. Use one dependency/security scanner strategy consistent with repository policy, such as `cargo audit` or `cargo deny`, rather than adding overlapping scanners without purpose.

For releases, publish deterministic artifact names and checksums; add signatures/provenance when required by the repository's release/security policy.

**Exit gate:** CI passes from a clean checkout and every supported release target produces the expected artifact.

### Step 11 — Performance, reliability, and resource benchmark

Measure JS versus Rust before making performance claims.

Record:

- cold startup
- warm startup where meaningful
- peak RSS
- command latency
- binary size
- repeated invocation throughput
- network and subprocess latency separately where practical

Use identical environments and representative fixtures. Investigate meaningful regressions before cutover.

**Exit gate:** benchmark commands are reproducible and results are recorded in the plan or a dedicated benchmark document.

### Step 12 — Controlled cutover and deprecation

Use staged rollout per tool:

```text
Stage A: JS is default; Rust is reference/opt-in.
Stage B: Rust is opt-in in real workflows; JS remains fallback.
Stage C: Rust is default; JS remains available during deprecation window.
Stage D: Rust-only after removal criteria are satisfied.
```

Before each transition require:

1. Feature parity evidence.
2. Security regression coverage.
3. Integration tests green.
4. Release artifact verified on supported targets.
5. Performance acceptable against baseline.
6. Documentation updated.
7. Rollback path exercised or otherwise proven.

Do not delete JS implementation code merely because Rust tests are green.

### Step 13 — Remove obsolete JS CLI paths

Only after every tool completes its deprecation window:

- remove obsolete JS CLI entrypoints
- remove CLI-only dependencies only when repository-wide search proves they are unused
- remove dead package scripts/build glue
- remove obsolete compatibility fixtures only when replacement coverage exists
- update package documentation
- verify no application imports depend on deleted TypeScript APIs

Do not remove shared TypeScript runtime/tool APIs if they remain in use outside the CLI.

**Exit gate:** repository-wide import/dependency search is clean, CI remains green, fresh installation works, and no production path depends on removed JS CLI code.

## Security requirements

### Terminal execution

- No accidental shell interpretation.
- Explicit argument boundaries.
- Guard behavior remains explicit.
- Timeout and child termination are deterministic.
- Environment handling is documented.
- Adversarial command/argument tests are required.

### HTTP/curl

- Safe-URL policy is explicit and tested.
- Guarded and bypassed modes are distinct.
- Redirect behavior is defined.
- DNS/hostname handling is tested according to the established policy.
- Unsafe destinations have negative tests.

### Supply chain

- Rust toolchain is pinned/documented.
- Dependency versions/features are reviewed.
- Security/advisory scanning follows one repository-approved strategy.
- Release artifacts are checksummed; signing/provenance is added when required.

## Files touched summary

The exact list is intentionally derived from Step 1. Expected areas include:

- `.agents/plans/027-cli-rust-refactor.md`
- new Rust workspace/crate files
- `packages/terminal-tool/**`
- `packages/curl-tool/**`
- `packages/searxng-search-tool/**`
- root/workspace package metadata as required
- `.github/workflows/**` as required
- parity/integration test fixtures
- release configuration and documentation

No unrelated application files should be changed merely to accommodate the rewrite.

## Definition of Done

- [ ] Migration boundary is documented for all three tools.
- [ ] Final Rust crate/workspace architecture is justified by actual coupling.
- [ ] Rust edition, pinned toolchain, MSRV, and supported targets are documented.
- [ ] CLI compatibility matrices cover arguments, defaults, output, errors, exit codes, and side effects.
- [ ] Differential/parity tests compare JS and Rust behavior using deterministic fixtures.
- [ ] SearXNG passes its cutover gate.
- [ ] curl passes its cutover gate, including explicit SSRF/security regression tests.
- [ ] terminal passes its cutover gate, including process-boundary and adversarial argument tests.
- [ ] pnpm/application integration works from a clean checkout without developer-specific absolute paths.
- [ ] Rust fmt, Clippy, tests, parity checks, existing JS checks, and relevant application checks pass in CI.
- [ ] Supported release targets build successfully and artifacts are named, checksummed, and documented.
- [ ] Release/install/rollback workflow is verified.
- [ ] JS-vs-Rust performance measurements are recorded; no unsupported performance claims remain.
- [ ] JS remains available through the agreed deprecation window for each tool.
- [ ] Repository-wide dependency/import search proves obsolete JS CLI code can be removed safely.
- [ ] Obsolete JS CLI entrypoints/dependencies/scripts are removed only after all cutover gates pass.
- [ ] Documentation reflects the final architecture and installation flow.
- [ ] `.agents/plans/README.md` moves Plan 027 to Completed with final PR/commit evidence.

## Rollback strategy

Each tool keeps its JS implementation until its Rust replacement has completed the cutover gate and deprecation window.

If a regression is found after Rust becomes default:

1. Switch the affected invocation back to the JS implementation.
2. Preserve the failing parity/regression case.
3. Fix the Rust implementation.
4. Re-run parity, security, integration, and release gates.
5. Repeat the cutover only after evidence is green.

Do not rewrite `dev` history to hide failed migration attempts.

## Evidence log

Record final evidence here as work progresses:

| Tool / Phase | Evidence | Result | Date |
| --- | --- | --- | --- |
| Inventory | Migration matrix + import/dependency graph | Pending | |
| Architecture | ADR decisions recorded in plan/PR | Pending | |
| SearXNG | Parity + integration + release verification | Pending | |
| curl | Parity + security regression + release verification | Pending | |
| terminal | Parity + process security + release verification | Pending | |
| Integration | Clean checkout + pnpm workflow | Pending | |
| CI/release | Supported targets + artifact checks | Pending | |
| Benchmark | Reproducible JS/Rust measurements | Pending | |
| Cutover | Rollout + rollback evidence | Pending | |
| Removal | Dependency/import audit | Pending | |

## Final closeout

Plan 027 may be marked `COMPLETED` only after every Definition of Done item is checked, final release artifacts are verified, the JS removal decision is supported by repository-wide dependency evidence, and the final PR/commit is recorded in `.agents/plans/README.md`.
