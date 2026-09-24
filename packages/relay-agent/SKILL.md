# Relay Agent

The relay is the `ai-tools relay` subcommand of the unified native Rust binary used by AI Code for controlled local or remote tool execution. The current implementation lives in [`../rust-tools/src/commands/relay.rs`](../rust-tools/src/commands/relay.rs) with the unified binary entrypoint at [`../rust-tools/src/main.rs`](../rust-tools/src/main.rs). `packages/relay-agent/` is integration guidance/package metadata, not a separate executable binary.

This document describes the **current Rust implementation**. The old Node/WebSocket relay, pairing-token flow, `bin/cli.mjs`, and unrestricted no-jail behavior are historical and must not be reintroduced.

## Current contract

- **Protocol:** MCP `2026-07-28` over Streamable HTTP (`POST /mcp`).
- **Platform:** Linux only for the relay binary; sandboxed execution requires Bubblewrap (`bwrap`).
- **Privilege:** refuses to start as UID 0/root.
- **Modes:** `local` (loopback) and `remote` (OAuth-protected resource server).
- **Listener binding:** `--bind-host` / `RELAY_AGENT_BIND_HOST` defaults to
  `127.0.0.1`. Local mode remains loopback-only; remote non-loopback binds require
  an explicit browser Origin and OAuth configuration. `0.0.0.0` is never a client URL.
- **Filesystem boundary:** execution is confined through relay policy plus Bubblewrap. `RELAY_WORKSPACE_ROOT` (or `--workspace-root`, with `--dir` as a compatibility alias) is the single root setting and defaults to `$HOME/Documents/Projects`; it defines the primary workspace and, unless explicitly overridden by `--execution-root`, the hard ceiling. Child repositories beneath it can be selected with `cwd`. Bubblewrap mounts system runtime paths (`/usr`, `/lib`, `/etc`, `/bin`, `/sbin`) read-only, isolates `/tmp` on tmpfs, keeps `/proc` and `/dev` minimal, clears child environment variables except standard runtime keys, and masks protected credential directories/files plus Unix domain sockets throughout the indexed user tree. Canonical dependency/generated roots such as `node_modules`, `target`, and recognized cache/build/output directories are intentionally pruned from protected-path indexing for bounded performance; their contents are not recursively credential-masked, so they must not be used as secret storage. Network namespace is unshared (`--unshare-net`) by default; `RELAY_ALLOW_TERMINAL_NETWORK=true` explicitly permits outbound/loopback terminal network.
- **Tools:** one runtime catalog builder composes a 51-tool retained Full base or 14-tool retained Primary core with explicitly enabled optional capabilities. The retained Full base includes local sandboxed execution (`terminal_exec`), configured network tools, Full-only read-only remote diagnostics (`ssh_readonly_exec`), bounded native workspace tools, remote Git transport, forge/issues/workflows, alerts, and Telegram integration. Plan 069 keeps `creative_status` discoverable and composes the remaining creative tools into Full only when `RELAY_ENABLE_CREATIVE=true`. Discovery and invocation are strictly aligned: tools advertised in `tools/list` match `tools/call` routing; invoking a capability-disabled tool returns structured revocation errors (`CAPABILITY_REVOKED`), and unknown tools return 404 errors. Local Git wrappers and LSP wrappers are not public catalog entries; use terminal for builds, tests, package managers, interpreters, scripts, and uncovered CLI work. Standard developer CLI tools are fully permitted inside the sandbox, while privilege escalation brokers (`sudo`, `su`, `doas`, `pkexec`, `runas`) and generic SSH clients are blocked and masked. Numbered catalog contracts under `.agents/contracts/` are immutable historical audit artifacts, not alternate active runtime versions.
- **Workspace bootstrap:** the retained `workspace_bootstrap` tool is available in Primary and Full and is intended for explicit `/init` / governance-initialization requests only. `action=inspect` is read-only and resolves the verified Git root, reports governance state, detects Node/Rust/Python/Go markers, package-manager state, and stack-native validation commands. `action=reconcile` re-detects the current stack, creates missing portable `ai-self/` + `.agents/` governance, and refreshes only files carrying the Masih Awam managed marker; unowned project guidance/configuration and durable canonical memory are preserved. This lets a later `/init` add newly introduced backend/frontend checks or remove stale generated checks when the stack changes. Reconcile installs no dependencies and must never run implicitly during ordinary or read-only repository work.
- **Bootstrap and resources:** `server/discover.instructions` (and the legacy-compatible `initialize.instructions`) advertise one bounded workspace/reporting bootstrap on every relay connection. The approved `workspace://<repo>/agent-guidance` resource includes `ai-self/BOOTSTRAP.md` when present, then repository `AGENTS.md` and `.agents/knowledge/resources.md`. The first-party Nuxt MCP path explicitly promotes discovered instructions into top-level chat and delegated-subagent system context only when provenance is `first-party-relay`; external/third-party MCP server instructions are never promoted by this mechanism. Arbitrary external MCP hosts may ignore server instructions/resources, so behavioral policy for ChatGPT is carried by the installed Masih Awam Workspace Workflow Skill rather than encoded into factual MCP tool descriptions. Other bounded read-only resources expose repository manifest, Git status, and HEAD metadata; there are no arbitrary resource templates/subscriptions/file-browsing resources.
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
- Use the most specific authorized project or repository `cwd` available for terminal work. A broad authorization root such as a multi-project `Projects` directory may legitimately spend the bounded execution budget on fail-closed protected-path preparation; narrowing `cwd` avoids unnecessary cross-tree indexing without weakening authorization.
- Agents must not start terminal work that is reasonably expected to exceed 60 seconds. For builds, tests, package-manager work, or other long operations, return the exact shell-compatible foreground operator command instead.
- The same rule applies to Creative production. Public MCP owns bounded project/Element/Asset/job/graph state, discovery, admission, cancellation, and bounded Blender inspection. Heavy Creative/Blender execution that can legitimately exceed 60 seconds is not a public MCP action; use the foreground `ai-tools creative --tool ... --input ...` operator path instead. In particular, public Creative job `wait`, graph execute/rerun, arbitrary Blender Python, render/bulk preview, import/export, and checkpoint execution are operator-only.
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

