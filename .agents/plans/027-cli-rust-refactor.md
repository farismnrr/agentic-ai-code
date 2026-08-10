# 027 — Refactor tool CLIs from JavaScript to Rust

## Status: PLANNED

Target branch: `dev`.

This plan replaces the current JavaScript/TypeScript CLI entrypoints with Rust
implementations incrementally, while preserving the existing tool contracts
and keeping the current implementations available as behavioral references
until parity is proven.

The repository is a pnpm workspace. The CLI entrypoints are currently owned by
individual tool packages rather than one monolithic CLI:

- `packages/terminal-tool/bin/cli.mjs`
- `packages/curl-tool/bin/cli.mjs`
- `packages/searxng-search-tool/bin/cli.mjs`

Their package implementations remain TypeScript and expose tool factories
through `src/index.ts`. The refactor therefore covers both the executable CLI
surface and the runtime implementation boundary where Rust actually needs to
replace JavaScript behavior; it is not just a rewrite of three argument
parsers.

## Goal

Move the executable tool implementations from JavaScript/TypeScript to Rust
with feature and behavior parity, while improving startup cost, type safety,
error handling, testability, and security boundaries.

The migration must be incremental and reversible. The existing JS
implementations remain the reference implementation until each Rust tool has
passed its parity and integration gates.

## Non-goals

- Do not redesign the user-facing CLI contract during the migration.
- Do not remove the existing JS implementations before Rust parity is proven.
- Do not change unrelated Nuxt/server architecture.
- Do not introduce Rust solely because it is faster; measure startup, memory,
and command latency before claiming an improvement.
- Do not weaken existing security controls in the name of API compatibility.
- Do not perform a repository-wide big-bang rewrite.

## Current implementation inventory

### `terminal-tool`

Current package: `packages/terminal-tool/`.

The package exposes `terminal-tool` through `bin/cli.mjs` and currently depends
on `execa`, `zod`, `@langchain/core`, and `ai`. The CLI accepts a command plus
options including `--cwd`, `--timeout`, and `--no-guard`. The CLI constructs
the tool and invokes it, while the implementation owns command execution and
its safety checks.

Rust must preserve the command-execution boundary, timeout behavior, working
directory behavior, output/error behavior, and the explicit guard/bypass
semantics. The Rust implementation must never silently turn a guarded command
execution path into unrestricted process execution.

### `curl-tool`

Current package: `packages/curl-tool/`.

The CLI accepts a URL plus request method, repeated headers, request body, and
`--no-guard`. Header parsing currently supports repeated `--header` values and
splits on the first `:`. The implementation has an SSRF/safe-URL boundary that
must remain explicit in Rust.

Rust must preserve method/header/body semantics, stdout/stderr behavior,
non-zero failure behavior, and the distinction between guarded and explicitly
bypassed requests.

### `searxng-search-tool`

Current package: `packages/searxng-search-tool/`.

The CLI accepts a positional query and an optional `--base-url`, defaulting to
`http://127.0.0.1:8888`. The implementation calls the SearXNG service through
the package tool abstraction.

Rust must preserve query handling, base URL configuration, HTTP failure
behavior, and output compatibility.

## Target architecture

Do not lock the exact crate/module layout until Step 1's inventory is complete.
The expected shape is a Rust workspace with clear separation between:

1. CLI parsing / command dispatch.
2. Domain/tool behavior.
3. External I/O (HTTP, subprocesses, filesystem where required).
4. Configuration and environment resolution.
5. Error types and process exit-code mapping.
6. Test fixtures and integration harnesses.

Use typed Rust models at package boundaries instead of passing unvalidated
JSON/string blobs through the entire implementation.

The final crate boundaries should follow actual coupling discovered during the
inventory. Prefer a small number of cohesive crates over one crate per file or
an unnecessarily elaborate workspace.

## Compatibility contract

For each CLI, establish the observable contract before implementation:

