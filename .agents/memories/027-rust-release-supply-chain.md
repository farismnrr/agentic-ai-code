# 027 Rust CLI Release & Supply Chain

## CI Pipeline

The CI pipeline for the Rust CLI tools (`terminal-tool`, `curl-tool`, `searxng-search-tool`) has been expanded into two primary phases in `.github/workflows/rust-ci.yml`:

1.  **Test and Quality Gates (`test` job):**
    *   **Formatting and Linting:** Enforces `cargo fmt` and `cargo clippy -- -D warnings`.
    *   **Security Audit:** Integrates `cargo audit` to automatically enforce a dependency security audit strategy on every PR and push.
    *   **Testing:** Executes the strict differential parity test suite (`cargo test --workspace`) ensuring no regressions against the baseline.
    *   **Toolchain Pinning:** Rust toolchain is explicitly pinned to `1.80.0` to avoid sudden compilation issues from newer compiler versions.

2.  **Cross-Compilation and Release (`build-release` job):**
    *   Executes only on pushes to `main` and `dev` (conditional to successful tests).
    *   Targets a matrix including `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, and `aarch64-apple-darwin`.
    *   Utilizes `cross` for seamless cross-compilation where native runners are unsuitable.
    *   Generates release-profile artifacts (`cargo build --release`).

## Checksum Generation & Provenance

During the release phase, the pipeline packages compiled binaries and generates standard `sha256sum` hashes for every individual platform binary. The checksum format relies on standard `sha256sum` (or `shasum -a 256` on macOS runners) and exports a `.sha256` file next to each executable artifact.

*   Artifacts are uploaded securely using `actions/upload-artifact@v4`.
*   This establishes a provenance trail back to the originating GitHub Actions run.
*   While detailed Sigstore provenance (SLSA) is not explicitly required at this stage, the deterministic build environment and sha256 checksums form a solid baseline.

## Clean-Checkout & Rollback Verification

The artifacts and integration are intended to guarantee a reliable rollback and execution model:

*   **Clean-Checkout Installation:** A clean checkout simply runs the target Rust binaries. Because the integration does not rely on local developer paths or `.mjs` files anymore, the binaries are explicitly the sole source of truth.
*   **Fallback / Rollback Procedure:**
    1.  A permanent JavaScript fallback is **not allowed** by the plan.
    2.  If a CLI regression is discovered in production, the rollback procedure involves reverting the specific release commit that introduced the bug, or picking a previously validated GitHub Actions artifact (from the `release-artifacts` payload) and running it directly.
    3.  Because the `rust-ci.yml` matrix explicitly pins the toolchain, a developer can check out a previous commit and reliably reproduce the exact binary that was rolled back.

## Verification Checklist

- [x] Matrix build works across architectures.
- [x] Checksum (`.sha256`) is bundled with every released binary.
- [x] Toolchain (`1.80.0`) provides deterministic reproduction.
- [x] `cargo audit` actively denies compromised dependencies in the supply chain.
