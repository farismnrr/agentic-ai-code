# 027 — Refactor tool CLIs from JavaScript to Rust

## Status: IN PROGRESS

**Target branch:** `feat/027-p1-rust-cli-tools` (PR #99 → `dev`)

> **Current reality:** PR #99 contains the main Rust CLI migration and removes the three JavaScript CLI entrypoints, but Plan 027 is **not complete yet**. The implementation satisfies the core Rust-only direction, while several production completion gates remain open.

### Scope invariant

This plan covers **only the executable CLI layer** for:

- `terminal-tool`
- `curl-tool`
- `searxng-search-tool`

The Nuxt web application, Vue frontend, Nuxt server/runtime, TypeScript tool factories, application-facing APIs, and existing web/runtime architecture remain unchanged and are explicitly out of scope.

Final architecture:

```text
terminal CLI       -> Rust only
curl CLI           -> Rust only
searxng CLI        -> Rust only

Nuxt/web/runtime   -> existing TypeScript/Nuxt architecture
application APIs   -> unchanged
```

## Implementation status

### DONE / verified in PR #99

- [x] Rust CLI implementations exist for all three tools.
- [x] `packages/terminal-tool/bin/cli.mjs` removed.
- [x] `packages/curl-tool/bin/cli.mjs` removed.
- [x] `packages/searxng-search-tool/bin/cli.mjs` removed.
- [x] JavaScript `package.json` `bin` mappings for these CLIs removed.
- [x] Core Rust argument parsing exists.
- [x] Terminal uses explicit process argument boundaries rather than shell-string interpolation.
- [x] Basic terminal, curl, and SearXNG integration tests exist.
- [x] Basic curl loopback/private-address blocking exists.
- [x] Basic Rust fmt/Clippy/test/build CI exists.
- [x] Nuxt/web runtime was not migrated by PR #99.
- [x] The final architecture remains CLI-only Rust; TypeScript/Nuxt remains the application/runtime stack.

### PARTIAL / needs hardening

- [ ] Terminal timeout contract is preserved: PR #99 hardcodes 30s and does not expose the documented `--timeout` option.
- [ ] Terminal timeout must prove deterministic child termination and no uncontrolled descendants.
- [ ] Terminal adversarial argument/process tests are incomplete.
- [ ] Curl SSRF policy needs comprehensive IPv4/IPv6/hostname/redirect/DNS edge-case coverage and an explicit CLI policy.
- [ ] SearXNG lacks the planned deterministic mock HTTP fixture suite for success/error/malformed/timeout behavior.
- [ ] Differential parity harness is not yet a strict JS-vs-Rust contract harness.
- [ ] Release/target matrix and artifact pipeline are incomplete.
- [ ] Rust toolchain is not pinned to an explicit version/MSRV.
- [ ] Benchmarks are present but methodology/resource measurements are not reproducible enough for a 10/10 closeout.
- [ ] Final repository-wide zero-JS-CLI audit has not been recorded as evidence.
- [ ] Final plan/evidence synchronization was missing and is now being tracked on this PR branch.

## Goal

Replace the three executable JavaScript CLIs with maintainable Rust binaries while preserving the documented public CLI contract and security behavior, then close the migration only after parity, security, CI/release, reliability, benchmark, and repository-audit gates pass.

The final state is **Rust-only for these three executable CLIs**. JavaScript/TypeScript may remain in the Nuxt/application layer and in TypeScript tool factories required by application consumers. Those are not CLI migration targets.

## Non-goals

- Do not migrate Nuxt, Vue, Nuxt server/runtime, or application-facing TypeScript APIs.
- Do not perform a repository-wide TypeScript-to-Rust rewrite.
- Do not remove TypeScript tool factories merely because their CLI wrappers move to Rust.
- Do not keep a permanent JavaScript CLI fallback after cutover.
- Do not weaken security controls for compatibility.
- Do not add generic Rust application infrastructure unrelated to these CLIs.
- Do not change existing web/API contracts.

## Completion gates

### 1. CLI-only inventory

**Status: 🟢 DONE / baseline established.**

- [x] Three executable CLI entrypoints identified.
- [x] CLI/application boundary identified.
- [x] Nuxt/application runtime explicitly excluded.
- [x] Migration limited to executable CLI layer.

Remaining evidence:

- [ ] Retain a behavior/dependency matrix for future maintenance.

### 2. Rust architecture and toolchain

**Status: 🟡 PARTIAL.**

- [x] Rust workspace and binaries exist.
- [x] Basic CI quality checks exist.
- [x] Formatting and Clippy checks are enforced.
- [x] Workspace tests run in CI.

Still required:

- [ ] Explicit Rust toolchain pin (`rust-toolchain.toml` or equivalent).
- [ ] Explicit MSRV policy.
- [ ] Supported OS/architecture matrix.
- [ ] Dependency/features rationale.
- [ ] Release profile/distribution strategy.

**Gate:** clean checkout uses the documented toolchain and fmt/Clippy/tests pass.

### 3. Strict differential parity

**Status: 🔴 NOT DONE.**

Current tests are useful integration/smoke tests, but they do not yet constitute a strict differential harness.

Required:

- [ ] Same input corpus executed against JS oracle and Rust implementation during migration.
- [ ] Exact exit-status comparison.
- [ ] Exact stdout comparison where contractual.
- [ ] Exact stderr comparison where contractual.
- [ ] Structured error-category comparison where wording is intentionally non-contractual.
- [ ] Explicit equivalence rules; no generic `Error:` prefix matching.
- [ ] Deterministic HTTP fixtures.
- [ ] Deterministic subprocess fixtures.
- [ ] Malformed-input cases.
- [ ] Timeout cases.
- [ ] Dependency-failure cases.
- [ ] Boundary cases.

Because the JS entrypoints are now deleted, preserve any required JS-oracle fixtures/results as migration evidence rather than relying on a deleted runtime executable.

**Gate:** every documented CLI contract item has deterministic parity evidence.

### 4. `terminal-tool` correctness and process safety

**Status: 🔴 GAP — core implementation exists, acceptance incomplete.**

PR #99 uses Rust process execution with explicit argument vectors, which is the correct architectural direction.

Still required:

#### CLI compatibility

- [ ] Restore the documented `--timeout` option.
- [ ] Preserve JS-compatible timeout semantics.
- [ ] Test timeout override values.
- [ ] Test default timeout.
- [ ] Test invalid timeout values.

#### Process lifecycle

- [ ] Prove timed-out child is terminated deterministically.
- [ ] Prove no uncontrolled child/descendant remains after timeout where applicable.
- [ ] Add regression test for timeout cleanup.

#### Argument boundaries

- [ ] Argument containing spaces.
- [ ] Empty argument.
- [ ] Leading `-` argument.
- [ ] Shell metacharacters treated as literal arguments.
- [ ] Quotes preserved correctly.
- [ ] Multiple arguments retain exact boundaries.

#### Process behavior

- [ ] Executable-not-found.
- [ ] Non-zero child exit.
- [ ] stdout preservation.
- [ ] stderr preservation.
- [ ] cwd behavior.
- [ ] Environment inheritance/filtering.
- [ ] Signal/interrupt semantics where supported.
- [ ] Guard semantics.
- [ ] `--no-guard` semantics.

**Gate:** terminal behavior matches the documented contract and timeout cannot leave uncontrolled processes.

### 5. `curl-tool` security and compatibility

**Status: 🟡 PARTIAL — basic SSRF guard exists, comprehensive acceptance incomplete.**

Already verified:

- [x] Rust curl CLI exists.
- [x] Basic localhost/private-address blocking exists.
- [x] Basic IP validation exists.
- [x] Basic request method/header/body support exists.

Still required:

- [ ] Explicit CLI safe-URL policy documented separately from application/runtime SSRF policy.
- [ ] Loopback/private/link-local coverage.
- [ ] IPv4 edge cases.
- [ ] IPv6 edge cases.
- [ ] IPv4-mapped IPv6 cases.
- [ ] Hostname resolution behavior.
- [ ] Hostname resolving to private address blocked.
- [ ] Hostname resolving to public address allowed where policy permits.
- [ ] DNS/rebinding-sensitive behavior evaluated.
- [ ] Validation-vs-connection TOCTOU risk addressed or explicitly bounded by design.
- [ ] Redirect policy documented.
- [ ] Redirect to private address tested.
- [ ] Redirect to loopback tested.
- [ ] Redirect to link-local tested.
- [ ] Redirect re-validation behavior explicitly tested.
- [ ] Redirect behavior in `--no-guard` mode tested.
- [ ] Malformed URLs.
- [ ] Unsafe destinations.
- [ ] Allowed public destination.
- [ ] Explicit `--no-guard` semantics.
- [ ] Timeout/network failure behavior.
- [ ] Repeated headers.
- [ ] Body and method semantics.
- [ ] stdout/stderr and exit-code behavior.

**Gate:** security tests prove guarded requests cannot reach prohibited destinations under the defined policy, while allowed requests and explicit bypass behavior remain compatible.

### 6. `searxng-search-tool` deterministic HTTP behavior

**Status: 🟡 PARTIAL — implementation exists, fixture coverage incomplete.**

- [x] Rust SearXNG CLI exists.
- [x] Basic CLI argument parsing exists.
- [x] Basic integration test exists.

Required deterministic local/mock HTTP tests:

- [ ] Successful response.
- [ ] Empty results.
- [ ] Malformed JSON.
- [ ] Unexpected response shape.
- [ ] HTTP 4xx/5xx.
- [ ] Connection failure.
- [ ] Timeout.
- [ ] Custom `--base-url`.
- [ ] Query encoding.
- [ ] Output/error behavior.

**Gate:** no public SearXNG service is required for CI tests; all contract cases are deterministic.

### 7. pnpm/workspace integration

**Status: 🟡 PARTIAL.**

- [x] Rust packages integrated into repository.
- [x] JavaScript `bin` mappings removed.
- [x] Rust binaries are intended CLI implementations.
- [x] Nuxt/application runtime remains unchanged.

Still required:

- [ ] Document fresh-checkout build/install workflow.
- [ ] Verify no developer-specific absolute paths.
- [ ] Verify Rust binary resolution on supported platforms.
- [ ] Verify no JS fallback selector remains.
- [ ] Verify Nuxt/application TypeScript consumers remain functional.

**Gate:** clean checkout invokes the intended Rust binary without changing Nuxt/runtime architecture.

### 8. CI, release, and supply chain

**Status: 🟡 PARTIAL — basic CI exists; production release gates do not.**

Already present:

- [x] `cargo fmt --check`.
- [x] Clippy with warnings denied.
- [x] Workspace tests.
- [x] Build job.

Still required:

- [ ] Strict parity/integration suite in CI.
- [ ] Pinned Rust toolchain.
- [ ] Dependency/security audit strategy.
- [ ] Supported release target matrix.
- [ ] Release-mode artifacts.
- [ ] Artifact naming/versioning.
- [ ] Checksums.
- [ ] Signatures/provenance where required.
- [ ] Clean-checkout release verification.
- [ ] Installation verification.
- [ ] Rollback verification.

**Gate:** CI and release workflow can produce and verify every promised CLI artifact from a clean checkout.

### 9. Performance/reliability benchmark

**Status: 🟡 PARTIAL.**

PR #99 contains initial JS-vs-Rust timing results, but peak RSS is not measured and methodology is not sufficiently reproducible.

Still required:

- [ ] Document benchmark commands.
- [ ] Fixed test inputs/fixtures.
- [ ] Cold-start methodology.
- [ ] Warm-start methodology where meaningful.
- [ ] Iterations/sample count.
- [ ] Hardware/toolchain/environment recorded.
- [ ] Peak RSS measurement.
- [ ] Binary size.
- [ ] Latency/throughput results.
- [ ] Network/subprocess latency separated where practical.
- [ ] No unsupported performance claims.

**Gate:** another developer can reproduce the benchmark and obtain comparable measurements.

### 10. Zero-JS-CLI cutover

**Status: 🟢 CORE CUTOVER DONE / FINAL AUDIT OPEN.**

Already done:

- [x] Delete `packages/terminal-tool/bin/cli.mjs`.
- [x] Delete `packages/curl-tool/bin/cli.mjs`.
- [x] Delete `packages/searxng-search-tool/bin/cli.mjs`.
- [x] Remove JavaScript package `bin` mappings.
- [x] Rust binaries are the intended CLI implementations.
- [x] Nuxt/application TypeScript remains outside the CLI migration scope.

Still required:

- [ ] Repository-wide search for old JS CLI launchers.
- [ ] Repository-wide search for `USE_RUST_CLI`.
- [ ] Repository-wide search for equivalent fallback flags.
- [ ] Search for stale Node CLI scripts.
- [ ] Audit CLI-only JS dependencies.
- [ ] Remove CLI-only JS dependencies proven unused.
- [ ] Audit docs for old JS CLI invocation.
- [ ] Audit scripts for old JS CLI invocation.
- [ ] Record final zero-JS-CLI evidence.

**Hard invariant:** after completion, none of the three migrated tools may have a JavaScript executable CLI implementation, launcher, fallback, or JS `bin` mapping.

### 11. Final repository audit and closeout

**Status: 🔴 NOT DONE.**

Final audit must verify:

- [ ] All Definition of Done items pass.
- [ ] Zero executable JS CLI paths remain for all three tools.
- [ ] No stale package scripts/bin mappings remain.
- [ ] CLI-only obsolete JS dependencies are removed where unused.
- [ ] Nuxt/application imports still pass.
- [ ] CI is green.
- [ ] Supported release artifacts are verified.
- [ ] Benchmark evidence is recorded.
- [ ] Rollback procedure is documented and tested.
- [ ] Final PR/merge evidence is recorded.
- [ ] `.agents/plans/README.md` is updated.

Only after this gate may Status become `COMPLETED`.

## Definition of Done

### Scope

- [x] Migration limited to the three executable CLIs.
- [x] Nuxt/Vue/Nuxt server/application runtime explicitly remains unchanged and out of scope.
- [x] Application-facing TypeScript APIs remain outside scope.

### Implementation

- [x] Rust implementations exist for all three CLIs.
- [x] Three JS CLI entrypoints deleted.
- [x] JS `bin` mappings removed.
- [ ] Rust toolchain/MSRV/platform policy finalized.

### Parity

- [ ] Strict differential harness complete.
- [ ] Terminal `--timeout` contract restored and verified.
- [ ] Terminal process termination verified.
- [ ] Terminal adversarial argument/process cases complete.
- [ ] Curl SSRF/security matrix complete.
- [ ] Curl redirect/DNS behavior verified.
- [ ] SearXNG deterministic HTTP fixture suite complete.

### Integration

- [x] Basic Rust CI/build/test integration exists.
- [ ] Fresh-checkout CLI installation/build workflow verified.
- [ ] No JS fallback selector remains.
- [ ] Nuxt/application TypeScript consumers remain functional.

### Release / quality

- [ ] Pinned Rust toolchain and MSRV.
- [ ] Security/dependency audit.
- [ ] Supported-target release builds.
- [ ] Checksums/artifact verification.
- [ ] Rollback evidence.
- [ ] Reproducible benchmark evidence.

### Final audit

- [ ] Repository-wide zero-JS-CLI audit.
- [ ] Documentation/scripts no longer reference old JS CLI entrypoints.
- [ ] Plan README/closeout metadata updated.
- [ ] Final PR/merge evidence recorded.
- [ ] Plan status changed to `COMPLETED` only after every required gate passes.

## Rollback strategy

Before final JS cleanup, preserve enough migration evidence to compare against the old JS behavior. After the Rust-only cutover, rollback must use a known-good Rust artifact or revert the integration/release commit; a permanent JavaScript fallback is not allowed.

If a regression appears:

1. Preserve the failing input as a deterministic regression test.
2. Fix the Rust implementation.
3. Re-run parity/security/integration/release checks.
4. Re-run benchmarks where relevant.
5. Only then close the gate.

## Evidence log

| Area | Status | Evidence / current reality |
|---|---|---|
| Rust CLIs | 🟢 Done | PR #99 implements all three Rust binaries |
| JS CLI entrypoints | 🟢 Done | Three `bin/cli.mjs` files deleted |
| JS `bin` mappings | 🟢 Done | Package mappings removed |
| Nuxt/web scope | 🟢 Done | No Nuxt/runtime migration in PR #99 |
| Terminal argument boundary | 🟢/🟡 Partial | Rust uses explicit argument vector; adversarial coverage incomplete |
| Terminal timeout | 🔴 Gap | `--timeout` contract missing; fixed 30s implementation |
| Child termination | 🔴 Gap | No deterministic no-descendant evidence yet |
| Curl basic SSRF | 🟢/🟡 Partial | Basic local/private blocking exists |
| Curl comprehensive security | 🔴 Gap | Edge/redirect/DNS policy coverage incomplete |
| SearXNG implementation | 🟢/🟡 Partial | Rust implementation exists; mock fixture matrix incomplete |
| Differential parity | 🔴 Gap | Current tests are not strict JS-vs-Rust differential tests |
| Basic CI | 🟢 Done | fmt/Clippy/test/build present |
| Release CI | 🔴 Gap | Target matrix/artifacts/checksums not complete |
| Toolchain/MSRV | 🔴 Gap | Explicit pin/MSRV missing |
| Benchmark | 🟡 Partial | Initial timing exists; methodology/RSS incomplete |
| Final zero-JS audit | 🔴 Gap | Needs repository-wide evidence |
| Plan closeout | 🔴 Gap | Must remain IN PROGRESS until all gates pass |

## Final closeout rule

**Plan 027 is NOT complete yet.**

Do not mark it `COMPLETED` merely because PR #99 has Rust implementations and removed the old JS entrypoints.

Plan 027 becomes `COMPLETED` only when all remaining red/yellow gates are green, especially:

1. Terminal timeout + child termination.
2. Curl security parity.
3. Deterministic SearXNG fixtures.
4. Strict differential parity.
5. Pinned toolchain/MSRV.
6. Release/target artifacts.
7. Reproducible benchmarks.
8. Repository-wide zero-JS-CLI audit.
9. Final documentation, rollback, and PR evidence.
