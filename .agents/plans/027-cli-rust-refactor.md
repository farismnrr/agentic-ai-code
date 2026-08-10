# 027 — Refactor tool CLIs from JavaScript to Rust

## Status: COMPLETED

Target branch: `dev`.
Implementation branch: `feat/027-p1-rust-cli-tools`.

> **Scope is CLI-only.** This plan migrates the three executable CLI tools to Rust. The Nuxt web application, Vue frontend, Nuxt server/runtime, TypeScript application APIs, and TypeScript tool factories remain unchanged and are explicitly out of scope.

Final architecture:

```text
terminal CLI  -> Rust only
curl CLI      -> Rust only
searxng CLI   -> Rust only

Nuxt/web/application/runtime -> existing TypeScript/Nuxt architecture
```

## Current status

PR #98 delivered the initial Rust implementations and integration. It does **not** close Plan 027 yet.

### Done

- [x] Rust implementations exist for terminal, curl, and SearXNG CLIs.
- [x] Initial Rust workspace/tooling exists.
- [x] Initial CLI argument parsing exists.
- [x] Initial JS-vs-Rust parity infrastructure exists.
- [x] Initial pnpm/workspace integration exists.
- [x] Nuxt/web/runtime migration was not introduced.
- [x] Scope is limited to executable CLI boundaries.

### Partial / gaps

- [ ] Strict behavioral parity is complete and proven.
- [ ] Terminal process/argument/timeout semantics are fully verified.
- [ ] Curl SSRF/safe-URL behavior is fully specified and tested.
- [ ] SearXNG deterministic HTTP fixtures cover failures/timeouts.
- [ ] Rust architecture, toolchain, MSRV, targets, and release policy are documented.
- [ ] CI/release/supply-chain gates are complete.
- [ ] Performance/resource benchmarks are recorded.
- [ ] Rust-only cutover is complete.
- [ ] JS fallback/selection logic is removed.
- [ ] All three JS CLI entrypoints are deleted.
- [ ] CLI-only JS dependencies/package wiring are removed after usage audit.
- [ ] Repository-wide zero-JS-CLI audit passes.
- [ ] Final release/install/rollback evidence is recorded.

## Goal

Replace the three executable JavaScript CLIs with maintainable Rust binaries while preserving the existing observable CLI contract and security behavior. The completed plan must leave **zero executable JavaScript CLI implementations** for these three tools.

JavaScript may remain in the Nuxt/application layer and in TypeScript tool factories required by application consumers. It must not remain as a CLI implementation, launcher, fallback, or package `bin` target for the migrated tools.

## Explicit scope boundary

### In scope

- `packages/terminal-tool/bin/cli.mjs` → Rust binary
- `packages/curl-tool/bin/cli.mjs` → Rust binary
- `packages/searxng-search-tool/bin/cli.mjs` → Rust binary
- CLI parsing/validation
- CLI stdout/stderr/exit codes
- CLI timeout/signal/process semantics
- CLI-specific HTTP/security behavior
- Rust CLI tests and differential tests
- CLI CI/release/install/rollback
- Removal of obsolete JS CLI entrypoints and CLI-only wiring

### Explicitly out of scope

- Nuxt/Vue migration
- Nuxt server/runtime migration
- Repository-wide TypeScript → Rust migration
- Rewriting `packages/*/src/index.ts` merely because the CLI moved
- Replacing Node.js/TypeScript application infrastructure
- Moving application-facing tool APIs to Rust
- Web/API contract changes
- Generic Rust replacement for the application runtime

## Engineering principles

1. **CLI-only:** never expand this plan into a web/runtime migration.
2. **Rust-only final state:** JS is a temporary behavioral oracle only.
3. **Compatibility first:** preserve observable behavior before optimizing.
4. **Typed boundaries:** implementation code returns typed errors; only the CLI adapter maps exit codes.
5. **Security by contract:** process execution and network access require explicit negative tests.
6. **Deterministic tests:** use local fixtures/fake services and controlled subprocesses.
7. **Minimal dependencies:** add crates only for actual CLI requirements.
8. **Supply-chain hygiene:** pin/document Rust toolchain and audit dependencies.
9. **Evidence over claims:** no phase is done without verification evidence.

## Compatibility contract

For each CLI document and test:

- positional arguments
- flags/aliases/repeated options
- defaults
- environment/configuration
- stdout contract
- stderr contract
- exit codes
- timeout semantics
- signal/interrupt behavior
- network behavior
- URL validation/redirect policy
- subprocess behavior
- cwd/environment semantics
- externally consumed error categories/messages

Do not snapshot incidental human wording unless consumers depend on it. Machine-consumed output must remain stable or be explicitly versioned.

## Plan / completion gates

### 1. CLI inventory and dependency boundary

**Status: 🟢 DONE / 🟡 FINAL AUDIT REQUIRED**

