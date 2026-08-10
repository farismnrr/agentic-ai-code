# Rust CLI Differential Parity

As part of the JS-to-Rust CLI migration (Plan 027, Step 3), this memory documents the differential parity strategy used to guarantee that the new Rust binaries perfectly mimic the removed Node.js executables. 

## 1. JS Oracles and The Contract
Because the original `cli.mjs` files were deleted in PR #99, we restored their final known-good state from Git history and preserved them in `packages/rust-tools/tests/` as "JS Oracles". 

These oracles serve as the indisputable contract. The parity test harness runs an identical set of edge cases against both the JS oracle (`node --experimental-strip-types`) and the Rust binary, ensuring outputs match.

## 2. Parity Test Harness (`differential_parity.rs`)
A dedicated integration test suite was created (`cargo test --test differential_parity`) with strict comparisons:
- **Exit Codes:** Must be identical (e.g. 0 on success, >0 on various specific failures).
- **Standard Output (stdout):** Exact string or JSON schema parity.
- **Standard Error (stderr):** Exact matches where contractual, and categorized matches otherwise.

## 3. Structured Error Equivalence
Node.js and Rust generate different low-level error strings (e.g., Node's `fetch failed` vs. Rust's `reqwest` network errors, or `ENOENT` vs `No such file or directory`). 
The harness includes explicit equivalence normalization rules so that identical failure modes are treated as parity successes instead of false-positive regressions.

## 4. Deterministic Fixtures
- **Local TCP Mock Server:** A deterministic local HTTP server is used during tests to feed exact success/failure/timeout JSON payloads to `searxng-search-tool` and `curl-tool`. This eliminates public network flakiness.
- **Timeout Proving:** Validated the internal `--timeout` option using sleep commands, confirming that the Rust CLI terminates child processes exactly as the JS CLI did. 
- **Edge Cases Tested:** Boundary conditions, missing arguments, malformed URLs, execution bypasses (`--no-guard`), and bad dependencies (missing binaries).

## 5. Maintenance Rule
If a regression in the CLI is discovered, a failing deterministic case MUST be added to `differential_parity.rs` to reproduce the JS output expectation before fixing the Rust implementation.
