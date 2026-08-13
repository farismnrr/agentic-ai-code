# Project

## What this repository is

`ai-code` is a Nuxt 4 coding-assistant application with a Rust native-tool/relay workspace. The web app provides authenticated chat, workspaces, model/provider configuration, MCP integrations, telemetry, and local/remote coding-agent execution surfaces.

The repository is deliberately split between:

- **Nuxt/Vue/TypeScript** for the web application, server APIs, persistence, model orchestration, MCP application wiring, and UI;
- **Rust** for the executable terminal/curl/search CLIs and the native MCP relay/security boundary.

Do not infer current architecture from historical plans alone. Current source/config plus `.agents/knowledge/` and the canonical [`../memories/README.md`](../memories/README.md) are the operational source of truth. Plan history through 029b is compacted in [`../plans/030-previous-plans-summary.md`](../plans/030-previous-plans-summary.md).

## Current stack

- Nuxt 4 / Vue 3 / TypeScript
- Nuxt UI + Tailwind CSS
- AI SDK + LangChain/LangGraph provider/orchestration paths
- PostgreSQL + Drizzle ORM
- session/OAuth/email auth flows
- OpenTelemetry + Jaeger/Loki integration
- MCP inbound/outbound integration
- Rust workspace under `packages/rust-tools/`
- Rust `relay-agent` with MCP Streamable HTTP and Linux Bubblewrap containment

## Repository orientation

### Web application

- `app/` — Vue pages, layouts, components, composables, plugins, and client UI.
- `server/api/` — Nitro HTTP API routes.
- `server/utils/` — shared server/domain integration logic used by routes and model/tool orchestration.
- `server/plugins/` — server initialization such as telemetry.
- `server/database/` — Drizzle schema and migrations.
- `shared/` — types/utilities shared across client/server boundaries.
- `nuxt.config.ts` / `app/app.config.ts` — Nuxt/runtime/UI configuration.

### Native tools and relay

- `packages/rust-tools/` — Rust workspace/source, native binaries, security policy, and release-oriented docs.
- `packages/terminal-tool/` — TypeScript terminal-tool API/skill wrapper; executable CLI is Rust.
- `packages/curl-tool/` — TypeScript curl-tool API/skill wrapper; executable CLI is Rust.
- `packages/searxng-search-tool/` — TypeScript search-tool API/skill wrapper; executable CLI is Rust.
- `packages/relay-agent/` — relay package metadata/skill; current executable is the Rust `relay-agent` binary from the Rust workspace.

The TypeScript package APIs remain valid application integration surfaces. Historical Plan 027 migrated the **executable CLI layer**, not the entire Nuxt runtime, to Rust.

Architecture boundaries reinforced by Plan 031: server application modules
compose use cases without H3 event objects; Rust relay transport owns HTTP
composition/security ordering while focused auth, validation, admission, and
observability modules own their policies; native Rust remains the executable
tool source of truth and sibling TypeScript packages remain integration APIs.

### Agent/project guidance

- `AGENTS.md` — single repository agent entrypoint.
- `.agents/knowledge/` — stable operating guidance.
- `.agents/skills/` — shared framework/tool skill discovery.
- `.agents/memories/README.md` — single canonical durable memory.
- `.agents/plans/030-previous-plans-summary.md` — compacted pre-reset plan history.
- `.agents/plans/031-...md` onward — future plans, one file per effort, incrementing and retained separately.
- `.agents/contracts/` — frozen client-visible contract evidence.
- `.githooks/pre-commit` — mandatory local commit gate.

The repository intentionally has **no CI workflow** and **no unit-test suite**.

## Normal verification

### Mandatory before every commit

Every commit must pass:

```sh
pnpm verify:commit
```

The tracked pre-commit hook runs the same command automatically after `pnpm install`. Never bypass it with `git commit --no-verify` or by disabling `core.hooksPath`.

`pnpm verify:commit` runs repository policy checks, agent-doc integrity, `pnpm lint`, and `pnpm typecheck`. `pnpm lint` covers ESLint plus Rust formatting/Clippy. `pnpm typecheck` generates the Nuxt type project, runs direct generated-project Vue typing, and performs warnings-denied Rust `cargo check`.

There is no remote CI safety net. PR descriptions must record local verification performed; GitHub mergeability is not proof of quality.

See [`../memories/README.md`](../memories/README.md#repository-policy-and-verification).

### Web application

`pnpm typecheck` runs `nuxt prepare --dotenv .env.example`, then `vue-tsc -p .nuxt/tsconfig.json --noEmit`, followed by the Rust workspace check. Do not replace the Nuxt/Vue portion with bare `nuxt typecheck`; that wrapper previously exited successfully while real generated-project errors remained.

Production bundling remains a separate runtime verification concern. Run `pnpm build` when the change needs bundling/SSR output verified in addition to the mandatory commit gate.

### Run the built app for local verification

Prefer `pnpm build && pnpm preview` over trusting a long-lived `pnpm dev` when verifying completed work. The dev watcher has produced stale-module/`ENOTDIR` failures after branch switches and file moves. Rebuild and restart preview after output changes.

### Rust workspace

The mandatory local gates pin the repository toolchain and cover formatting, warnings-denied `cargo check`, and Clippy. Security-sensitive Rust changes may additionally require `cargo audit` and the relevant deterministic scripts under `scripts/`.

The production `relay-agent` contract is Linux + Bubblewrap. Do not document macOS/Windows relay support unless the sandbox/release contract changes deliberately.

### No unit tests

Do not introduce a unit-test framework or unit-test suite by default. Existing deterministic acceptance/security scripts may remain as explicit local verification for protocol/security boundaries; they are not CI and are not a substitute for the mandatory lint/typecheck gate.

## Runtime/config orientation

- Start from [`.env.example`](../../.env.example) for available environment keys.
- Runtime config conventions and stable setup notes live in [`tooling.md`](tooling.md).
- Provider, MCP, relay, auth, and database behavior should be verified against current source/config before editing docs because those surfaces changed substantially across historical plans.
- Client-visible MCP contracts that are intentionally frozen live in [`../contracts/`](../contracts/); do not change them casually.
