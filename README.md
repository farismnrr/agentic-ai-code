# AI Code

AI Code is a workspace-scoped AI coding application built with Nuxt 4. It combines authenticated web chat, configurable model providers, MCP integrations, project workspaces, and native Rust execution tools behind a local/remote relay boundary.

The authoritative agent-facing project notes live in [`.agents/`](.agents/README.md).

## Stack

- **Web:** Nuxt 4, Vue, Nuxt UI 4, Tailwind CSS 4
- **AI:** AI SDK 7, LangChain, Anthropic/OpenAI-compatible/Vertex providers
- **Data/Auth:** PostgreSQL, Drizzle ORM, `nuxt-auth-utils`
- **MCP:** Model Context Protocol SDK plus the native `ai-tools relay`
- **Native tools:** A single unified Rust binary (`ai-tools`) for terminal execution, HTTP requests, SearXNG search, and the relay agent
- **Package manager:** pnpm

See [`package.json`](package.json), [`Cargo.toml`](Cargo.toml), and [`.agents/knowledge/project.md`](.agents/knowledge/project.md) for current implementation/toolchain details.

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
  rust-tools/           Native unified binary implementation (ai-tools)
scripts/                Local policy/quality/hook helpers plus deterministic acceptance scripts
.githooks/              Mandatory tracked local Git hooks
.agents/                Agent knowledge, skills, plans, canonical memory, and contracts
```

This repository intentionally has **no CI workflow** and **no unit-test suite**.

## Relay execution lifecycle

The native relay exposes the synchronous `terminal_exec`, `http_fetch`, and `web_search` tools plus `terminal_job_start`, `terminal_job_get`, and `terminal_job_cancel` for first-party/non-Tasks polling. `terminal_exec` advertises optional MCP Tasks support: Tasks-capable clients get the standard task lifecycle, while the fallback job tools expose bounded live stdout/stderr for clients such as the local Nuxt terminal.

Terminal deadlines are operator policy, not a hard-coded five-minute ceiling. `timeout_ms: 0` means no command deadline unless `RELAY_MAX_TERMINAL_TIMEOUT_MS` configures an operator maximum. Running output is drained continuously and retained as bounded tails; cancellation, timeout, and relay shutdown terminate the full sandboxed process tree. The owner-home profile uses an explicit `RELAY_TOOLCHAIN_PATH` allowlist and masks common credential stores rather than inheriting the relay process PATH.

## Setup

Requirements:

- Node.js 22+
- pnpm 11 (exact version pinned by `packageManager`)
- Rust 1.95.0 for native tool builds
- PostgreSQL for application persistence
- Linux + Bubblewrap for the production `ai-tools relay` sandbox boundary

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

The gate includes repository policy enforcement, compact agent-doc integrity, all configured JS/Vue and Rust lint checks, and all configured Nuxt/Vue and Rust type/compile checks. A failing gate blocks the commit until fixed. There is no remote CI safety net.

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

## Releases

The repository deliberately has no GitHub Actions release workflow. Releases are promoted manually from a reviewed `main` commit after local verification.

For a stable `vX.Y.Z` release:

1. merge the implementation branch into `dev`, then promote `dev` to `main` through a PR;
2. verify the final `main` commit locally with `pnpm verify:commit` and `pnpm build`;
3. create and push the annotated `vX.Y.Z` tag on that exact commit;
4. build and verify release artifacts with `pnpm release:build vX.Y.Z`;
5. publish the multi-arch web image to GHCR and the native archive/checksums to GitHub Releases with `pnpm release:publish vX.Y.Z`.

The native release target is `x86_64-unknown-linux-gnu`; the production relay contract remains Linux + Bubblewrap. The GHCR image defaults to `ghcr.io/farismnrr/ai-code`, is built for `linux/amd64` and `linux/arm64`, and publishes `vX.Y.Z`, `X.Y.Z`, and `latest` tags for stable releases. The publish script fails closed unless the checkout is clean, on `main`, the requested tag points at `HEAD`, and that tag is already present on `origin`.

`pnpm release:build vX.Y.Z` runs the mandatory local gate, builds Nuxt, builds the native CLI, and writes the direct `dist/vX.Y.Z/ai-tools-x86_64-unknown-linux-gnu` download used by the UI, a `ai-tools-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` archive, and `SHA256SUMS`. `dist/` remains untracked build output.

## Verification policy

There is no CI and no unit-test suite. Quality is enforced locally before each normal commit.

For dependency changes, also run `pnpm audit` before merge. For security-sensitive Rust/MCP changes, run the relevant deterministic scripts under `scripts/` and `cargo audit` when applicable.

Existing deterministic acceptance/security scripts are targeted local verification tools; they are not unit tests and are not CI.

## Agent workflow and durable context

Agent guidance is centralized and vendor-neutral:

- [`AGENTS.md`](AGENTS.md) — the only repository agent entrypoint
- [`.agents/README.md`](.agents/README.md) — authoritative guidance index and context model
- [`.agents/memories/README.md`](.agents/memories/README.md) — **single canonical durable memory**
- [`.agents/plans/030-previous-plans-summary.md`](.agents/plans/030-previous-plans-summary.md) — one-time compact history of Plans 001–029b

Future plans start at **031** under `.agents/plans/`, stay as separate incrementing files, and are not automatically folded into Plan 030.

## Branching

Do not commit directly to `main` or `dev`. Work branches target `dev`; promotion from `dev` to `main` only happens when explicitly requested. See [`.agents/knowledge/git.md`](.agents/knowledge/git.md).
