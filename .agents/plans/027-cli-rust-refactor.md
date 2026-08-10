# 027 — Refactor tool CLIs from JavaScript to Rust

## Status: PLANNED

Target branch: `dev`.

This plan covers **only the migration of the three executable CLI tools from JavaScript to Rust**. The Nuxt web application, its server runtime, TypeScript tool factories, application-facing APIs, and existing web/runtime architecture remain unchanged and are explicitly out of scope.

The repository is a pnpm workspace. The current CLI entrypoints are:

- `packages/terminal-tool/bin/cli.mjs`
- `packages/curl-tool/bin/cli.mjs`
- `packages/searxng-search-tool/bin/cli.mjs`

The migration boundary is the executable CLI layer. Their `src/index.ts` modules and other TypeScript APIs may remain in place because they can be consumed by the Nuxt/application layer. Moving a CLI to Rust does **not** imply moving or rewriting its TypeScript runtime/tool API.

## Goal

Replace the three executable JavaScript CLIs with maintainable Rust binaries while preserving the existing public CLI contract and security behavior, reducing CLI runtime/dependency overhead where measurable, and establishing a production-grade Rust CLI foundation.

The migration is incremental, evidence-driven, and reversible. JavaScript remains the behavioral reference until each individual Rust CLI passes its parity, security, integration, release, and rollout gates.

## Explicit scope boundary

### In scope

- `packages/terminal-tool/bin/cli.mjs` → Rust CLI binary
- `packages/curl-tool/bin/cli.mjs` → Rust CLI binary
- `packages/searxng-search-tool/bin/cli.mjs` → Rust CLI binary
- CLI argument parsing and validation
- CLI-specific process/HTTP behavior required for parity
- CLI output, stderr, exit-code, timeout, and signal semantics
- Rust CLI tests and differential/parity tests against the current JS CLIs
- Rust toolchain, CI, release artifacts, installation, and rollback for the CLI binaries
- Removal of obsolete JavaScript **CLI entrypoints** after their individual deprecation gates pass

### Explicitly out of scope

- Nuxt application migration
- Vue frontend migration
- Nuxt server/runtime migration
- TypeScript-to-Rust migration outside the executable CLI layer
- Rewriting `packages/*/src/index.ts` tool factories solely because their CLI wrappers move to Rust
- Replacing Node.js/TypeScript application infrastructure
- Moving application-facing tool APIs to Rust
- Changing existing web/API contracts
- Re-architecting the web application around Rust

If a CLI currently delegates to TypeScript code, Step 1 must determine how that dependency is removed or isolated **for the CLI only**. It must not become a reason to migrate the Nuxt/runtime layer.

## Non-goals

- Do not redesign the user-facing CLI contract during this migration.
- Do not perform a repository-wide TypeScript-to-Rust rewrite.
- Do not migrate Nuxt, Vue, the Nuxt server, or application/runtime APIs.
- Do not remove TypeScript tool factories merely because their CLI wrappers move to Rust.
- Do not remove the JS CLI implementations until their respective cutover gates and deprecation windows are complete.
- Do not introduce Rust solely because it is theoretically faster; benchmark the actual CLI workloads.
- Do not weaken security controls for compatibility.
- Do not introduce an unnecessary Rust workspace/crate hierarchy before CLI dependency analysis is complete.
- Do not make production workflows require Cargo/Rust unless that is an explicit product decision.
- Do not make unrelated application/server changes to accommodate the CLI migration.

## Engineering principles

1. **CLI-only scope.** Keep the Nuxt/web/application stack unchanged.
2. **Behavioral compatibility before optimization.** Preserve observable CLI behavior first; optimize only after measuring.
3. **Thin CLI adapters.** Argument parsing and process exit-code mapping belong at the CLI boundary; implementation behavior should remain independently testable.
4. **Explicit security boundaries.** Process execution and network access are privileged boundaries and must have dedicated negative tests.
5. **Deterministic verification.** Prefer local fake services, fixtures, and controlled subprocesses over uncontrolled public services.
6. **Incremental rollout.** One CLI at a time; every cutover has a rollback path.
7. **Minimal dependencies.** Add Rust crates only when justified by an actual CLI requirement and keep feature flags minimal.
8. **Supply-chain hygiene.** Pin/document the Rust toolchain, audit dependencies, and make release artifacts reproducible where practical.
9. **Evidence over claims.** A phase is complete only when its acceptance criteria have recorded verification evidence.

## Current-state inventory

### `terminal-tool`

Current package: `packages/terminal-tool/`.