- command and subcommand names
- positional arguments
- flags and aliases
- defaults
- environment variables
- config files, if any
- accepted input shapes
- stdout content/format
- stderr content/format
- exit codes
- timeout semantics
- signal/interrupt behavior
- network behavior and URL validation
- subprocess behavior and working-directory semantics
- error cases

Where output is intentionally human-readable rather than a stable machine
contract, test the important semantic properties instead of freezing incidental
wording.

Where output is consumed programmatically, preserve the format exactly or
introduce an explicit versioned contract before changing it.

## Plan

### Step 1 — Complete the current-state inventory

Before writing Rust production code, inspect the full implementations rather
than only their `bin/cli.mjs` wrappers.

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
- workspace configuration and package scripts
- all imports/usages of these packages from the Nuxt/server code
- existing tests, fixtures, CI workflows, and documentation

Produce a table in this plan as implementation proceeds containing, for every
public CLI behavior: JS source, Rust target, compatibility requirement, and
verification method.

**Files touched:** this plan plus the audited files only as references; no
production code changes are required for this step.

**Verification:** inventory is complete enough that every public CLI behavior
has an identified owner and test strategy; no dependency or call site is left
unclassified.

### Step 2 — Establish Rust workspace and engineering baseline

Create the minimum Rust workspace structure justified by Step 1.

Choose dependencies based on actual requirements. Likely candidates include
`clap` for argument parsing, `tokio` for async I/O where required, `reqwest` for
HTTP, and a structured error approach such as `thiserror`; confirm versions and
features against the repository's MSRV/platform requirements before adding
anything.

Establish:

- `rustfmt` formatting
- Clippy with warnings treated as errors for the Rust workspace
- unit/integration test conventions
- documented MSRV/toolchain
- dependency policy and minimal feature flags
- release profile suitable for CLI binaries
- deterministic/reproducible build expectations where practical

Do not introduce unsafe Rust unless a concrete requirement is identified and
reviewed.

**Files touched:** new Rust workspace/crates, workspace metadata, CI/toolchain
configuration, and root/package integration files as required.

**Verification:** formatting, Clippy, unit tests, and a clean release build
pass on the supported development platform; dependency choices are justified
by actual code paths.

### Step 3 — Build the parity test harness before migration

Create a reusable harness that can execute the JS and Rust CLIs with the same
inputs and capture:

- exit status
- stdout
- stderr
- execution duration
- relevant side effects

Use deterministic fixtures and fake/local services wherever possible. Network
and subprocess tests must not depend on an uncontrolled public service.

For security-sensitive paths, add explicit negative tests rather than relying
only on happy-path snapshots.

Examples:

- terminal: valid command, missing command, invalid cwd, timeout, non-zero child
  exit, guarded execution, explicit bypass, signal behavior
- curl: GET/POST, repeated headers, body, malformed header, invalid URL,
  blocked unsafe URL, explicit bypass, HTTP error, timeout
- searxng: query, default/custom base URL, malformed URL, service unavailable,
  invalid response, timeout

**Files touched:** shared test harness, per-tool fixtures/integration tests,
local fake-service helpers as required.

**Verification:** at least one known-good JS invocation and its Rust-equivalent
produce equivalent observable behavior for each tool before any cutover.

### Step 4 — Define the Rust API boundary for each tool

For each package, separate CLI concerns from tool behavior so the binary is a
thin adapter.

The preferred boundary is conceptually:

`CLI args -> typed config/input -> tool/service -> typed result/error -> CLI output`

Map domain errors to stable process exit codes at the outermost CLI layer.
Do not scatter `process.exit`-equivalent behavior through business logic.

Preserve existing security decisions inside the tool/service boundary, not as
an accidental side effect of CLI parsing.

**Files touched:** Rust crate modules for each tool; parity tests may be
extended as needed.

**Verification:** core behavior can be tested without spawning the CLI binary,
and CLI integration tests verify only argument/output/exit-code wiring.

### Step 5 — Migrate `searxng-search-tool` first

