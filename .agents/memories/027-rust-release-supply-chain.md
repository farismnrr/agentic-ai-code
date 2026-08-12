# 027 Rust CLI Release & Supply Chain

Plan 027 established the native-tool release/supply-chain baseline. Later relay security work narrowed the supported platform contract, and the repository subsequently removed CI entirely. This memory records the durable decisions plus the **current** manual/local verification posture.

## Durable decisions

- Native CLI artifacts are built from reviewed Rust source with a pinned repository toolchain.
- Rust quality/security gates are blocking: formatting, warnings-denied check/Clippy, and applicable security checks must not be bypassed just to publish.
- Released native artifacts carry SHA-256 checksums.
- A permanent JavaScript executable fallback is not part of rollback. Roll back to a known-good Rust artifact/commit instead.
- The package MSRV and the repository-pinned compiler are different concepts: `Cargo.toml` declares MSRV 1.88.0 while repository development pins Rust 1.95.0.

## Current repository reality

The old workflow files referenced by Plan 027/028 are historical. The repository now intentionally has **no `.github/workflows/` CI configuration** and no automated release workflow.

Current relay/native platform policy remains intentionally narrow:

- supported relay release target: `x86_64-unknown-linux-gnu`;
- `relay-agent` is Linux-only because the production sandbox boundary requires Bubblewrap (`bwrap`);
- macOS/Windows relay targets must not be reintroduced without an equivalent secure containment design;
- do not infer relay platform support from whether a simpler sibling CLI can compile on another OS.

The original Plan 027 matrix references to macOS/aarch64 are historical migration evidence, not the current release promise.

## Verification baseline

Every commit must pass the repository-local gate:

```sh
pnpm verify:commit
```

For the Rust workspace that includes formatting, warnings-denied `cargo check`, and Clippy. Security-sensitive native/relay work should also run `cargo audit` and the relevant deterministic scripts under `scripts/`.

There is no remote CI safety net. Do not publish a native artifact from a commit whose required local verification was bypassed or failed.

## Release/checksum posture

Native releases are manual/operator actions. When publishing:

1. build from the reviewed commit with the pinned Rust toolchain;
2. run the mandatory local commit gate and applicable security/acceptance checks;
3. package only supported target artifacts;
4. generate SHA-256 checksum files for published binaries;
5. retain commit/tag provenance in the release record.

Do not claim GitHub Actions provenance, Sigstore, SLSA, or another signing/attestation mechanism unless the repository actually implements it at that time.

## Rollback

1. Do not restore a permanent JavaScript CLI fallback.
2. Revert the offending native integration/release commit or deploy a previously validated Rust artifact.
3. Re-run the current local commit/security gates against the rollback candidate.
4. Treat release target/platform changes as security-boundary changes when `relay-agent` containment is involved.

See [`027-zero-js-cli-cutover.md`](027-zero-js-cli-cutover.md), [`028-relay-agent-phase19-security-decisions.md`](028-relay-agent-phase19-security-decisions.md), [`no-ci-local-commit-gates.md`](no-ci-local-commit-gates.md), [`../../packages/rust-tools/README.md`](../../packages/rust-tools/README.md), and [`../plans/028-relay-agent-rust-rewrite.md`](../plans/028-relay-agent-rust-rewrite.md).
