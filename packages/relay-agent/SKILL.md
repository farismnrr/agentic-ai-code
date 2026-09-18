# Relay Agent

`relay-agent` is the native Rust MCP coding server used by AI Code for controlled local or remote tool execution. The current implementation lives in [`../rust-tools/cli/src/commands/relay.rs`](../rust-tools/cli/src/commands/relay.rs) with the unified binary entrypoint at [`../rust-tools/cli/src/main.rs`](../rust-tools/cli/src/main.rs).

This document describes the **current Rust implementation**. The old Node/WebSocket relay, pairing-token flow, `bin/cli.mjs`, and unrestricted no-jail behavior are historical and must not be reintroduced.

## Current contract

- **Protocol:** MCP `2026-07-28` over Streamable HTTP (`POST /mcp`).
- **Platform:** Linux only for the relay binary; sandboxed execution requires Bubblewrap (`bwrap`).
- **Privilege:** refuses to start as UID 0/root.
- **Modes:** `local` (loopback) and `remote` (OAuth-protected resource server).
- **Listener binding:** `--bind-host` / `RELAY_AGENT_BIND_HOST` defaults to
  `127.0.0.1`. Local mode remains loopback-only; remote non-loopback binds require
  an explicit browser Origin and OAuth configuration. `0.0.0.0` is never a client URL.
- **Filesystem boundary:** execution is confined through relay policy plus Bubblewrap. `RELAY_WORKSPACE_ROOT` (or `--workspace-root`, with `--dir` as a compatibility alias) is the single root setting and defaults to `$HOME/Documents/Projects`; it defines the primary workspace and, unless explicitly overridden by `--execution-root`, the hard ceiling. Child repositories beneath it can be selected with `cwd`. Bubblewrap mounts system runtime paths (`/usr`, `/lib`, `/etc`, `/bin`, `/sbin`) read-only, isolates `/tmp` on tmpfs, keeps `/proc` and `/dev` minimal, clears child environment variables except standard runtime keys, and recursively masks protected credential directories (`.ssh`, `.aws`, `.cargo/credentials`, `.env.*`) and Unix domain sockets across all visible depths. Network namespace is unshared (`--unshare-net`) by default; `RELAY_ALLOW_TERMINAL_NETWORK=true` explicitly permits outbound/loopback terminal network.
- **Tools:** one runtime catalog builder composes a 50-tool retained Full base or 13-tool retained Primary core with explicitly enabled optional capabilities. The retained Full base includes local sandboxed execution (`terminal_exec`), configured network tools, Full-only read-only remote diagnostics (`ssh_readonly_exec`), bounded native workspace tools, remote Git transport, forge/issues/workflows, alerts, and Telegram integration. Plan 069 keeps `creative_status` discoverable and composes the remaining creative tools into Full only when `RELAY_ENABLE_CREATIVE=true`. Discovery and invocation are strictly aligned: tools advertised in `tools/list` match `tools/call` routing; invoking a capability-disabled tool returns structured revocation errors (`CAPABILITY_REVOKED`), and unknown tools return 404 errors. Local Git wrappers and LSP wrappers are not public catalog entries; use terminal for builds, tests, package managers, interpreters, scripts, and uncovered CLI work. Standard developer CLI tools are fully permitted inside the sandbox, while privilege escalation brokers (`sudo`, `su`, `doas`, `pkexec`, `runas`) and generic SSH clients are blocked and masked. Numbered catalog contracts under `.agents/contracts/` are immutable historical audit artifacts, not alternate active runtime versions.
- **Resources:** bounded read-only repository manifest, approved agent guidance, Git status, and HEAD metadata via server-owned `workspace://` URIs; no arbitrary resource templates/subscriptions/file browsing.
- **Docker:** arbitrary terminal processes do not receive the Docker socket by default. Direct `docker` calls may use the configured socket only after their arguments pass the bounded read-only diagnostic policy; lifecycle mutations and unknown operations fail closed. `--allow-docker` / `RELAY_ALLOW_DOCKER=true` is a separate full-authority escape hatch for trusted single-owner development and should remain disabled unless the operator deliberately accepts host-level Docker authority.

### Workspace activity ledger (Plan 050)

The relay can record every mediated tool call at the shared MCP execution
boundary. `RELAY_ACTIVITY_MODE=off` is the compatibility default;
`required` durably admits a bounded `started` event in an encrypted,
owner-only SQLite outbox before workspace execution. Configure
`RELAY_ACTIVITY_STATE_DIR`, `RELAY_ACTIVITY_SINK_URL`, and the one-time
enrollment `RELAY_ACTIVITY_SOURCE_TOKEN` to enable authenticated asynchronous
delivery. The source ID and local encryption key persist in the state directory;
unacknowledged records are retained across sink outages/restarts and quota
failure is fail-closed. 401/403 marks delivery degraded rather than hammering a
revoked credential.

The relay derives workspace scope from its canonical `WorkspaceAllowlist` root;
it never trusts a Nuxt workspace UUID from an MCP client. `clientInfo` is
presentation metadata only. Structured text mutations may provide exact
historical evidence; process and Git operations remain bounded
summary/unavailable evidence when exact provenance is not relay-owned. Activity
payloads are not OTel/Loki telemetry and never include raw arguments/results,
prompts, auth, environment variables, or arbitrary stdout/stderr.

The security boundary is server-side authorization plus the Bubblewrap sandbox. Client confirmation UI, MCP annotations, or tool descriptions are not security controls.

### Long-running / slow-operation contract

