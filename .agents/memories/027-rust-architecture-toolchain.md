# Rust Architecture and Toolchain

As part of the JS-to-Rust CLI migration (Plan 027), this memory documents the baseline toolchain, compilation profiles, and target support matrix for the migrated tools.

## 1. Toolchain & MSRV

- **Pinned Toolchain:** The `packages/rust-tools/rust-toolchain.toml` specifies the toolchain channel as `1.95.0` to guarantee reproducible builds across developer machines and CI pipelines. It includes `rustfmt` and `clippy` components for strict style enforcement.
- **MSRV Policy:** The Minimum Supported Rust Version (MSRV) is explicitly declared in `Cargo.toml` as `rust-version = "1.75.0"`. This strikes a balance between relying on a modern 2021 edition feature set and maximizing backwards compatibility for older host build environments.

## 2. Release Profile & Distribution

To ensure the CLI binaries are highly performant with minimal footprint (replacing heavier Node.js scripts), the following release profile is applied in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3       # Maximize speed
lto = true          # Enable Link-Time Optimization for smaller/faster binaries
codegen-units = 1   # Maximize optimization at the cost of compile time
strip = true        # Strip debug symbols to dramatically reduce binary size
```

This configuration creates optimized, zero-dependency standalone executables that start instantly, fulfilling the "zero JS CLI" performance goals.

## 3. Supported OS & Architecture Matrix

The CLI tools compile to native binaries targeting the environments where the `ai-code` agent/application stack is typically hosted.

| OS | Architectures | Status | Target Triple (Rust) |
|---|---|---|---|
| **Linux** | x86_64, aarch64 | Primary | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` |
| **macOS** | aarch64 (Apple Silicon), x86_64 | Supported | `aarch64-apple-darwin`, `x86_64-apple-darwin` |

*Note: Windows is supported as a secondary target (`x86_64-pc-windows-msvc`), but deployment focuses on Linux and macOS environments.*

## 4. Dependency & Features Rationale

- `clap`: Used for standard, POSIX-compliant CLI argument parsing. It ensures deterministic boundary and quote preservation (critical for `terminal-tool`).
- `tokio`: Used because `curl-tool` and `terminal-tool` require async task execution (HTTP requests and child process timeout management).
- `reqwest`: The standard choice for robust, high-performance HTTP requests in `curl-tool` and `searxng-search-tool`.
- `hickory-resolver`: Directly required by `curl-tool` to pre-resolve DNS and enforce strict SSRF blocks (e.g., rejecting DNS rebinds to 127.0.0.1). *Note: We intentionally use `hickory-resolver` over the older `trust-dns-resolver` to mitigate an IDNA vulnerability (`RUSTSEC-2024-0421`) discovered during our `cargo audit`.*
- `shell-words`: Safely splits string commands into explicit argument vectors for `terminal-tool` to prevent shell-injection vulnerabilities.

These dependencies were chosen to provide exactly the required features for parity with the old Node.js tools while maximizing safety and minimizing bloat.
