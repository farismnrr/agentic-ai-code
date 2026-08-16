# AI Code

AI Code is a self-hosted coding-assistant workspace built around a Nuxt web application and a native Rust execution relay.

It gives an authenticated user one place to:

- chat with configurable AI model providers;
- organize conversations by workspace;
- connect MCP servers and tools;
- execute coding tasks through a sandboxed native relay;
- use the same OAuth-protected MCP relay from the web app, external MCP client, or another compatible MCP client;
- observe application and relay behavior through OpenTelemetry-compatible telemetry.

The project is deliberately split into two trust zones: the **web application** owns chat, users, persistence, providers, and MCP orchestration, while **`ai-tools relay`** owns native command execution and its Linux/Bubblewrap security boundary.

## Start here

The human/operator documentation lives in [`docs/`](docs/README.md):

- [Getting started](docs/getting-started.md) — clone, prerequisites, database, environment, build, and first run
- [Architecture](docs/architecture.md) — how the Nuxt app, MCP layer, Rust relay, OAuth, and tunnel fit together
- [Configuration](docs/configuration.md) — runtime environment and security-sensitive settings
- [Authentication](docs/authentication.md) — AI Code login versus remote MCP OAuth
- [Keycloak](docs/keycloak.md) — configure an external Authorization Server for the MCP relay
- [Remote MCP deployment](docs/remote-mcp.md) — run the relay safely and expose it through an outbound HTTPS tunnel
- [Connect external MCP client](docs/external-mcp.md) — connect the public MCP endpoint and verify tool access
- [Development](docs/development.md) — repository workflow and validation
- [Releases](docs/releases.md) — build and publish web + CLI releases from `main`
- [Troubleshooting](docs/troubleshooting.md) — common setup, OAuth, relay, and Docker issues

Agent-specific repository guidance is separate under [`.agents/`](.agents/README.md). It contains architecture contracts, plans, durable memory, and implementation rules for coding agents; it is not the primary operator handbook.

## What ships

### Web application

Nuxt 4 / Vue provides authenticated chat, workspaces, provider/model settings, MCP server management, persistence, and telemetry. PostgreSQL + Drizzle back the application data.

### Native CLI and relay

The Rust workspace builds one `ai-tools` binary. Its relay exposes MCP `2026-07-28` over Streamable HTTP and provides:

- workspace: `directory_list`, `file_search`, `text_search`, `file_read`, `file_edit`, `file_write`;
- execution: `terminal_exec`, `terminal_job_start`, `terminal_job_get`, `terminal_job_cancel`;
- web: `http_fetch`, `web_search`.

The production relay is Linux-only, refuses to run as root, and uses Bubblewrap for filesystem/process containment. For the single-owner coding profile, the execution root can be the owner's home directory so the same MCP connection can move between sibling repositories without exposing the rest of the host filesystem.

## Repository layout

```text
app/                    Nuxt UI and browser-side application
server/                 Nitro APIs, application layer, infrastructure, auth, MCP, telemetry
shared/                 Shared types and schemas
packages/rust-tools/    Rust workspace and unified ai-tools binary
packages/*-tool/        TypeScript integration/package surfaces
packages/relay-agent/   Relay integration guidance
scripts/                Verification, deployment helpers, and release tooling
docs/                   Human/operator documentation
.agents/                Agent-only knowledge, plans, memory, contracts, and evidence
ai-self/                Persistent MCP-assisted repository operating skills/policies
```

## Project policy at a glance

- package manager: **pnpm 11.18.0**
- development Rust toolchain: **Rust 1.95.0**
- normal Nuxt development port: **3333**
- integration branch: **`dev`**
- release branch: **`main`**
- no GitHub Actions CI workflow and no unit-test suite by project policy
- every normal local commit must pass `pnpm verify:commit`

For installation and the first runnable setup, continue with **[docs/getting-started.md](docs/getting-started.md)**.