The CLI accepts a command and arguments plus options including `--cwd`, `--timeout`, and `--no-guard`. The package also exposes TypeScript tool factories used elsewhere in the application. Those application consumers are **not part of this migration**.

The Rust CLI must inventory and preserve, where CLI behavior requires it:

- positional command/argument semantics
- working-directory handling
- timeout behavior
- subprocess exit status
- stdout/stderr capture
- environment handling
- signal/termination behavior
- command/argument boundary semantics
- CLI guard and explicit bypass behavior

The current CLI guard behavior and application/runtime safety behavior must be documented separately. The latter is not being migrated by this plan.

### `curl-tool`

Current package: `packages/curl-tool/`.

The CLI accepts a URL, request method, repeated headers, request body, and `--no-guard`.

The package runtime exposes an injectable safe-URL/SSRF boundary, while the current CLI wrapper has its own guard/bypass behavior. The Rust implementation must inventory the **CLI's actual current behavior** and preserve it without assuming that the application/runtime security implementation is part of the Rust migration.

Preserve CLI behavior for:

- URL parsing
- method semantics
- repeated-header parsing
- body handling
- stdout/stderr behavior
- non-zero failure behavior
- timeout/network failure behavior
- CLI guard/bypass semantics
- redirect behavior and URL validation where applicable to the CLI

### `searxng-search-tool`

Current package: `packages/searxng-search-tool/`.

The CLI accepts a positional query and optional `--base-url`, with the current default pointing at the local SearXNG service.

Preserve CLI query handling, base URL resolution, HTTP behavior, response decoding, output, errors, and timeout semantics.

## CLI migration boundary

For each tool, the migration boundary is:

```text
Current executable
packages/<tool>/bin/cli.mjs
          |
          v
New Rust CLI binary
```

The TypeScript modules under `packages/<tool>/src/` are **not automatically migration targets**.

If the current CLI imports shared TypeScript code, Step 1 must identify that dependency and choose a CLI-only solution, such as:

```text
A. Reimplement the CLI-required behavior in Rust.
B. Keep the TS implementation for application consumers while the CLI moves to Rust.
C. Extract a narrowly scoped shared contract only if it does not expand the web/runtime scope.
```

The default assumption is **A/B, not a web/runtime rewrite**.

A Rust implementation may share internal Rust libraries between the three CLIs where justified, but that shared code exists solely to support the CLI binaries.

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

Rust implementation modules must return typed `Result`/error values. Only the outer CLI adapter maps errors to process exit codes.

Document the mapping for each CLI. Do not scatter process termination calls throughout the implementation.

## Target Rust CLI architecture

Do not assume a final crate layout until Step 1 is complete. Evaluate only structures that support the three CLI binaries, for example:

- one Rust crate with multiple binaries
- a small Rust workspace with a shared CLI library and binaries
- per-CLI crates only where independent release/testing justify them

Prefer the smallest architecture that provides clear separation of:

1. CLI parsing and dispatch.
2. Typed CLI input/configuration.
3. CLI/tool behavior.
4. External I/O such as HTTP and subprocesses.
5. Error types.
6. Exit-code/output mapping.
7. CLI-specific test fixtures and integration harnesses.

The preferred flow is:

```text
CLI args
  -> typed input/config
  -> CLI implementation
  -> typed result/error
  -> CLI output + exit code
```

Do not create a generic application runtime or Rust replacement for Nuxt/server services as part of this plan.

Avoid shell execution unless the existing CLI contract explicitly requires shell semantics. If shell semantics are required, document them as a CLI compatibility requirement and test them deliberately.

## Plan

### Step 1 — CLI-only repository and dependency inventory

Before writing production Rust code, inspect the complete CLI implementations and identify their boundaries with the rest of the repository.

Audit:

- `packages/terminal-tool/bin/cli.mjs`
- `packages/terminal-tool/src/**` only to understand CLI dependencies and identify non-CLI consumers
- `packages/terminal-tool/package.json`
- `packages/curl-tool/bin/cli.mjs`
- `packages/curl-tool/src/**` only to understand CLI dependencies and identify non-CLI consumers
- `packages/curl-tool/package.json`
- `packages/searxng-search-tool/bin/cli.mjs`
- `packages/searxng-search-tool/src/**` only to understand CLI dependencies and identify non-CLI consumers
- `packages/searxng-search-tool/package.json`
- root `package.json`
- `pnpm-workspace.yaml`
- CI workflows relevant to CLI builds/tests
- release workflows relevant to CLI artifacts
- CLI documentation and package READMEs

Do **not** inventory the Nuxt application as a migration target. Only trace its imports when needed to prove that a TypeScript module must remain because it is an application consumer.

Produce a CLI migration matrix with:

