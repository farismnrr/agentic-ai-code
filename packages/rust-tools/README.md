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

## Security & Sibling Binary Trust Boundary
- The `relay-agent` invokes `terminal-tool`, `curl-tool`, and `searxng-search-tool` by explicitly resolving them relative to its own executable directory (`std::env::current_exe()`). It does not rely on the system `$PATH`. 
- **Trust Assumption:** The directory containing `relay-agent` is considered a trust boundary. Sibling binaries within this directory are trusted. An untrusted local user must not be able to replace or tamper with these binaries.
- **Installation Requirements:** Ensure that the release/install directories are owned by `root` (or a dedicated service user) and have appropriate restrictive permissions (e.g., `755` for directories and binaries) so that unprivileged users cannot overwrite them.
- **Integrity Verification:** Consider implementing binary integrity verification (e.g., checksums or signatures) only if the deployment threat model requires protection against local binary tampering by an attacker who has already gained elevated privileges.
