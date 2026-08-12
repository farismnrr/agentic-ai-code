# AI Code

AI Code is a workspace-scoped AI coding application built with Nuxt 4. It combines authenticated web chat, configurable model providers, MCP integrations, project workspaces, and native Rust execution tools behind a local/remote relay boundary.

The authoritative agent-facing project notes live in [`.agents/`](.agents/README.md).

## Stack

- **Web:** Nuxt 4, Vue, Nuxt UI 4, Tailwind CSS 4
- **AI:** AI SDK 7, LangChain, Anthropic/OpenAI-compatible/Vertex providers
- **Data/Auth:** PostgreSQL, Drizzle ORM, `nuxt-auth-utils`
- **MCP:** Model Context Protocol SDK plus the native `relay-agent`
- **Native tools:** Rust binaries for terminal execution, HTTP requests, SearXNG search, and the relay agent
- **Package manager:** pnpm

See [`package.json`](package.json), [`packages/rust-tools/Cargo.toml`](packages/rust-tools/Cargo.toml), and [`.agents/knowledge/project.md`](.agents/knowledge/project.md) for current implementation/toolchain details.

## Repository layout

```text
app/                    Nuxt UI, pages, components, composables, plugins
server/                 Nitro API routes, auth, chat, MCP, persistence, telemetry
shared/                 Types and schemas shared across client/server
packages/
  terminal-tool/        AI SDK-facing terminal tool package
  curl-tool/            AI SDK-facing HTTP tool package
  searxng-search-tool/  AI SDK-facing search tool package
  relay-agent/          Relay package surface
  rust-tools/           Native implementations for all four binaries
scripts/                Local quality/hook helpers plus deterministic acceptance scripts
.githooks/              Mandatory tracked local Git hooks
.agents/                Agent knowledge, skills, plans, memories, and contracts
```

This repository intentionally has **no CI workflow** and **no unit-test suite**.

## Setup

Requirements:

- Node.js 22+
- pnpm 11 (exact version pinned by `packageManager`)
- Rust 1.95.0 for native tool builds
- PostgreSQL for application persistence
- Linux + Bubblewrap for the production `relay-agent` sandbox boundary

Install dependencies and prepare generated Nuxt/native artifacts:

```bash
pnpm install
```

`pnpm install` also activates the tracked pre-commit hook via `core.hooksPath=.githooks`.

Copy `.env.example` to `.env` and fill the values required by the feature you are running. Runtime configuration is documented in [`.agents/knowledge/tooling.md`](.agents/knowledge/tooling.md).

## Mandatory commit gate

Every commit must pass:

```bash
pnpm verify:commit
```

The pre-commit hook runs this automatically. Never bypass it with `git commit --no-verify`.

The gate includes:

- agent-doc/index integrity;
- all configured JS/Vue and Rust lint checks;
- all configured Nuxt/Vue and Rust type/compile checks.

A failing gate blocks the commit until fixed. There is no remote CI safety net.

## Common commands

| Task | Command |
| --- | --- |
| Mandatory commit gate | `pnpm verify:commit` |
| Production build | `pnpm build` |
| Build native Rust tools | `pnpm build:tools` |
| Preview production build | `pnpm preview` |
| Dev server | `pnpm dev` |
| All linters | `pnpm lint` |
| All type/compile checks | `pnpm typecheck` |
| Dependency audit | `pnpm audit` |
| Generate DB migration | `pnpm db:generate` |
| Apply DB migrations | `pnpm db:migrate` |

For local runtime verification, prefer a clean `pnpm build` followed by `pnpm preview` when relevant.

## Verification policy

There is no CI and no unit-test suite. Quality is enforced locally before each commit.

For dependency changes, also run `pnpm audit` before merge. For security-sensitive Rust/MCP changes, run the relevant deterministic scripts under `scripts/` and `cargo audit` when applicable.

Existing deterministic acceptance/security scripts are targeted local verification tools; they are not unit tests and are not CI.

## Agent workflow

Agent guidance is centralized and vendor-neutral:

- [`AGENTS.md`](AGENTS.md) — the only repository agent entrypoint
- [`.agents/README.md`](.agents/README.md) — authoritative guidance index and closeout rules

Implementation plans are stored in [`.agents/plans/`](.agents/plans/README.md); durable decisions and traps are indexed in [`.agents/memories/`](.agents/memories/README.md).

## Branching

Do not commit directly to `main` or `dev`. Work branches target `dev`; promotion from `dev` to `main` only happens when explicitly requested. See [`.agents/knowledge/git.md`](.agents/knowledge/git.md).
