# Relay Agent

`relay-agent` is the native Rust MCP coding server used by AI Code for controlled local or remote tool execution. The current implementation lives in [`../rust-tools/cli/src/commands/relay.rs`](../rust-tools/cli/src/commands/relay.rs) with the unified binary entrypoint at [`../rust-tools/cli/src/main.rs`](../rust-tools/cli/src/main.rs).

This document describes the **current Rust implementation**. The old Node/WebSocket relay, pairing-token flow, `bin/cli.mjs`, and unrestricted no-jail behavior are historical and must not be reintroduced.

## Current contract

- **Protocol:** MCP `2026-07-28` over Streamable HTTP (`POST /mcp`).
- **Platform:** Linux only for the relay binary; sandboxed execution requires Bubblewrap (`bwrap`).
- **Privilege:** refuses to start as UID 0/root.
- **Modes:** `local` (loopback) and `remote` (OAuth-protected resource server).
- **Filesystem boundary:** execution is confined to an explicit `execution_root` and enforced through relay policy plus Bubblewrap. The single-user laptop profile uses the canonical non-root owner home as the root; `--dir` remains an independent starting `cwd`.
- **Tools:** 25 total — sandboxed execution (`terminal_exec`, `terminal_job_start`, `terminal_job_get`, `terminal_job_cancel`), configured network tools (`http_fetch`, `web_search`), bounded native workspace tools (`directory_list`, `file_search`, `text_search`, `file_read`, `file_edit`, `file_write`, `apply_patch`), read-only Git tools (`git_status`, `git_diff`, `git_log`, `git_show`, `git_blame`), and bounded LSP-backed code tools (`code_symbols`, `code_definition`, `code_references`, `code_implementations`, `code_hover`, `code_diagnostics`, `code_rename_preview`).
- **Resources:** bounded read-only repository manifest, approved agent guidance, Git status, and HEAD metadata via server-owned `workspace://` URIs; no arbitrary resource templates/subscriptions/file browsing.
- **Docker:** denied by default. Trusted single-owner local development may explicitly opt in with `--allow-docker` / `RELAY_ALLOW_DOCKER=true`, which exposes only the configured Docker socket; treat that socket as effectively host-level authority and keep it disabled for remote/production deployments unless the operator deliberately accepts that expansion.

The security boundary is server-side authorization plus the Bubblewrap sandbox. Client confirmation UI, MCP annotations, or tool descriptions are not security controls.

### Long-running terminal contract

- `terminal_exec` declares optional MCP Tasks support. Tasks-capable clients use the standard `io.modelcontextprotocol/tasks` lifecycle; tool-level non-zero exits complete the task with `isError: true` rather than becoming JSON-RPC failures.
- First-party/non-Tasks clients that need live output use `terminal_job_start/get/cancel`; polling returns bounded current stdout/stderr tails without introducing a second process runner.
- `timeout_ms: 0` means no command deadline unless `RELAY_MAX_TERMINAL_TIMEOUT_MS` sets an operator cap. There is no unconditional five-minute terminal ceiling.
- Running pipes are drained continuously. Output retention is bounded and reports omitted earlier bytes rather than killing noisy commands solely for exceeding the retained log window.
- Manual cancel, timeout, and relay shutdown terminate/reap the sandbox process tree through the same authoritative job manager.

## Build

From repository root:

```bash
pnpm build:tools
```

Or directly:

```bash
cargo build --manifest-path packages/rust-tools/cli/Cargo.toml --release --locked --bin ai-tools
```

The repository pins Rust 1.95.0. Current local verification/release policy is documented in [`../rust-tools/README.md`](../rust-tools/README.md).

## Local mode

Local mode is the default and binds to loopback. Supply the project directory and browser/Nuxt origin explicitly:

```bash
cargo run --manifest-path packages/rust-tools/cli/Cargo.toml --bin ai-tools -- relay \
  --mode local \
  --dir /home/user/project \
  --execution-root /home/user \
  --origin http://localhost:3333
```

