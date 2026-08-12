# Plan 027 — Final Closeout

**Date:** 2026-08-10  
**Plan:** [027-cli-rust-refactor.md](../plans/027-cli-rust-refactor.md)  
**Status:** ✅ COMPLETED

---

## Summary

Plan 027 is complete. The executable CLI layer for `terminal-tool`, `curl-tool`, and `searxng-search-tool` was migrated from JavaScript to Rust. Those three tools no longer have supported JavaScript CLI entrypoints, launchers, npm `bin` mappings, or permanent JavaScript fallback paths.

The Nuxt application and TypeScript LangChain/AI SDK tool factories were explicitly outside this CLI migration and remain valid application APIs.

---

## Gate-by-gate verification

| Gate | Result | Evidence |
|------|--------|----------|
| Zero JS CLI entrypoints | ✅ PASS | The three migrated package `bin/cli.mjs` entrypoints were removed; see [`027-zero-js-cli-cutover.md`](027-zero-js-cli-cutover.md) |
| No stale `bin` mappings | ✅ PASS | The three migrated package manifests no longer expose JavaScript CLI `bin` mappings |
| No obsolete CLI-only JS deps | ✅ PASS | Plan 027 audit removed/verified CLI-only dependency residue for the migrated layer |
| TypeScript application APIs intact | ✅ PASS | `packages/{terminal,curl,searxng-search}-tool/src/index.ts` remain the application-facing APIs |
| Rust binaries present | ✅ PASS | `packages/rust-tools/src/bin/{terminal-tool,curl-tool,searxng-search-tool}.rs` are the executable implementations |
| Historical quality gate | ✅ PASS | At Plan 027 closeout the repository used workflow-backed Rust fmt/check/Clippy/audit. That CI evidence is historical; the current repository uses the mandatory local commit gate documented in [`no-ci-local-commit-gates.md`](no-ci-local-commit-gates.md). |
| Benchmark evidence | ✅ PASS | [`027-performance-benchmark.md`](027-performance-benchmark.md) records the migration measurements |
| Rollback documented | ✅ PASS | [`027-rust-release-supply-chain.md`](027-rust-release-supply-chain.md) documents current manual release/rollback posture |
| PR/merge evidence | ✅ PASS | PR #99 (`feat/027-p1-rust-cli-tools` → `dev`) |
| Plan index updated | ✅ PASS | [`../plans/README.md`](../plans/README.md) lists Plan 027 as completed |

> Historical Plan 027 evidence may mention `.github/workflows/rust-ci.yml` or the later consolidated `.github/workflows/ci.yml`. The repository intentionally has no CI now. Those old workflow names document what was used during the migration; they are not current operating instructions.

---

## Core memory files from Plan 027

| File | Covers |
|------|--------|
| [`027-rust-architecture-toolchain.md`](027-rust-architecture-toolchain.md) | Rust workspace/toolchain boundary |
| [`027-strict-differential-parity.md`](027-strict-differential-parity.md) | Migration parity strategy |
| [`027-terminal-tool-process-safety.md`](027-terminal-tool-process-safety.md) | Terminal process safety |
| [`027-curl-tool-ssrf-policy.md`](027-curl-tool-ssrf-policy.md) | Curl SSRF policy |
| [`027-searxng-deterministic-fixtures.md`](027-searxng-deterministic-fixtures.md) | SearXNG deterministic fixtures |
| [`027-pnpm-workspace-integration.md`](027-pnpm-workspace-integration.md) | pnpm/Rust workspace integration |
| [`027-rust-release-supply-chain.md`](027-rust-release-supply-chain.md) | Release and rollback |
| [`027-performance-benchmark.md`](027-performance-benchmark.md) | Performance evidence |
| [`027-zero-js-cli-cutover.md`](027-zero-js-cli-cutover.md) | Final executable-layer cutover invariant |
| `027-final-closeout.md` | Final closeout (this file) |

Additional Plan 027 memories are indexed in [`README.md`](README.md).

---

## Invariants upheld

- **Hard invariant:** No JavaScript executable CLI implementation, launcher, fallback, or npm `bin` mapping is supported for the three migrated tools.
- **Scope invariant:** Nuxt/Vue application code and TypeScript tool factories are separate application concerns, not JavaScript CLI fallbacks.
- **Quality invariant (current):** every commit must pass the repository-local policy/lint/type gate; CI and unit-test suites are intentionally absent.
- **Rollback:** A permanent JavaScript CLI fallback is disallowed; rollback uses a known-good Rust artifact or reverts the integration/release change.
