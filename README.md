# AI Code

AI Code is a workspace-scoped AI coding application built with Nuxt 4. It combines an authenticated web chat, configurable model providers, MCP integrations, project workspaces, and native Rust execution tools behind a local/remote relay boundary.

This repository is no longer the stock Nuxt UI starter. The authoritative agent-facing project notes live in [`.agents/`](.agents/README.md).

## Stack

- **Web:** Nuxt 4, Vue, Nuxt UI 4, Tailwind CSS 4
- **AI:** AI SDK 7, LangChain, Anthropic/OpenAI-compatible/Vertex providers
- **Data/Auth:** PostgreSQL, Drizzle ORM, `nuxt-auth-utils`
- **MCP:** Model Context Protocol SDK plus the native `relay-agent`
- **Native tools:** Rust binaries for terminal execution, HTTP requests, SearXNG search, and the relay agent
- **Package manager:** pnpm

See [`package.json`](package.json), [`packages/rust-tools/Cargo.toml`](packages/rust-tools/Cargo.toml), and [`.agents/knowledge/project.md`](.agents/knowledge/project.md) for the current implementation/toolchain details.

## Repository layout

```text
app/                    Nuxt UI, pages, components, composables, plugins
server/                 Nitro API routes, auth, chat, MCP, persistence, telemetry
shared/                 Types and schemas shared across client/server
packages/
  terminal-tool/        AI SDK-facing terminal tool package
  curl-tool/            AI SDK-facing HTTP tool package
  searxng-search-tool/  AI SDK-facing search tool package
  relay-agent/          Relay package/release surface
  rust-tools/           Native implementations for all four binaries
scripts/                Deterministic MCP/security/release acceptance scripts
.agents/                Agent knowledge, skills, plans, memories, and hook docs
```

## Setup

Requirements:

- Node.js 22+
- pnpm 11 (the exact version is pinned by `packageManager`)
- Rust 1.95.0 for native tool builds
- PostgreSQL for application persistence
- Linux + Bubblewrap for the production `relay-agent` sandbox/release target

Install dependencies and prepare generated Nuxt/native artifacts:

```bash
pnpm install
```

Copy `.env.example` to `.env` and fill the values required by the feature you are running. Runtime configuration is documented in [`.agents/knowledge/tooling.md`](.agents/knowledge/tooling.md); `.env.example` is the source of truth for available environment keys.

## Common commands

| Task | Command |
| --- | --- |
| Production build | `pnpm build` |
| Build native Rust tools | `pnpm build:tools` |
| Preview production build | `pnpm preview` |
| Dev server | `pnpm dev` |
| Lint | `pnpm lint` |
| Type check | `pnpm typecheck` |
| Dependency audit | `pnpm audit` |
| Generate DB migration | `pnpm db:generate` |
| Apply DB migrations | `pnpm db:migrate` |

For local verification, prefer a clean `pnpm build` followed by `pnpm preview`; the repo has known dev-watcher and typecheck caveats documented under [`.agents/memories/`](.agents/memories/README.md).

## Native tool verification

The Rust workspace is under `packages/rust-tools/` and pins Rust 1.95.0. CI currently enforces:

```bash
cd packages/rust-tools
cargo fmt --all -- --check
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo audit
```

The release contract for `relay-agent` is Linux + Bubblewrap; do not infer macOS/Windows relay support from the sibling CLI binaries.

## Agent workflow

Agent guidance is centralized so Claude Code, Gemini/Antigravity, and other agents do not maintain separate copies:

- [`AGENTS.md`](AGENTS.md) — generic entrypoint
- [`CLAUDE.md`](CLAUDE.md) — Claude Code entrypoint
- [`GEMINI.md`](GEMINI.md) — Gemini/Antigravity entrypoint
- [`.agents/README.md`](.agents/README.md) — authoritative index and closeout rules

Implementation plans are stored in [`.agents/plans/`](.agents/plans/README.md); durable decisions and traps are indexed in [`.agents/memories/`](.agents/memories/README.md).

## Branching

Do not commit directly to `main` or `dev`. Work branches target `dev`; promotion from `dev` to `main` only happens when explicitly requested. See [`.agents/knowledge/git.md`](.agents/knowledge/git.md).