Important:

- `--dir` is the default working directory.
- `--execution-root` is the filesystem containment root. For a single-owner coding relay, set it to `/home/user`; sibling projects can then be selected with `cwd` without restarting or reconnecting.
- The execution root must resolve to an allowed user-owned path; unsafe/shallow system roots are rejected.
- Bubblewrap must be installed before startup.
- The process must run as an unprivileged user.
- Wildcard origins are rejected.
- User-managed toolchains are opt-in through repeated `--toolchain-path` flags (or `RELAY_TOOLCHAIN_PATH`); the relay never inherits an arbitrary parent `PATH`.
- LSP executables are operator-approved through repeated `--lsp-server language=executable` (or `RELAY_LSP_SERVER`) entries. The executable is resolved only from the relay safe PATH/toolchain directories; repository files cannot replace it or provide command arguments. The public `code_*` MCP tools use that bounded substrate; unsupported server capabilities fail distinctly rather than being fabricated.
- The owner-home Bubblewrap namespace masks common credential stores (`.ssh`, cloud credentials, Docker/Kubernetes credentials, and common token files). Review the exact deployment policy before relying on a command that needs one of them.

Default port: `47821`.

Stop a port-scoped relay instance with:

```bash
ai-tools relay stop --port 47821
```

## Remote mode

Remote mode is an OAuth Resource Server and must fail closed. At minimum it requires the configured issuer, audience/resource, and owner subject; the issuer must be a canonical HTTPS URI.

Representative invocation:

```bash
ai-tools relay \
  --mode remote \
  --dir /home/relay/workspace \
  --execution-root /home/relay/workspace \
  --origin https://app.example.com \
  --oauth-issuer https://issuer.example.com/ \
  --oauth-audience https://relay.example.com/mcp \
  --oauth-owner-subject '<stable-subject>'
```

Do not weaken remote mode by falling back to local/no-auth behavior, trusting forwarded headers from arbitrary peers, accepting insecure production issuer URLs, moving permissions into tool arguments, or enabling Docker/Tailscale host-socket authority implicitly.

Trusted proxy behavior is explicit. If `--trusted-proxy` is enabled, configure the allowed peer/CIDR required by current relay config rather than treating all forwarded headers as trusted.

## Verification

This repository intentionally has **no CI workflow and no unit-test suite**. The mandatory local commit gate is the baseline:

```bash
pnpm verify:commit
```

For security-sensitive relay/MCP changes, also run applicable local checks, typically including:

```bash
cargo audit
bash scripts/phase4-black-box.sh
bash scripts/phase7-chatgpt-contract.sh
bash scripts/phase-039c-contract.sh
bash scripts/phase8-zero-bypass.sh
```


For the Plan 039C protocol/session foundation, also run:

```bash
bash scripts/verify-lsp-foundation.sh
```

This deterministic fixture exercises framing, correlation, lifecycle, capability capture, process/sandbox isolation, bounded errors/output, and sibling-workspace isolation without depending on a real language server.

The tracked pre-commit gate already covers Rust formatting, warnings-denied Clippy, and warnings-denied `cargo check` through root lint/typecheck. The deterministic scripts above are targeted security/protocol checks, not a unit-test suite.

Live external ChatGPT/OAuth behavior must be verified separately when a future task depends on it; repository/static checks are not proof of a live external integration.

## Durable design context

Before changing the relay security model, read:

- the canonical [relay/MCP memory](../../.agents/memories/README.md#relay-agent-and-mcp-security-invariants) for current durable invariants;
- [Plan 030 historical summary](../../.agents/plans/030-previous-plans-summary.md) for compacted Plan 026/027/028/029/029b history;
- current Rust source/config and deterministic contract/security scripts.

All plans through 029b were explicitly closed for a planning refresh. Current 031+ plan status and the next unused numeric plan are recorded in the canonical memory; re-audit current behavior and use the next unused number rather than reopening an old file.