This repository intentionally has **no CI workflow**; verification is repository-local and includes real Rust/web test suites. Use the tracked lifecycle rather than historical plan-numbered scripts:

```bash
# normal checkpoint commit gate
pnpm guardrail:fast

# closure gate
pnpm guardrail:full
```

The guardrails run repository/agent/architecture/test-layout checks and the applicable stack gates. Rust full verification includes formatting, warnings-denied Clippy/check, and Cargo tests under `packages/rust-tools/tests/`. For a focused relay/security change, run the smallest directly relevant Cargo integration test while iterating, then the applicable full guardrail before closure. Dependency/security-sensitive changes additionally require the relevant audit (`cargo audit` and/or `pnpm audit`) when dependency changes justify it.

`scripts/` is reserved for current repository guardrails and hook installation; do not resurrect removed `phase-*`, `verify-*`, or other plan-numbered validation scripts from historical plans/contracts. Live external-client/OAuth behavior must still be verified separately when a task depends on it; repository/static checks are not proof of a live external integration.

## Durable design context

Before changing the relay security model, read:

- the canonical [relay/MCP memory](../../.agents/memories/README.md#rustnative-tool-invariants) for current durable invariants;
- [Plan 030 historical summary](../../.agents/plans/030-previous-plans-summary.md) for compacted Plan 026/027/028/029/029b history;
- current Rust source/config and deterministic contract/security scripts.

All plans through 029b were explicitly closed for a planning refresh. Current 031+ plan status and the next unused numeric plan are recorded in the canonical memory; re-audit current behavior and use the next unused number rather than reopening an old file.

## MCP tool profiles (Plan 045)

The relay supports `RELAY_TOOL_PROFILE=full|primary` (or `--tool-profile`). `full` is the default and canonical superset; `primary` is the smaller public routing/UX fast path and does not change the underlying authorization or filesystem boundaries. The repository remote launcher pins Primary.

Primary has a 14-tool retained terminal/workspace core. Full has a 51-tool retained base. The current runtime catalog is the selected base plus explicitly enabled optional capabilities; numbered snapshots are immutable history only. Dedicated SSH diagnostics are Full-only and relay-owned: clients may select a final alias plus a bounded ordered `via` alias chain, but no client parses SSH config or receives raw SSH options/key paths. The relay resolves contained Include files, alias-only ProxyJump, per-hop identity/known-host material, and transport; diagnostic execution occurs only on the final alias. Agents receive only the selected active runtime schemas and should prefer a supplied dedicated tool before terminal fallback.

A simultaneous public Full + Primary deployment is a separate operator decision because separate endpoints may require reviewed OAuth/resource configuration. Where a client can hide actions client-side, that can be used for A/B testing without a second endpoint.