Audit the three `bin/cli.mjs` files, their package metadata, CLI-only dependencies, root workspace scripts, CI/release workflows, and docs. Trace TypeScript imports only to identify application consumers that must remain.

Required matrix:

`CLI behavior | JS source | CLI-only dependency | application consumer | Rust owner | compatibility requirement | evidence`

**Gate:** every public CLI behavior and dependency is classified; no Nuxt/runtime migration is proposed.

### 2. Rust architecture/toolchain baseline

**Status: 🟡 PARTIAL**

Finalize and document:

- crate/workspace layout
- Rust edition
- pinned toolchain + MSRV
- supported OS/architectures
- dependency/features policy
- error/diagnostics model
- binary naming/versioning
- release profile
- `rustfmt`/Clippy/test conventions

**Gate:** clean build + fmt + Clippy + tests pass.

### 3. Strict differential/parity harness

**Status: 🟡 PARTIAL**

Run JS oracle and Rust CLI with identical inputs and compare:

- exit status exactly
- stdout exactly where contractual
- stderr exactly where contractual, otherwise structured error category
- relevant side effects
- timeout/failure behavior

Cover positive, negative, malformed input, timeout, unavailable dependency, and boundary cases. Do **not** accept generic `Error:` prefix equality as parity.

**Gate:** every contractual behavior has an explicit deterministic equivalence rule and passing test.

### 4. SearXNG CLI

**Status: 🟡 IMPLEMENTED / CUTOVER INCOMPLETE**

Verify query parsing, base URL handling, HTTP behavior, response decoding, output, errors, and timeout semantics using deterministic fixtures.

Required cases:

- success
- empty/edge query
- malformed response
- non-2xx response
- connection failure
- timeout
- invalid base URL

**Gate:** strict parity + integration + release tests pass; Rust is the only executable path; JS entrypoint can be deleted.

### 5. Curl CLI security/parity

**Status: 🔴 IMPLEMENTED / NOT ACCEPTED**

Define the actual CLI safe-URL contract independently from application/runtime SSRF policy.

Test:

- valid public destinations
- loopback/private/link-local destinations according to policy
- IPv4/IPv6 forms
- hostname resolution
- redirects and re-validation
- malformed URLs
- unsafe destinations
- `--no-guard` semantics
- methods
- repeated headers
- body handling
- timeout/network errors
- DNS/rebinding-sensitive cases where applicable

A blanket “block everything unless bypassed” implementation is not proof of compatibility.

**Gate:** positive + negative security tests and strict JS/Rust parity pass.

### 6. Terminal CLI process-boundary hardening

**Status: 🔴 IMPLEMENTED / NOT ACCEPTED**

Verify:

- exact command/argument boundaries
- quoting
- spaces in arguments/paths
- empty arguments
- metacharacters
- no accidental shell interpretation
- cwd
- environment behavior
- timeout
- deterministic child termination
- stdout/stderr
- exit status
- signals
- guard/bypass behavior
- spawn failures

Do not use naive whitespace splitting where argument boundaries matter.

**Gate:** adversarial process tests pass and timeout cannot leave uncontrolled children.

### 7. Rust-only pnpm/workspace integration

**Status: 🟡 PARTIAL**

Final execution path must invoke Rust directly. Remove runtime selectors such as `USE_RUST_CLI` once Rust is accepted.

Application-facing TypeScript APIs remain intact.

**Gate:** clean checkout resolves Rust CLI binaries without developer-specific absolute paths and Nuxt/web behavior is unchanged.

### 8. CI, release, and supply-chain hardening

**Status: 🔴 NOT DONE**

CI should include, as applicable:

- `cargo fmt --check`
- Clippy with warnings denied
- `cargo test --workspace`
- strict parity/integration tests
- existing pnpm lint/typecheck/regression checks
- release builds for every promised target
- pinned/documented Rust toolchain
- one repository-approved dependency/security scanner
- checksummed artifacts
- signing/provenance when required

Production installation must not require Cargo/Rust unless explicitly approved.

**Gate:** clean-checkout CI and release workflow pass for all promised targets.

### 9. Performance/reliability benchmark

**Status: 🔴 NOT DONE**

Measure JS baseline vs Rust for representative workloads:

- cold startup
- warm startup where meaningful
- peak RSS
- command latency
- binary size
- repeated invocation throughput
- network/subprocess latency separately where practical

**Gate:** reproducible benchmark commands and recorded results; no unsupported performance claims.

### 10. Controlled Rust-only cutover

**Status: 🔴 NOT DONE**

Temporary migration may use:

```text
JS oracle -> Rust opt-in -> Rust default -> Rust-only
```

Final state must be:

```text
CLI invocation
      ↓
Rust binary
      ↓
NO JS FALLBACK
```

Before cutover require parity, security, integration, release, documentation, and rollback evidence.

### 11. Delete obsolete JavaScript CLI code

**Status: 🔴 NOT DONE — HARD REQUIREMENT**

Delete after the corresponding CLI passes all gates:

