# Plan 027 — Final Closeout

**Date:** 2026-08-10  
**Plan:** [027-cli-rust-refactor.md](../plans/027-cli-rust-refactor.md)  
**Status:** ✅ COMPLETED

---

## Summary

Plan 027 is fully complete. All three JavaScript CLI implementations
(`terminal-tool`, `curl-tool`, `searxng-search-tool`) have been migrated to
Rust and the repository is 100 % free of JavaScript CLI entrypoints,
launchers, and fallbacks.

---

## Gate-by-gate verification

| Gate | Result | Evidence |
|------|--------|----------|
| Zero JS CLI entrypoints | ✅ PASS | `grep -R "cli.mjs" packages/{terminal,curl,searxng-search}-tool` → no results |
| No stale `bin` mappings | ✅ PASS | `find packages -name package.json | xargs grep -l '"bin"'` → no results |
| No obsolete CLI-only JS deps | ✅ PASS | `minimist`, `yargs`, `commander` absent from all three tool `package.json` |
| TypeScript sources intact | ✅ PASS | `packages/{terminal,curl,searxng-search}-tool/src/index.ts` all exist |
| Rust binaries present | ✅ PASS | `packages/rust-tools/src/bin/{terminal-tool,curl-tool,searxng-search-tool}.rs` all exist |
| CI config valid | ✅ PASS | `.github/workflows/rust-ci.yml` present; Rust 1.95.0 pinned (MSRV 1.88.0), `cargo audit` configured |
| Benchmark evidence | ✅ PASS | `.agents/memories/027-performance-benchmark.md` exists (RSS: 155 MB → 14 MB; latency: 250 ms → 2 ms) |
| Rollback documented | ✅ PASS | `.agents/memories/027-rust-release-supply-chain.md` exists |
| PR/merge evidence | ✅ PASS | PR #99 (`feat/027-p1-rust-cli-tools` → `dev`) |
| README updated | ✅ PASS | `.agents/plans/README.md` moved 027 to Completed list |

---

## Memory files created during plan 027

| File | Step |
|------|------|
| `027-rust-architecture-toolchain.md` | Step 1-2 |
| `027-strict-differential-parity.md` | Step 3 |
| `027-terminal-tool-process-safety.md` | Step 4 |
| `027-curl-tool-ssrf-policy.md` | Step 5 |
| `027-searxng-deterministic-fixtures.md` | Step 6 |
| `027-pnpm-workspace-integration.md` | Step 7 |
| `027-rust-release-supply-chain.md` | Step 8 |
| `027-performance-benchmark.md` | Step 9 |
| `027-zero-js-cli-cutover.md` | Step 10 |
| `027-final-closeout.md` | Step 11 (this file) |

---

## Invariants upheld

- **Hard invariant:** No JavaScript executable CLI implementation, launcher,
  fallback, or `bin` mapping exists for any of the three migrated tools.
- **Scope invariant:** The Nuxt/Vue web application and TypeScript tool
  factories are unchanged and remain out of scope.
- **Rollback:** A permanent JavaScript fallback is disallowed; rollback uses a
  known-good Rust artifact or reverts the integration/release commit.
