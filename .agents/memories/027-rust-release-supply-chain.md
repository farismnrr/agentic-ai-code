# 027 Rust CLI Release & Supply Chain

Plan 027 established the native-tool release/supply-chain baseline. The workflow layout and supported target matrix changed later during the relay-agent security hardening, so this memory records both the durable decision and the **current** repository reality.

## Durable decisions

- Native CLI artifacts are built from reviewed Rust source with a pinned repository toolchain.
- Rust quality/security gates are blocking: formatting, warnings-denied check/Clippy, and `cargo audit` must not be bypassed just to publish.
- Released native artifacts carry SHA-256 checksums.
- A permanent JavaScript executable fallback is not part of rollback. Roll back to a known-good Rust artifact/commit instead.
- The package MSRV and the repository-pinned compiler are different concepts: `Cargo.toml` declares MSRV 1.88.0 while repository CI/development pins Rust 1.95.0.

## Current CI/release source of truth

The old Plan 027 workflow name `.github/workflows/rust-ci.yml` was later consolidated/superseded. **Current CI and native release behavior lives in `.github/workflows/ci.yml`.** Read that workflow before changing or documenting exact job names, triggers, or targets.

Current relay/native release policy is intentionally narrower than the original Plan 027 cross-platform matrix:

- release target currently packaged by CI: `x86_64-unknown-linux-gnu`;
- `relay-agent` is Linux-only because the production sandbox boundary requires Bubblewrap (`bwrap`);
- macOS/Windows relay targets were removed rather than shipping an insecure no-sandbox fallback;
- do not infer relay platform support from whether a simpler sibling CLI can compile on another OS.

The original Plan 027 matrix references to macOS/aarch64 are historical migration evidence, not the current release promise.

## Verification baseline

Current repository CI enforces commands equivalent to:

```sh
cd packages/rust-tools
cargo fmt --all -- --check
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo audit
```

The workflow also carries the current deterministic security/release gates for `relay-agent`; verify exact scripts and dependencies in `.github/workflows/ci.yml` because later Plan 028/029 work intentionally strengthened the Plan 027 baseline.

## Checksum and provenance posture

Release packaging produces SHA-256 checksum material for the intended native artifacts and ties artifacts to the reviewed GitHub Actions run/commit. Do not claim Sigstore/SLSA signing unless the current workflow actually implements it; checksums and CI traceability are the established baseline.

## Rollback

1. Do not restore a permanent JavaScript CLI fallback.
2. Revert the offending native integration/release commit or deploy a previously validated Rust artifact.
3. Re-run the current CI/security gates against the rollback candidate.
4. Treat release target/platform changes as security-boundary changes when `relay-agent` containment is involved.

See [`027-zero-js-cli-cutover.md`](027-zero-js-cli-cutover.md), [`028-relay-agent-phase19-security-decisions.md`](028-relay-agent-phase19-security-decisions.md), [`../../packages/rust-tools/README.md`](../../packages/rust-tools/README.md), and [`../plans/028-relay-agent-rust-rewrite.md`](../plans/028-relay-agent-rust-rewrite.md).