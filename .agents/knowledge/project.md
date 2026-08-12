# Project

AI Code is a Nuxt 4 application with a native Rust tool/relay workspace. It started from the Nuxt UI starter, but the repository now includes authenticated AI chat, configurable providers, persistent workspaces/conversations, inbound/outbound MCP surfaces, telemetry, and sandboxed native coding tools.

Use this file as the orientation map, not as a replacement for runtime source/config. Exact dependency versions live in [`../../package.json`](../../package.json), and exact Rust toolchain/release configuration lives under [`../../packages/rust-tools/`](../../packages/rust-tools/) plus CI.

## Main stack

- **Nuxt 4 / Vue / Nuxt UI 4 / Tailwind CSS 4** — web application and UI.
- **AI SDK 7 + LangChain** — chat/model/tool orchestration.
- **Anthropic, OpenAI-compatible, Vertex AI** — configurable model-provider families.
- **PostgreSQL + Drizzle ORM** — application persistence and migrations.
- **`nuxt-auth-utils`** — application session/auth integration.
- **Model Context Protocol** — application MCP integration plus the Rust relay server.
- **Rust 1.95.0** — pinned native-tool workspace for `terminal-tool`, `curl-tool`, `searxng-search-tool`, and `relay-agent`.
- **pnpm 11** — JavaScript workspace/package manager; exact version is pinned in `package.json`.

## Repository layout

```text
app/
  components/           Nuxt/Vue UI components
  composables/          Client/shared application state and chat helpers
  layouts/              Application layouts
  pages/                File-based routes
  plugins/              Nuxt client/runtime plugins
  assets/               CSS and static app assets

server/
  api/                  Nitro API routes (auth, chat, workspaces, providers, MCP, etc.)
  database/             Drizzle schema/persistence support
  utils/                Server-side helpers and integrations
  plugins/              Server runtime/telemetry hooks

shared/                 Types/schemas shared across client and server

packages/
  terminal-tool/        AI-facing package/skill for terminal execution
  curl-tool/            AI-facing package/skill for HTTP requests
  searxng-search-tool/  AI-facing package/skill for search
  relay-agent/          Relay packaging/release surface
  rust-tools/           Native Rust implementations for all four binaries

scripts/                Deterministic MCP/security/release acceptance scripts
.agents/                Shared agent knowledge, skills, plans, memories, contracts
.github/workflows/       CI and release automation
nuxt.config.ts           Nuxt/runtime/module configuration
.mcp.json                Project-scoped MCP client configuration
```

Do not assume a directory is disposable from its name. Check its contents and existing memories first; this repo has previously lost real configuration by treating fixture-like paths as throwaway.

## Commands

| Task | Command |
| --- | --- |
| Install/prepare | `pnpm install` |
| Production build | `pnpm build` |
| Build native tools | `pnpm build:tools` |
| Local production-like verification | `pnpm build && pnpm preview` |
| Dev server (use intentionally) | `pnpm dev` |
| Lint | `pnpm lint` |
| Nuxt/Vue type gate | `pnpm typecheck` |
| Dependency audit | `pnpm audit` |
| Generate DB migration | `pnpm db:generate` |
| Apply DB migrations | `pnpm db:migrate` |
| Regenerate Nuxt types/native postinstall artifacts | `pnpm postinstall` |

## Verification rules

### Web application

Before declaring web/application work complete, run the gates relevant to the change. At minimum the normal repository gates are:

```sh
pnpm lint
pnpm typecheck
pnpm audit
```

`pnpm typecheck` is intentionally a generated-project gate: it builds Nuxt against `.env.example`, then runs `vue-tsc -p .nuxt/tsconfig.json --noEmit`. Do not replace it with bare `nuxt typecheck`; that wrapper previously exited successfully while real generated-project errors remained.

Use `pnpm build` separately when runtime bundling/SSR output itself needs verification beyond the type gate. See [`../memories/007-typecheck-gate-was-silent.md`](../memories/007-typecheck-gate-was-silent.md) and [`../memories/013-nuxt-ui-slot-typecheck-gate.md`](../memories/013-nuxt-ui-slot-typecheck-gate.md).

### Run the built app for local verification

Prefer `pnpm build && pnpm preview` over `pnpm dev` when verifying completed work. The dev watcher has produced stale-module/`ENOTDIR` failures after branch switches and file moves. Rebuild and restart preview after the output changes; preview does not pick up a new `.output` automatically.

See [`../memories/dev-mode-vs-build.md`](../memories/dev-mode-vs-build.md).

### Rust workspace

CI pins Rust 1.95.0 and currently enforces the native workspace with commands equivalent to:

```sh
cd packages/rust-tools
cargo fmt --all -- --check
RUSTFLAGS='-D warnings' cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo audit
```

The production `relay-agent` contract is Linux + Bubblewrap. Do not document macOS/Windows relay support unless the sandbox/release contract changes deliberately.

## Runtime/config orientation

- Start from [`.env.example`](../../.env.example) for available environment keys.
- Runtime config conventions and stable setup notes live in [`tooling.md`](tooling.md).
- Provider, MCP, relay, auth, and database behavior should be verified against current source/config before editing docs because those surfaces have changed substantially across recent plans.
- Client-visible MCP contracts that are intentionally frozen live in [`../contracts/`](../contracts/); do not change them casually.