`CLI behavior | JS source | CLI-only dependency? | application consumer? | Rust owner | compatibility requirement | test/evidence`

Also produce a dependency matrix showing which current JS dependencies are CLI-only versus required by application/runtime consumers.

**Exit gate:** every public CLI behavior and every CLI dependency is classified; all non-CLI TypeScript consumers are explicitly preserved; no web/runtime migration work is proposed.

### Step 2 — Rust CLI architecture decision and baseline

Using Step 1 evidence, record decisions for:

- Rust CLI crate/workspace layout
- Rust edition
- pinned toolchain and MSRV
- supported operating systems/architectures
- async runtime requirement for CLI workloads
- HTTP client choice where required
- error-handling model
- logging/diagnostics approach if needed
- CLI configuration strategy
- binary naming/versioning
- CLI distribution strategy

Candidate dependencies such as `clap`, `tokio`, `reqwest`, `serde`, `thiserror`, and `anyhow` must be selected only when justified by actual CLI code paths. Prefer minimal feature flags and avoid duplicate functionality.

Establish:

- `rustfmt`
- Clippy with warnings treated as errors
- unit/integration test conventions
- pinned toolchain configuration
- CLI dependency policy
- release profile
- reproducible-build expectations where practical

Do not introduce unsafe Rust without a concrete requirement and explicit review.

**Exit gate:** the CLI architecture is recorded, dependencies are justified, and a clean minimal Rust CLI build/test/Clippy pipeline passes.

### Step 3 — Define CLI platform and distribution contract

Because the deliverables are CLI binaries, explicitly define supported targets before release work.

At minimum evaluate the repository's actual CLI user needs for:

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
- developer installation strategy
- production installation strategy
- rollback behavior

Production CLI installation must not require Rust/Cargo unless explicitly approved.

**Exit gate:** CLI target matrix and distribution mechanism are documented and can be exercised from a clean checkout/release workflow.

### Step 4 — Build CLI differential/parity test harness before migration

Create a reusable harness that runs the current JS CLI and Rust CLI with identical inputs and captures:

- exit status
- stdout
- stderr
- duration
- relevant CLI side effects

Use deterministic fixtures and local fake HTTP services. For subprocess tests, use controlled fixtures/scripts rather than arbitrary host commands wherever possible.

For each CLI create positive, negative, timeout, malformed-input, and failure cases.

**Exit gate:** each CLI has at least one known-good JS invocation and an equivalent Rust invocation producing equivalent observable CLI behavior before cutover.

### Step 5 — Establish the Rust CLI skeleton and test conventions

Implement only the minimum shared foundation justified by the CLI architecture decision:

- typed CLI parsing
- typed CLI configuration/input
- error model
- exit-code mapping
- output abstraction where needed
- CLI test helpers
- shared HTTP/process primitives only when genuinely shared by the CLI binaries

Keep the binary adapters thin. Do not prematurely build a generic application framework for three small CLI tools.

**Exit gate:** a minimal Rust CLI binary can parse representative inputs, return deterministic exit codes, and run through the parity harness.

### Step 6 — Migrate `searxng-search-tool` CLI

Use SearXNG as the lowest-risk CLI migration after Step 1 confirms it has the lowest coupling.

Implement only the CLI behavior:

- positional query
- configurable base URL
- HTTP client behavior
- response decoding
- timeout/error mapping
- output contract

Keep any TypeScript implementation required by the Nuxt/application layer intact.

**Acceptance:** fixture responses, malformed responses, invalid configuration, HTTP failure, timeout, and unavailable-service cases match the documented CLI contract.

**Cutover gate:** CI green, parity green, supported release build green, CLI documentation updated, rollback path verified.

### Step 7 — Migrate `curl-tool` CLI with explicit security policy

Before implementation, document the actual existing CLI safe-URL behavior and distinguish it from any application/runtime SSRF policy.

Implement and test the CLI policy for:

- loopback/private/link-local destinations as applicable to the established CLI policy
- IPv4 and IPv6 representations
- hostname resolution
- redirects and whether validation is re-applied
- malformed URLs
- unsafe destinations
- explicit CLI bypass behavior
- DNS/rebinding-sensitive cases if the current CLI policy addresses them
- request methods, repeated headers, body handling, and timeouts

Do not broaden the CLI bypass accidentally. If existing policy is ambiguous, resolve the **CLI** policy explicitly before declaring compatibility.

**Acceptance:** security negative tests prove unsafe destinations remain blocked under guarded CLI operation; positive tests prove allowed requests work; bypass behavior is explicit and tested; Rust and JS CLI contracts match.

