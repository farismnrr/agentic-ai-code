# Rust Tools

This workspace contains the native implementations of:

- `terminal-tool`
- `curl-tool`
- `searxng-search-tool`
- `relay-agent`

The three tool CLIs were migrated from JavaScript in Plan 027. `relay-agent` was subsequently rewritten as a native Rust MCP server in Plan 028. There is no supported JavaScript CLI fallback path.

## Toolchain

- **Edition:** Rust 2021
- **MSRV:** 1.88.0 (`Cargo.toml`)
- **Repository-pinned toolchain:** Rust 1.95.0 (`rust-toolchain.toml` and CI)

Use the pinned toolchain for repository development/verification. The MSRV is a package compatibility floor, not the version CI uses.

## Architecture

The sibling CLIs are small native executors:

- `terminal-tool` — process execution with explicit guard/allow controls and timeout/process-group handling.
- `curl-tool` — HTTP client with SSRF protections unless an explicit local bypass is requested.
- `searxng-search-tool` — SearXNG query client.
- `relay-agent` — MCP `2026-07-28` server that exposes controlled coding capabilities and invokes the sibling tools through the relay security boundary.

`relay-agent` resolves trusted sibling binaries relative to its own executable rather than trusting an arbitrary `$PATH`. The installation directory is therefore part of the trust boundary and must not be writable by the unprivileged runtime user.

## Relay security/platform contract

The current relay contract is deliberately stricter than the generic sibling CLI contract:

- **Linux only.** The relay binary fails compilation on non-Linux targets because its execution sandbox requires Bubblewrap (`bwrap`).
- **Unprivileged runtime.** The relay refuses to run as UID 0.
- **Filesystem containment.** Execution is constrained to the configured execution root and enforced through Bubblewrap plus server policy.
- **Local/remote modes.** Local mode is loopback-oriented; remote mode is OAuth-protected and must fail closed.
- **Docker is deferred.** Do not expose the host Docker socket as a workaround for missing isolated Docker execution.

See [`../relay-agent/SKILL.md`](../relay-agent/SKILL.md), [Plan 028](../../.agents/plans/028-relay-agent-rust-rewrite.md), and [Plan 029b](../../.agents/plans/029b-external-mcp-mcp-production-hardening.md) before changing these boundaries.

## Build

From repository root:

```bash
pnpm build:tools
```

Or directly:

```bash
cargo build --manifest-path packages/rust-tools/Cargo.toml --release --locked
```

## Verification

Repository CI currently enforces:

```bash
cd packages/rust-tools
cargo fmt --all -- --check
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo audit
cd ../..
bash scripts/phase8-zero-bypass.sh
```

MCP-specific changes may also require the deterministic Phase 4/6/7 scripts described by the active Plan 029/029b acceptance criteria.

The old `node tests/parity.mjs` instruction is obsolete; that JavaScript parity harness is not the current verification source of truth.

## Release policy

Current GitHub Actions release packaging for the native binaries is **`x86_64-unknown-linux-gnu`**. The workflow intentionally removed macOS/Windows relay targets because there is no supported equivalent to the required Bubblewrap sandbox and no insecure fallback is allowed.

Do not document a platform as a supported relay release target merely because one of the simpler sibling CLI binaries can compile there.

Release artifacts are built from source with Cargo, packaged with SHA-256 checksums, and gated by the JS/Rust CI jobs before tagged relay releases are created.

## CLI notes

The package-level TypeScript tool factories under sibling `packages/*-tool/` are still application APIs, but the standalone executable CLIs are the Rust binaries in this workspace. Package skill docs must not advertise removed `npx @ai-code/*` bin mappings.

Use each binary's `--help` as the command-line source of truth:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin terminal-tool -- --help
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin curl-tool -- --help
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin searxng-search-tool -- --help
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin relay-agent -- --help
```
