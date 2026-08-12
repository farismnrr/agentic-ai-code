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
- **Repository-pinned toolchain:** Rust 1.95.0 (`rust-toolchain.toml`)

Use the pinned toolchain for repository development/verification. The MSRV is a package compatibility floor, not the normal repository compiler.

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

See [`../relay-agent/SKILL.md`](../relay-agent/SKILL.md), [Plan 028](../../.agents/plans/028-relay-agent-rust-rewrite.md), and [Plan 029b](../../.agents/plans/029b-chatgpt-mcp-production-hardening.md) before changing these boundaries.

## Build

From repository root:

```bash
pnpm build:tools
```

Or directly:

```bash
cargo build --manifest-path packages/rust-tools/Cargo.toml --release --locked
```

## Mandatory commit verification

The repository has **no CI** and **no unit-test suite**. Rust quality is part of the mandatory local commit gate:

```bash
pnpm verify:commit
```

The root commands include:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
```

Security-sensitive relay/MCP changes may additionally require:

```bash
cargo audit
bash scripts/phase8-zero-bypass.sh
```

and the deterministic acceptance scripts relevant to Plan 029/029b.

The old JavaScript parity harness is obsolete and is not a current verification source of truth.

## Release policy

There is **no automated GitHub Actions release workflow**. Native releases are a manual/operator action after local verification.

The supported relay release target remains **`x86_64-unknown-linux-gnu`** because production containment requires Linux + Bubblewrap. Do not document macOS/Windows relay support merely because one of the simpler sibling CLI binaries can compile there.

When publishing native artifacts manually:

- build from the reviewed commit with the pinned Rust toolchain;
- run the mandatory local commit gate plus applicable Rust security checks;
- package the intended Linux artifact(s);
- generate and publish SHA-256 checksums;
- do not weaken the sandbox/platform contract merely to broaden the release matrix.

## CLI notes

The package-level TypeScript tool factories under sibling `packages/*-tool/` are still application APIs, but the standalone executable CLIs are the Rust binaries in this workspace. Package skill docs must not advertise removed `npx @ai-code/*` bin mappings.

Use each binary's `--help` as the command-line source of truth:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin terminal-tool -- --help
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin curl-tool -- --help
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin searxng-search-tool -- --help
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin relay-agent -- --help
```
