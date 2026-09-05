# AI Code

AI Code is a self-hosted coding-assistant workspace built around a Nuxt web application and a native Rust execution relay.

It gives an authenticated user one place to:

- chat with configurable AI model providers;
- organize conversations by workspace;
- connect MCP servers and tools;
- execute coding tasks through a sandboxed native relay;
- use the same OAuth-protected MCP relay from the web app or any compatible MCP client;
- review durable per-workspace activity history and exact supported mutation diffs;
- observe application and relay behavior through OpenTelemetry-compatible telemetry.

The project is deliberately split into two trust zones: the **web application** owns chat, users, persistence, providers, and MCP orchestration, while **`ai-tools relay`** owns native command execution and its Linux/Bubblewrap security boundary.

## Start here

The human/operator documentation lives in [`docs/`](docs/README.md):

- [Getting started](docs/getting-started.md) — clone, prerequisites, database, environment, build, and first run
- [Architecture](docs/architecture.md) — how the Nuxt app, MCP layer, Rust relay, OAuth, and tunnel fit together
- [Configuration](docs/configuration.md) — runtime environment and security-sensitive settings
- [Authentication](docs/authentication.md) — AI Code login versus remote MCP OAuth
- [OAuth/OIDC provider](docs/oauth-provider.md) — configure an external Authorization Server for the MCP relay
- [Remote MCP deployment](docs/remote-mcp.md) — run the relay safely and expose it through an outbound HTTPS tunnel
- [Connect an MCP client](docs/mcp-client.md) — connect the public MCP endpoint and verify tool access
- [Development](docs/development.md) — repository workflow and validation
- [Releases](docs/releases.md) — build and publish web + CLI releases from `main`
- [Troubleshooting](docs/troubleshooting.md) — common setup, OAuth, relay, and Docker issues

Agent-specific repository guidance is separate under [`.agents/`](.agents/README.md). It contains architecture contracts, plans, durable memory, and implementation rules for coding agents; it is not the primary operator handbook.

## What ships

### Web application

Nuxt 4 / Vue provides authenticated chat, workspaces, provider/model settings, MCP server management, persistence, and telemetry. PostgreSQL + Drizzle back the application data.

### Native CLI and relay

The Rust workspace builds one `ai-tools` binary. Its relay exposes MCP `2026-07-28` over Streamable HTTP and provides:

- workspace: `directory_list`, `file_search`, `text_search`, `file_read`, `file_edit`, `file_write`, `apply_patch`;
- Git inspection and bounded local mutation: `git_status`, `git_diff`, `git_log`, `git_show`, `git_blame`, branch/stage/commit/merge/rebase/conflict primitives;
- remote Git and forge delivery: validated remote discovery/fetch/push/branch cleanup plus forge-neutral change-request list/get/create/update/checks/merge backed by a reviewed forge adapter;
- LSP code intelligence: `code_symbols`, `code_definition`, `code_references`, `code_implementations`, `code_hover`, `code_diagnostics`, `code_rename_preview`;
- execution: `terminal_exec`, `terminal_job_start`, `terminal_job_get`, `terminal_job_cancel`;
- web: `http_fetch`, `web_search`.

The relay also exposes bounded read-only repository resources for manifest, approved agent guidance, Git status, and HEAD metadata. The first-party agent UI renders tool calls by capability category, keeps approval inputs bounded/sensitivity-aware, and surfaces task/context/subagent/background/orchestration state without exposing hidden reasoning. Agent mode can define bounded dependency graphs, dispatch independent child work through the existing subagent/background runtime, require writer worktrees, reconcile child evidence, and gate delivery until reviewed writer work is integrated and high-severity/conflicting findings are cleared.

When enabled, the relay's Plan 050 activity recorder durably captures bounded
workspace lifecycles in an encrypted owner-local outbox before execution;
Nuxt/PostgreSQL provides the owned Logs read model. Activity is separate from
OpenTelemetry/Loki, and opaque process/Git/delegated work is never presented as
an exact source diff without relay-owned proof. See
[configuration](docs/configuration.md#workspace-activity-ledger).

The production relay is Linux-only, refuses to run as root, and uses Bubblewrap for filesystem/process containment. For the single-owner coding profile, the execution root can be the owner's home directory so the same MCP connection can move between sibling repositories without exposing the rest of the host filesystem.

## Repository layout

```text
app/                    Nuxt UI and browser-side application
server/                 Nitro APIs, application layer, infrastructure, auth, MCP, telemetry
shared/                 Shared types and schemas
packages/rust-tools/    Rust workspace and unified ai-tools binary
packages/*-tool/        TypeScript integration/package surfaces
packages/relay-agent/   Relay integration guidance
scripts/                Repository guardrails and Git hook installation
test/                   Web/Node unit and integration tests
ops/                    Operational, deployment, migration, and release helpers
docs/                   Human/operator documentation
.agents/                Agent-only knowledge, plans, memory, contracts, and evidence
ai-self/                Persistent MCP-assisted repository operating skills/policies
```

## Project policy at a glance

- package manager: **pnpm 11.18.0**
- development Rust toolchain: **Rust 1.95.0**
- normal Nuxt development port: **3333**
- integration/release branch: **`main`**
- implementation work: dedicated feature branch → PR → `main`
- no hosted CI workflow; web tests live under `test/`, Rust tests use Cargo package `tests/`, and production files contain no inline tests
- checkpoint commits use fast `pnpm guardrail:fast`; closure uses `pnpm guardrail:full`, and release preparation uses `pnpm guardrail:release`. The main-only pre-push hook runs the affected full gate against the exact pushed range.
- `scripts/` is guardrails-only; feature tests are named for behavior rather than plan numbers

For installation and the first runnable setup, continue with **[docs/getting-started.md](docs/getting-started.md)**.