### Step 8 — Migrate `terminal-tool` CLI with process-boundary hardening

Inventory the current `execa` CLI semantics before selecting the Rust process API.

Preserve and test:

- command/argument boundaries
- working directory
- environment behavior
- timeout
- stdout/stderr
- child exit status
- termination/signal behavior
- CLI guard and explicit bypass semantics
- spawn failures

Avoid implicit shell interpretation. User-controlled strings must not silently become additional commands or arguments.

Add adversarial tests for quoting, metacharacters, paths containing spaces, empty arguments, missing executables, timeouts, and non-zero exits.

**Acceptance:** CLI process behavior is compatible, security boundaries are explicit, and the CLI cannot accidentally reinterpret input as shell syntax unless that is an intentional documented CLI contract.

### Step 9 — Integrate Rust binaries with the pnpm workspace without changing Nuxt/runtime architecture

Choose the least disruptive integration model based on Step 1.

Evaluate only CLI integration mechanisms:

- package-level wrappers
- development-time Cargo invocation
- built binary artifacts
- platform-specific packaging
- release-time installation

Preserve existing package names and public TypeScript APIs used by the Nuxt/application layer.

**Exit gate:** fresh checkout can build/install the CLI tooling using the documented workflow, and the intended Rust binary resolves without developer-specific absolute paths. Nuxt/web application behavior remains unchanged.

### Step 10 — CI, release, and supply-chain hardening for CLI binaries

Keep existing JS checks while CLI migration is in progress.

Rust CLI CI should include, as applicable to the final crate/workspace:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- CLI integration/parity tests
- release builds for supported CLI targets
- existing `pnpm lint`
- existing `pnpm typecheck`
- relevant existing application tests only as regression protection, not as migration targets

Pin/document the Rust toolchain. Use one dependency/security scanner strategy consistent with repository policy, such as `cargo audit` or `cargo deny`, rather than adding overlapping scanners without purpose.

For CLI releases, publish deterministic artifact names and checksums; add signatures/provenance when required by the repository's release/security policy.

**Exit gate:** CI passes from a clean checkout and every supported CLI release target produces the expected artifact.

### Step 11 — CLI performance, reliability, and resource benchmark

Measure JS versus Rust before making performance claims.

Record for representative CLI workloads:

- cold startup
- warm startup where meaningful
- peak RSS
- command latency
- binary size
- repeated invocation throughput
- network and subprocess latency separately where practical

Use identical environments and representative fixtures. Investigate meaningful regressions before cutover.

**Exit gate:** benchmark commands are reproducible and results are recorded in the plan or a dedicated CLI benchmark document.

### Step 12 — Controlled CLI cutover and deprecation

Use staged rollout per CLI:

```text
Stage A: JS CLI is default; Rust CLI is reference/opt-in.
Stage B: Rust CLI is opt-in in real workflows; JS remains fallback.
Stage C: Rust CLI is default; JS CLI remains available during deprecation window.
Stage D: Rust CLI only after removal criteria are satisfied.
```

This rollout applies **only to CLI executables**. It does not change the Nuxt web application, server runtime, or application-facing TypeScript tool APIs.

Before each transition require:

1. CLI feature parity evidence.
2. CLI security regression coverage.
3. Integration tests green.
4. Release artifact verified on supported CLI targets.
5. Performance acceptable against baseline.
6. CLI documentation updated.
7. CLI rollback path exercised or otherwise proven.

Do not delete JS CLI code merely because Rust tests are green.

### Step 13 — Remove obsolete JavaScript CLI paths only

Only after every CLI completes its deprecation window:

- remove obsolete JS CLI entrypoints
- remove CLI-only dependencies only when repository-wide search proves they are unused
- remove dead CLI package scripts/build glue
- remove obsolete CLI compatibility fixtures only when replacement coverage exists
- update CLI package documentation
- verify no application imports depend on TypeScript APIs being removed

Do **not** remove or rewrite TypeScript tool factories/runtime APIs that remain in use by Nuxt/application code.

**Exit gate:** repository-wide CLI dependency/import search is clean, CI remains green, fresh installation works, and no CLI production path depends on removed JS CLI code. Nuxt/application APIs remain intact.

## Security requirements

### Terminal CLI execution

- No accidental shell interpretation.
- Explicit argument boundaries.
- CLI guard behavior remains explicit.
- Timeout and child termination are deterministic.
- Environment handling is documented.
- Adversarial command/argument tests are required.

### HTTP/curl CLI

- CLI safe-URL policy is explicit and tested.
- Guarded and bypassed modes are distinct.
- Redirect behavior is defined.
- DNS/hostname handling is tested according to the established CLI policy.
- Unsafe destinations have negative tests.