- `terminal_exec` is synchronous-only. It does not advertise MCP Tasks, does not accept `execution_mode`, and has no public `terminal_job_start/get/cancel` fallback.
- Terminal `timeout_ms` is bounded to `1..=60000` milliseconds with a 30 second default. `RELAY_MAX_TERMINAL_TIMEOUT_MS` may lower the effective ceiling but cannot raise terminal execution above 60 seconds.
- Agents must not start terminal work that is reasonably expected to exceed 60 seconds. For builds, tests, package-manager work, or other long operations, return the exact shell-compatible foreground operator command instead.
- The 60-second terminal ceiling applies to agent-executed relay calls only. Commands handed to the human operator must not be wrapped in `timeout`, backgrounded/detached, or have normal progress output redirected/suppressed merely to bound runtime; operator-run commands may run longer and should remain directly observable.
- If a terminal command reaches its deadline, the relay terminates and reaps the sandbox process tree. Agents must not retry it as background work.
- The internal job manager remains the single lifecycle owner for synchronous process spawn, pipe draining, bounded output retention, timeout, cancellation-on-request-drop, and process cleanup.
- `ssh_readonly_exec`, `http_fetch`, and `web_search` follow the same synchronous-only public execution model; no retained coding tool accepts caller-selected `execution_mode`, and exposed `timeout_ms` values are capped at 60 seconds.

## Build

From repository root:

```bash
pnpm build:tools
```

Or directly:

```bash
cargo build --manifest-path packages/rust-tools/Cargo.toml --release --locked --bin ai-tools
```

The repository pins Rust 1.95.0. Current local verification/release policy is documented in [`../rust-tools/README.md`](../rust-tools/README.md).

## Local mode

Local mode is the default and binds to loopback. The workspace root defaults to `$HOME/Documents/Projects`; set `RELAY_WORKSPACE_ROOT` or pass `--workspace-root` to select another tree. Supply the browser/Nuxt origin explicitly:

```bash
cargo run --manifest-path packages/rust-tools/Cargo.toml --bin ai-tools -- relay \
  --mode local \
  --workspace-root /home/user/Documents/Projects \
  --origin http://localhost:3333
```

Important:

- `RELAY_WORKSPACE_ROOT` / `--workspace-root` sets the primary root; it defaults to `$HOME/Documents/Projects`, and `--dir` remains a compatibility alias. Keep that Projects tree as the user-project root: the `ai-code` checkout is source-only and may be selected as `cwd` for repository development, but must never be used as a Creative/Blender project root or final acceptance-output root. Blender creative projects belong under `$HOME/Documents/Projects/Blender/<creative-project>/...`.
- `--execution-root` is an explicit hard-ceiling override. When omitted, it uses the same root. Child folders under Projects are selectable directly with `cwd`; siblings outside a narrower primary root require `workspace_add` and must remain inside the ceiling.
- The execution root must resolve to an allowed user-owned path; unsafe/shallow system roots are rejected.
- Bubblewrap must be installed before startup.
- The process must run as an unprivileged user.
- Wildcard origins are rejected.
- Common user-managed runtimes are discovered from the owner profile (Cargo/Rust, Node managers, Bun, pnpm/npm prefixes, and bounded Conda environments) after ownership and permission checks. Other runtimes may be added through repeated `--toolchain-path` flags (or `RELAY_TOOLCHAIN_PATH`); the relay never inherits an arbitrary parent `PATH`.
- Local language-server workflows remain terminal fallback operations in the retained base. If a future reviewed capability promotes a semantic tool, its executable must be operator-approved through the existing bounded `--lsp-server language=executable` / `RELAY_LSP_SERVER` configuration and safe PATH checks.
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
  --bind-host 0.0.0.0 \
  --workspace-root /home/relay/workspace \
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
Run the applicable remote-client contract acceptance script under `scripts/`.
bash scripts/phase-039c-contract.sh
bash scripts/phase8-zero-bypass.sh
```


For the Plan 039C protocol/session foundation, also run:

```bash
bash scripts/verify-lsp-foundation.sh
```

This deterministic fixture exercises framing, correlation, lifecycle, capability capture, process/sandbox isolation, bounded errors/output, and sibling-workspace isolation without depending on a real language server.

The tracked pre-commit gate already covers Rust formatting, warnings-denied Clippy, and warnings-denied `cargo check` through root lint/typecheck. The deterministic scripts above are targeted security/protocol checks, not a unit-test suite.

Live external-client/OAuth behavior must be verified separately when a future task depends on it; repository/static checks are not proof of a live external integration.

## Durable design context

Before changing the relay security model, read:

- the canonical [relay/MCP memory](../../.agents/memories/README.md#relay-agent-and-mcp-security-invariants) for current durable invariants;
- [Plan 030 historical summary](../../.agents/plans/030-previous-plans-summary.md) for compacted Plan 026/027/028/029/029b history;
- current Rust source/config and deterministic contract/security scripts.

All plans through 029b were explicitly closed for a planning refresh. Current 031+ plan status and the next unused numeric plan are recorded in the canonical memory; re-audit current behavior and use the next unused number rather than reopening an old file.

## MCP tool profiles (Plan 045)

The relay supports `RELAY_TOOL_PROFILE=full|primary` (or `--tool-profile`). `full` is the default and canonical superset; `primary` is the smaller public routing/UX fast path and does not change the underlying authorization or filesystem boundaries. The repository remote launcher pins Primary.

Primary has a 13-tool retained terminal/workspace core. Full has a 50-tool retained base. The current runtime catalog is the selected base plus explicitly enabled optional capabilities; numbered snapshots are immutable history only. Dedicated SSH diagnostics are Full-only and relay-owned: no client parses SSH config or receives raw SSH options/key paths. Agents receive only the selected active runtime schemas and should prefer a supplied dedicated tool before terminal fallback.

A simultaneous public Full + Primary deployment is a separate operator decision because separate endpoints may require reviewed OAuth/resource configuration. Where a client can hide actions client-side, that can be used for A/B testing without a second endpoint.
