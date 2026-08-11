# Rust Tools CLI Migration

This workspace contains the Rust migration for `relay-agent`, `terminal-tool`, `curl-tool`, and `searxng-search-tool`.

## Architecture
These tools were migrated from JS to Rust to enforce strict process boundary checks, safe-URL policies (SSRF protection), and zero JS fallback execution. The tools are designed as thin, statically-compiled native binaries that provide precise, contract-preserving behavior mirroring the original JS tools but without the Node.js overhead. 

Specifically, `relay-agent` is a native Rust MCP (Model Context Protocol) server. It exposes a standard Streamable HTTP MCP endpoint (spec 2026-07-28). It does not use Node.js and it is no longer based on a proprietary WebSocket protocol.

## Toolchain & MSRV
- **Rust Edition:** 2021
- **Toolchain:** Stable (latest)
- **MSRV:** 1.88.0 (Minimum Supported Rust Version)

## Release Policy
- Binaries are built in `release` mode for production.
- Supported targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.
- Releases are managed through GitHub Actions and attached to GitHub releases.

## Error Model
- All errors are typed internally.
- Expected contractual errors (like SSRF blocks or empty commands) return an output string prefix `Error:` and exit code `0` to match the JS oracle's behavior.
- Argument parsing errors return exit code `2`.
- Any underlying execution or missing binary errors exit with `1`.

## Development
- Format: `cargo fmt`
- Lint: `cargo clippy -- -D warnings`
- Parity Tests: `node tests/parity.mjs`
