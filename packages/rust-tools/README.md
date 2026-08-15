# Rust Tools

This workspace contains the native implementation of the unified `ai-tools` binary, which provides:

- `terminal`
- `curl`
- `searxng`
- `relay`

The separate tool CLIs were migrated from JavaScript during historical Plan 027 and unified into a single binary during Plan 033. There is no supported JavaScript CLI fallback path.

## Toolchain

- **Edition:** Rust 2021
- **MSRV:** 1.88.0 (`Cargo.toml`)
- **Repository-pinned toolchain:** Rust 1.95.0 (`rust-toolchain.toml`)

Use the pinned toolchain for repository development/verification. The MSRV is a package compatibility floor, not the normal repository compiler.

## Architecture

The unified native executor (`ai-tools`) exposes the following subcommands:

- `terminal` — process execution with explicit guard/allow controls and timeout/process-group handling.
- `curl` — HTTP client with SSRF protections unless the explicit guard bypass is requested.
- `searxng` — SearXNG query client.
- `relay` — MCP `2026-07-28` server exposing controlled coding capabilities through the relay security boundary.

The `relay` subcommand executes other subcommands relative to its own executable rather than trusting arbitrary `$PATH`. The installation directory is therefore part of the trust boundary and must not be writable by the unprivileged runtime user.

## Relay security/platform contract

- **Linux only.** Relay containment requires Bubblewrap (`bwrap`).
- **Unprivileged runtime.** The relay refuses UID 0.
- **Filesystem containment.** Execution is constrained to the configured execution root through Bubblewrap plus server policy.
- **Local/remote modes.** Local is loopback-oriented; remote is OAuth-protected and fail-closed.
- **Docker is deferred.** Do not expose the host Docker socket as a workaround for missing isolated Docker execution.
- **Long-running execution.** One bounded job manager owns spawn, output draining, timeout, cancellation, process-tree cleanup, retention, and concurrency for synchronous calls, MCP Tasks, and fallback jobs.
- **Timeout policy.** `timeout_ms = 0` is deadline-free unless an operator maximum is configured; terminal execution has no unconditional five-minute server ceiling.
- **Output policy.** stdout/stderr are drained continuously into bounded retained tails; exceeding retention omits older bytes instead of killing an otherwise valid process.

See [`../relay-agent/SKILL.md`](../relay-agent/SKILL.md), the canonical [memory](../../.agents/memories/README.md#rust-cli-migration-invariants), and [Plan 030 history](../../.agents/plans/030-previous-plans-summary.md) before changing these boundaries.

## Build

From repository root:

```bash
pnpm build:tools
```

Or directly:

```bash
cargo build --manifest-path Cargo.toml --release --locked --bin ai-tools
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
- build the reviewed release bundle with `pnpm release:build vX.Y.Z`;
- publish the native archive and generated `SHA256SUMS` from the exact stable tag with `pnpm release:publish vX.Y.Z`;
- keep publish operations fail-closed to a clean `main` checkout whose requested tag points at `HEAD` and is already present on `origin`;
- do not weaken sandbox/platform contracts merely to broaden the release matrix.

The GitHub Release publishes the direct `ai-tools-x86_64-unknown-linux-gnu` asset required by the UI, a `ai-tools-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` archive, and `SHA256SUMS`. The same publish command also builds and pushes the web image to GHCR for `linux/amd64` and `linux/arm64`.

## CLI notes

Package-level TypeScript tool factories under sibling `packages/*/` are still application APIs, but the standalone executable CLI is the single Rust binary (`ai-tools`) in this workspace. Package skill docs must not advertise removed `npx @ai-code/*` bin mappings.

Use each binary's `--help` as the command-line source of truth.