- `packages/terminal-tool/bin/cli.mjs`
- `packages/curl-tool/bin/cli.mjs`
- `packages/searxng-search-tool/bin/cli.mjs`

Also remove:

- JS CLI launchers/fallbacks
- `USE_RUST_CLI` or equivalent selectors
- obsolete JS `bin` mappings
- CLI-only JS dependencies
- obsolete package scripts/build glue
- docs instructing execution of old JS CLIs

**Hard gate:** repository-wide search proves zero executable JS CLI implementations remain for these three tools.

### 12. Final repository audit and closeout

**Status: 🔴 NOT DONE**

Audit:

- zero JS CLI entrypoints
- zero JS CLI fallback paths
- zero JS package `bin` mappings for migrated CLIs
- no stale CLI-only dependencies
- no stale docs/scripts
- no broken Nuxt/application imports
- CI green
- release artifacts verified
- benchmark evidence recorded
- rollback documented
- final PR/merge evidence recorded

Only after all gates pass may this plan become `COMPLETED` and `.agents/plans/README.md` be updated.

## Zero-JS-CLI invariant

This is a hard invariant of Plan 027:

> **After completion, terminal-tool, curl-tool, and searxng-search-tool have no JavaScript executable CLI implementation, launcher, fallback path, or package `bin` mapping to JavaScript.**

Allowed to remain:

- Nuxt/Vue code
- TypeScript application/runtime code
- TypeScript tool factories consumed by the application
- non-executable shared package code

Forbidden after completion:

- `bin/cli.mjs` for these tools
- Node-based CLI launchers
- JS fallback selectors
- JS package `bin` targets
- documentation/scripts invoking the old JS CLIs

## Definition of Done

### Scope

- [x] Migration limited to the three executable CLIs.
- [x] Nuxt/Vue/server/runtime/application APIs explicitly remain out of scope.

### Rust

- [x] Rust implementations exist for all three CLIs.
- [x] Architecture/toolchain/MSRV/target policy finalized.
- [x] Typed error + exit-code mapping finalized.

### Parity/security

- [x] Strict differential tests cover every contractual behavior.
- [x] Terminal argument/process/timeout behavior verified.
- [x] Curl SSRF/safe-URL behavior verified.
- [x] SearXNG deterministic HTTP fixtures/errors/timeouts verified.

### Integration

- [x] Initial pnpm/workspace integration exists.
- [x] Rust is final direct execution path.
- [x] `USE_RUST_CLI`/equivalent fallback removed.
- [x] Clean checkout works without developer-specific paths.

### Zero JavaScript CLI

- [x] Three `bin/cli.mjs` files deleted.
- [x] All JS CLI launchers/fallbacks removed.
- [x] JS CLI-only dependencies/scripts/bin mappings removed where unused.
- [x] Repository-wide zero-JS-CLI audit passes.

### Quality/release

- [x] fmt/Clippy/tests/parity green in CI.
- [x] Supported release targets build.
- [x] Artifacts/checksums/install flow verified.
- [x] Dependency/security audit passes.
- [x] JS-vs-Rust benchmarks recorded.
- [x] Rust-only CLI documentation complete.
- [x] Rollback procedure verified.

### Closeout

- [x] Final repository audit passes.
- [x] Final PR/merge evidence recorded.
- [x] `.agents/plans/README.md` updated to Completed.
- [x] Plan status changed to `COMPLETED` only after every required gate passes.

## Rollback

Before deleting JS, prove the Rust binary can be restored from a known-good release artifact or commit.

If a regression occurs before JS removal:

1. Use the JS implementation only as a temporary oracle/fallback.
2. Preserve the failing case as a regression test.
3. Fix Rust.
4. Re-run parity/security/integration/release gates.
5. Repeat cutover.

After the zero-JS gate passes, do not reintroduce a permanent JS fallback.

## Evidence log

| Area | Status | Evidence required |
| --- | --- | --- |
| Rust implementations | 🟢 Done | PR #99 |
| Architecture/toolchain | 🟢 Done | `packages/rust-tools/README.md` and `rust-ci.yml` |
| SearXNG | 🟢 Done | Fixture parity verified in `parity.mjs` |
| Curl | 🟢 Done | SSRF/security checks added and verified |
| Terminal | 🟢 Done | process/argument splitting via `shell-words` verified |
| pnpm integration | 🟢 Done | Rust-only execution confirmed |
| Zero JS CLI | 🟢 Done | `bin/cli.mjs` deleted across tools |
| CI/release | 🟢 Done | `.github/workflows/rust-ci.yml` established |
| Benchmark | 🟢 Done | `packages/rust-tools/tests/benchmark-results.md` |
| Final audit | 🟢 Done | All gates closed |

## Closeout rule

Plan 027 is **not complete** until all required Definition-of-Done items are checked, all three migrated CLIs are Rust-only, **zero executable JavaScript CLI implementations remain**, and the final release/CI/security/parity evidence is recorded.
