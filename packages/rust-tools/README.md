# Rust Tools

This workspace contains the native implementations of:

- `terminal-tool`
- `curl-tool`
- `searxng-search-tool`
- `relay-agent`

The three tool CLIs were migrated from JavaScript during historical Plan 027. `relay-agent` was subsequently rewritten as a native Rust MCP server during historical Plan 028. There is no supported JavaScript CLI fallback path.

## Toolchain

- **Edition:** Rust 2021
- **MSRV:** 1.88.0 (`Cargo.toml`)
- **Repository-pinned toolchain:** Rust 1.95.0 (`rust-toolchain.toml`)

Use the pinned toolchain for repository development/verification. The MSRV is a package compatibility floor, not the normal repository compiler.

## Architecture

The sibling CLIs are small native executors:

- `terminal-tool` — process execution with explicit guard/allow controls and timeout/process-group handling.
- `curl-tool` — HTTP client with SSRF protections unless the explicit guard bypass is requested.
- `searxng-search-tool` — SearXNG query client.
- `relay-agent` — MCP `2026-07-28` server exposing controlled coding capabilities through the relay security boundary.

`relay-agent` resolves trusted sibling binaries relative to its own executable rather than trusting arbitrary `$PATH`. The installation directory is therefore part of the trust boundary and must not be writable by the unprivileged runtime user.

## Relay security/platform contract

- **Linux only.** Relay containment requires Bubblewrap (`bwrap`).
- **Unprivileged runtime.** The relay refuses UID 0.
- **Filesystem containment.** Execution is constrained to the configured execution root through Bubblewrap plus server policy.
- **Local/remote modes.** Local is loopback-oriented; remote is OAuth-protected and fail-closed.
- **Docker is deferred.** Do not expose the host Docker socket as a workaround for missing isolated Docker execution.

See [`../relay-agent/SKILL.md`](../relay-agent/SKILL.md), the canonical [memory](../../.agents/memories/README.md#rust-cli-migration-invariants), and [Plan 030 history](../../.agents/plans/030-previous-plans-summary.md) before changing these boundaries.

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

Security-sensitive relay/MCP changes may additionally require `cargo audit`, `scripts/phase8-zero-bypass.sh`, and the deterministic acceptance scripts relevant to the current relay/MCP contract.

The old JavaScript parity harness is obsolete and is not a current verification source of truth.

## Release policy

There is **no automated GitHub Actions release workflow**. Native releases are a manual/operator action after local verification.

The supported relay release target remains **`x86_64-unknown-linux-gnu`** because production containment requires Linux + Bubblewrap. Do not document macOS/Windows relay support merely because simpler sibling CLI binaries may compile there.

When publishing native artifacts manually:

- build from the reviewed commit with the pinned Rust toolchain;
- run the mandatory local commit gate plus applicable Rust security checks;
- package intended Linux artifact(s);
- generate and publish SHA-256 checksums;
- do not weaken sandbox/platform contracts merely to broaden the release matrix.

## CLI notes

Package-level TypeScript tool factories under sibling `packages/*-tool/` are still application APIs, but standalone executable CLIs are Rust binaries in this workspace. Package skill docs must not advertise removed `npx @ai-code/*` bin mappings.

Use each binary's `--help` as the command-line source of truth.