Use SearXNG as the lowest-risk migration to validate the architecture:

- positional query parsing
- configurable base URL
- HTTP client lifecycle
- response decoding
- timeout/error mapping
- stdout contract

Keep the JS implementation intact as the oracle during this step.

**Files touched:** `packages/searxng-search-tool/**`, Rust workspace files,
parity tests, and package integration metadata.

**Verification:** Rust and JS produce equivalent results for fixture responses,
HTTP errors, invalid configuration, and service-unavailable cases. CI passes.

### Step 6 — Migrate `curl-tool` with SSRF/security parity as a gate

Treat the URL guard as a security boundary, not an implementation detail.

First model the existing safe-URL behavior and its tests. Then implement the
Rust equivalent with explicit tests for:

- loopback/private/link-local destinations as applicable to the current policy
- hostname resolution behavior
- redirects and whether validation is re-applied after redirects
- IPv4/IPv6 representations
- malformed URLs
- DNS/rebinding-sensitive behavior if the existing implementation addresses it
- explicit `--no-guard` behavior

Do not broaden bypass behavior accidentally. If the existing implementation's
security policy is ambiguous, document and resolve it before declaring parity.

Preserve method/header/body semantics and the existing CLI contract.

**Files touched:** `packages/curl-tool/**`, Rust workspace, security fixtures,
parity tests, and integration metadata.

**Verification:** all existing security tests pass; new negative tests prove
unsafe destinations remain blocked; positive tests prove normal requests work;
Rust and JS outputs/errors remain compatible.

### Step 7 — Migrate `terminal-tool` with process-execution hardening

Treat subprocess execution as a privileged boundary.

Inventory the current `execa` behavior before choosing the Rust process API.
Preserve:

- argument/command semantics
- working directory
- environment handling
- timeout behavior
- stdout/stderr capture
- child exit status
- signal propagation/termination
- guarded vs explicit bypass behavior

Avoid invoking a shell unless the current CLI contract explicitly requires shell
semantics. If shell parsing is currently part of the public behavior, document
that fact and reproduce it deliberately rather than accidentally.

Add tests for command injection edge cases and ensure user-controlled strings
are not silently reinterpreted as additional arguments.

**Files touched:** `packages/terminal-tool/**`, Rust workspace, process fixtures,
parity tests, and integration metadata.

**Verification:** process, timeout, failure, signal, guard, and argument-boundary
tests pass; no command is executed outside the intended working-directory and
argument contract.

### Step 8 — Integrate Rust binaries with the pnpm workspace

Define how the Nuxt application and package consumers invoke the Rust binaries
without coupling application code to a developer-specific filesystem path.

Evaluate the least disruptive option based on Step 1's actual import graph:

- package-level binary wrappers
- checked-in build scripts
- development-time Cargo invocation
- release binary artifacts
- platform-specific packaging

Do not leave a production path that requires Rust/Cargo to be installed unless
that is an explicit product requirement.

Preserve existing package names and public JS APIs for callers that are not
part of the CLI migration unless a separate breaking-change plan is approved.

**Files touched:** root/package manifests, package scripts, Rust build/release
configuration, server/tool integration points, and documentation.

**Verification:** fresh checkout can build/install the project following the
documented workflow; CLI binaries resolve correctly in development and in the
intended production/distribution environment.

### Step 9 — CI, release, and supply-chain hardening

Add CI coverage for the Rust implementation without dropping the existing JS
checks prematurely.