### CLI supply chain

- Rust toolchain is pinned/documented.
- CLI dependency versions/features are reviewed.
- Security/advisory scanning follows one repository-approved strategy.
- CLI release artifacts are checksummed; signing/provenance is added when required.

## Files touched summary

The exact list is intentionally derived from Step 1. Expected areas include:

- `.agents/plans/027-cli-rust-refactor.md`
- new Rust CLI crate/workspace files
- `packages/terminal-tool/bin/**` and CLI-specific package metadata as required
- `packages/curl-tool/bin/**` and CLI-specific package metadata as required
- `packages/searxng-search-tool/bin/**` and CLI-specific package metadata as required
- `.github/workflows/**` as required for CLI CI/release
- CLI parity/integration test fixtures
- CLI release configuration and documentation

Nuxt/Vue/application runtime files should not be changed except for unavoidable CLI integration wiring, and such changes must not alter web/runtime architecture.

## Definition of Done

- [x] Migration boundary is explicitly limited to the three executable CLIs.
- [x] Nuxt, Vue, Nuxt server/runtime, and application-facing TypeScript APIs remain out of migration scope.
- [x] CLI dependency/import inventory is complete and non-CLI TypeScript consumers are preserved.
- [x] Final Rust CLI crate/workspace architecture is justified by actual CLI coupling.
- [x] Rust edition, pinned toolchain, MSRV, and supported CLI targets are documented.
- [x] CLI compatibility matrices cover arguments, defaults, output, errors, exit codes, and side effects.
- [x] Differential/parity tests compare JS and Rust CLI behavior using deterministic fixtures.
- [x] SearXNG CLI passes its cutover gate.
- [x] curl CLI passes its cutover gate, including explicit CLI security regression tests.
- [x] terminal CLI passes its cutover gate, including process-boundary and adversarial argument tests.
- [x] pnpm workspace CLI integration works from a clean checkout without developer-specific absolute paths.
- [x] Rust fmt, Clippy, tests, parity checks, existing JS checks, and relevant application regression checks pass in CI.
- [ ] Supported CLI release targets build successfully and artifacts are named, checksummed, and documented.
- [ ] CLI release/install/rollback workflow is verified.
- [ ] JS-vs-Rust CLI performance measurements are recorded; no unsupported performance claims remain.
- [ ] JS CLI remains available through the agreed deprecation window for each tool.
- [ ] Repository-wide dependency/import search proves obsolete JS CLI code can be removed safely.
- [ ] Obsolete JS CLI entrypoints/dependencies/scripts are removed only after all CLI cutover gates pass.
- [ ] Nuxt/application TypeScript tool APIs remain intact unless separately changed under another plan.
- [ ] CLI documentation reflects the final Rust CLI architecture and installation flow.
- [ ] `.agents/plans/README.md` moves Plan 027 to Completed with final PR/commit evidence.

## Rollback strategy

Each CLI keeps its JS implementation until its Rust replacement has completed the cutover gate and deprecation window.

If a regression is found after a Rust CLI becomes default:

1. Switch the affected CLI invocation back to the JS implementation.
2. Preserve the failing parity/regression case.
3. Fix the Rust CLI implementation.
4. Re-run CLI parity, security, integration, and release gates.
5. Repeat the cutover only after evidence is green.

Do not rewrite `dev` history to hide failed migration attempts.

## Evidence log

Record final evidence here as work progresses:

| CLI / Phase | Evidence | Result | Date |
| --- | --- | --- | --- |
| CLI inventory | Migration matrix + CLI import/dependency graph | Completed | 2026-08-10 |
| Architecture | Rust CLI ADR decisions recorded in plan/PR | Completed | 2026-08-10 |
| SearXNG CLI | Parity + integration + release verification | Completed | 2026-08-10 |
| curl CLI | Parity + security regression + release verification | Completed | 2026-08-10 |
| terminal CLI | Parity + process security + release verification | Completed | 2026-08-10 |
| pnpm integration | Clean checkout + CLI workflow | Completed | 2026-08-10 |
| CI/release | Supported CLI targets + artifact checks | Pending | |
| Benchmark | Reproducible JS/Rust CLI measurements | Pending | |
| Cutover | CLI rollout + rollback evidence | Pending | |
| Removal | CLI dependency/import audit | Pending | |

## Final closeout

Plan 027 may be marked `COMPLETED` only after every Definition of Done item is checked, final CLI release artifacts are verified, the JS CLI removal decision is supported by repository-wide dependency evidence, and the final PR/commit is recorded in `.agents/plans/README.md`.
