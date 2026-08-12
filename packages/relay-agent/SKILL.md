# Relay Agent

`relay-agent` is the native Rust MCP coding server used by AI Code for controlled local or remote tool execution. The current implementation lives in [`../rust-tools/src/relay_agent/`](../rust-tools/src/relay_agent/) with the binary entrypoint at [`../rust-tools/src/bin/relay-agent.rs`](../rust-tools/src/bin/relay-agent.rs).

This document describes the **current Rust implementation**. The old Node/WebSocket relay, pairing-token flow, `bin/cli.mjs`, and unrestricted "no directory jail" behavior are historical and must not be reintroduced.

## Current contract

- **Protocol:** MCP `2026-07-28` over Streamable HTTP (`POST /mcp`).
- **Platform:** Linux only for the relay binary; sandboxed execution requires Bubblewrap (`bwrap`).
- **Privilege:** refuses to start as UID 0/root.
- **Modes:** `local` (loopback) and `remote` (OAuth-protected resource server).
- **Filesystem boundary:** execution is confined to an explicit `execution_root` (defaults to the resolved `--dir`) and enforced through the relay policy plus Bubblewrap.
- **Tools:** sandboxed terminal execution, HTTP fetch, and SearXNG-backed web search.
- **Docker:** intentionally unsupported/deferred until an isolated Docker backend/broker exists; never expose the host Docker socket to make it work.

The security boundary is server-side authorization plus the Plan 028 sandbox. Client confirmation UI, MCP annotations, or tool descriptions are not security controls.

## Build

From the repository root:

```bash
pnpm build:tools
```

Or directly:

```bash
cargo build --manifest-path packages/rust-tools/Cargo.toml --release --locked --bin relay-agent
```

The repository pins Rust 1.95.0. CI/release policy is documented in [`../rust-tools/README.md`](../rust-tools/README.md).

## Local mode

Local mode is the default and binds to loopback. Supply the project directory and browser/Nuxt origin explicitly:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin relay-agent -- \
  --mode local \
  --dir /home/user/project \
  --execution-root /home/user/project \
  --origin http://localhost:3333
```

Important:

- `--dir` is the default working directory.
- `--execution-root` is the filesystem containment root. When omitted it resolves from `--dir`; prefer setting it explicitly in operator-facing examples.
- The execution root must resolve to an allowed user-owned path; unsafe/shallow system roots are rejected.
- Bubblewrap must be installed before startup.
- The process must run as an unprivileged user.
- Wildcard origins are rejected.

Default port: `47821`.

Stop a port-scoped relay instance with:

```bash
relay-agent stop --port 47821
```

## Remote mode

Remote mode is an OAuth Resource Server and must fail closed. At minimum it requires the configured issuer, audience/resource, and owner subject; the issuer must be a canonical HTTPS URI.

Representative invocation:

```bash
relay-agent \
  --mode remote \
  --dir /home/relay/workspace \
  --execution-root /home/relay/workspace \
  --origin https://app.example.com \
  --oauth-issuer https://issuer.example.com/ \
  --oauth-audience https://relay.example.com/mcp \
  --oauth-owner-subject '<stable-subject>'
```

Do not weaken remote mode by:

- falling back to local/no-auth behavior;
- trusting forwarded headers from arbitrary peers;
- accepting insecure issuer URLs in production;
- moving OAuth scopes/permissions into user-controlled tool arguments;
- exposing the host Docker socket or privileged container controls.

Trusted proxy behavior is explicit. If `--trusted-proxy` is enabled, configure the allowed peer/CIDR as required by the current relay config rather than treating all forwarded headers as trusted.

## Verification

The repository's authoritative verification is CI plus deterministic scripts, not an old JavaScript parity harness:

```bash
cd packages/rust-tools
cargo fmt --all -- --check
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo audit
cd ../..
bash scripts/phase4-black-box.sh
bash scripts/phase7-chatgpt-contract.sh
bash scripts/phase8-zero-bypass.sh
```

Live ChatGPT/OAuth acceptance remains a separate operator gate; repository/static checks must not be described as proof that live OAuth/ChatGPT acceptance passed.

## Durable design context

Read these before changing the relay security model:

- [Plan 028](../../.agents/plans/028-relay-agent-rust-rewrite.md) — Rust rewrite and sandbox boundary.
- [Plan 029](../../.agents/plans/029-chatgpt-native-mcp-integration.md) — ChatGPT MCP integration delta.
- [Plan 029b](../../.agents/plans/029b-chatgpt-mcp-production-hardening.md) — remaining live acceptance/deferred Docker work.
- [Plan 028 security decisions](../../.agents/memories/028-relay-agent-phase19-security-decisions.md).
- [Docker capability blocker](../../.agents/memories/029b-docker-capability-blocker.md).