Required checks should include:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings` (adapt features if
  the final workspace does not use them)
- `cargo test --workspace`
- release builds for supported targets
- parity/integration tests
- existing `pnpm lint`
- existing `pnpm typecheck`
- existing relevant application tests

Pin/document the Rust toolchain and keep dependency features minimal. Add
`cargo deny`, `cargo audit`, or the repository's chosen dependency/security
scanner only if it fits the existing CI/security policy; avoid adding multiple
overlapping scanners without a reason.

For releases, define supported targets, binary naming, checksums/signatures if
artifacts are distributed, and rollback behavior.

**Files touched:** `.github/workflows/**`, Rust toolchain/configuration, release
scripts/manifests, and security/dependency configuration where justified.

**Verification:** CI passes from a clean checkout and every supported release
target produces the expected binary artifact.

### Step 10 — Performance and reliability benchmark

Benchmark the old and new implementations before claiming that Rust improves
performance.

Measure at minimum:

- cold startup time
- warm startup time where meaningful
- peak RSS / memory footprint
- command latency
- binary size
- repeated invocation throughput for representative commands

Use identical environments and representative fixtures. Record results in the
plan or an appropriate benchmark document. Investigate regressions rather than
optimizing prematurely.

**Files touched:** benchmark harness/configuration and documentation/results.

**Verification:** reproducible benchmark command exists and produces comparable
measurements for JS and Rust implementations.

### Step 11 — Cut over one tool at a time

For each tool, require all of the following before removing its JS CLI path:

1. Feature parity is demonstrated.
2. Security-sensitive behavior has explicit regression coverage.
3. Integration tests pass.
4. CI builds the Rust binary for supported targets.
5. Performance is at least acceptable against the baseline.
6. Documentation is updated.
7. A rollback path exists for the first release after cutover.

Switch the default invocation to Rust, but keep the JS implementation available
for the agreed deprecation window.

**Files touched:** per-tool package manifests/scripts, application integration,
documentation, and release configuration.

**Verification:** normal developer and production workflows use Rust; rollback
can restore the JS implementation without rewriting history.

### Step 12 — Remove obsolete JavaScript implementations

Only after every tool has completed its cutover window:

- remove unused JS CLI entrypoints
- remove obsolete CLI-only dependencies (`execa`, etc.) only when no other
  code imports them
- remove dead package scripts/build glue
- remove obsolete compatibility tests
- update package READMEs and developer documentation
- update `.agents/plans/README.md` to move this plan from In Flight to Completed
  and record the shipped commit/PR

Do not remove shared TypeScript tool APIs if they are still consumed by the
application; distinguish CLI migration from API migration.

**Verification:** dependency graph contains no dead CLI dependencies, all CI
checks remain green, fresh installation works, and no remaining imports point
to removed JS CLI paths.

## Files touched summary

The exact final file list is intentionally derived during Step 1, but is
expected to include:

- `.agents/plans/027-cli-rust-refactor.md`
- Rust workspace/crate files (new)
- `packages/terminal-tool/**`
- `packages/curl-tool/**`
- `packages/searxng-search-tool/**`
- root/workspace package metadata as required
- `.github/workflows/**` as required
- integration/parity test fixtures
- documentation/release configuration as required

No unrelated application files should be modified merely to accommodate the
rewrite.

## Definition of Done

- All three CLI tools have Rust implementations with documented ownership and
  module boundaries.
- CLI arguments, defaults, output, errors, exit codes, and side effects have a
  documented compatibility contract.
- Rust behavior is verified against deterministic JS reference cases.
- Terminal process-execution safety and curl SSRF protections have explicit
  regression tests.
- Rust formatting, Clippy, unit/integration tests, and release builds run in CI.
- Supported development and production workflows no longer require the JS CLI
  implementation after its deprecation window.
- Rust binaries are packaged/distributed in a documented, reproducible way.
- Benchmark results are recorded rather than making unverified performance
  claims.
- Obsolete JS CLI code and dependencies are removed only after dependency and
  integration verification.
- `.agents/plans/README.md` records this plan as completed with its final PR or
  commit reference.

## Rollback strategy

During migration, each tool keeps its JS implementation until its Rust
replacement passes the cutover gate. If a Rust regression is discovered after
cutover, switch the package/application invocation back to the JS implementation
for that tool, fix the Rust implementation, and repeat the parity gate.

Do not rewrite `dev` history to hide failed migration attempts; preserve the
migration trail in commits/PRs.
