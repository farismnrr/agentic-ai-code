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
- `server/api/` — Nitro HTTP API routes / transport adapters.
- `server/application/` — application use cases/policies and application-owned contracts. These modules do not import concrete infrastructure, DB/Drizzle, H3/Nitro, or provider/AI/MCP implementation types.
- `server/infrastructure/` — database, AI/provider, MCP, and other concrete integration adapters introduced/moved during Plan 031/031A.
- `server/utils/` — legacy/mixed server helpers still present in current source; do **not** assume every file here is a pure utility. Plan 031B explicitly audits and relocates files whose real owner is application or infrastructure.
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

### Current shipped architecture

The post-closure remediation is implemented at `bd22cc6`. The shipped server follows:

```text
server/api (transport/composition)
  -> server/application (use cases/policies and application-owned contracts)
      <- server/infrastructure (DB / AI SDK / providers / LangGraph / MCP / filesystem/network adapters)
```

The final rules are:

- `server/api/**` handles auth, HTTP parsing/validation, use-case invocation, and response adaptation rather than business/persistence ownership. Concrete adapters are composed at the Nitro application-context/plugin boundary, not imported by routes;
- `server/application/**` owns use-case/business semantics and application-facing contracts without importing concrete infrastructure, Drizzle, H3/Nitro event types, or AI/provider/MCP implementation SDKs;
- `server/infrastructure/**` implements application contracts and owns concrete DB, provider, AI SDK/LangGraph, MCP, filesystem/network, and similar integrations;
- `scripts/check-architecture.sh` (run from `pnpm verify:commit`) enforces application/API import boundaries and representative negative fixtures, including type-only and facade bypasses;
- frontend components are grouped by feature (`app/components/{chat,workspace,settings,shell}/`), while genuinely cross-feature/landing primitives may remain at the component root;
- Rust relay transport keeps router/bootstrap composition separate from access-policy/OAuth orchestration and MCP request/tool/task handlers, while focused auth, validation, admission, and observability modules remain the policy owners; application execution/workspace, MCP catalog, and CLI-vs-validated-config responsibilities are likewise split behind stable facades;
- native Rust remains the executable tool source of truth and sibling TypeScript packages remain integration APIs; Plan 039B adds bounded read-only Git intelligence through fixed Git CLI argv/config and a constrained native `apply_patch` mutation primitive that reuses workspace containment/atomic replacement rather than shelling out to `patch`.
- Plan 039C is CLOSED / VERIFIED. It adds a narrow, vendor-neutral LSP substrate under `packages/rust-tools/application/src/lsp/` (session/process lifecycle in `manager.rs`/`protocol.rs`/`document.rs`; language-agnostic symbol/definition/reference/hover/diagnostic/rename normalization shared by every adapter in `semantic.rs` + `rename.rs`; thin per-language wrappers in `rust.rs` and `typescript.rs` covering rust-analyzer and typescript-language-server/`@vue/language-server`). It stays outside MCP transport routing; `packages/rust-tools/application/src/code.rs` is the only place that adapts it into the public `code_*` MCP tools. `.vue` uses the bounded `tsserver_bridge` with the already-installed `@vue/typescript-plugin` shipped inside `@vue/language-server`; exact `_vue:*` commands are preserved on the wire. Full Vue definition/references/hover/diagnostics remain an explicit non-blocking limitation of this installed server build rather than a claimed semantic pass; document symbols are proven real.
- Plan 039D adds the shared capability effect/risk contract at `shared/utils/capability-policy.ts`, persisted conversation permission modes, structured approval facts, component-aware protected credential paths in `relay-core`, and opt-in terminal network access. First-party approval state narrows remembered approvals; MCP annotations remain hints and relay hard policy remains authoritative.
- Plan 039I is CLOSED / VERIFIED. Stable MCP tool identity/scoping remains lockstep with approval ownership, and the relay exposes only bounded server-owned read-only `workspace://` resources for manifest, approved agent guidance, Git status, and HEAD metadata under MCP `2026-07-28`; templates/subscriptions are intentionally absent.
- Plan 039J composes existing agent surfaces in the Nuxt UI: category-driven tool presentation, sensitivity-aware approval summaries, bounded diff/result previews, compact subagent/background cards, and task/context state. Semantic agent/tool observability stays on the Plan-035 logger/OTel sanitizer chokepoint; raw prompts, tool arguments/results, credentials, private absolute paths, and raw provider/tool errors are never telemetry attributes.

The remediation also restored application ownership boundaries across API composition, identity/settings/conversation/model/workspace use cases, and infrastructure adapters. Provider credential containment, repository-wide API ownership, mixed utility cleanup, strict architecture probes, and JWT pre-validation compatibility remain implemented. Same-origin redirect handling remains bounded and policy-validated; cross-origin authenticated provider redirects are rejected.

### Agent/project guidance

- `docs/` — human/operator installation, deployment, MCP client, development, and release handbook.
- `AGENTS.md` — single repository agent entrypoint.
- `.agents/knowledge/` — stable operating guidance.
- `.agents/skills/` — shared framework/tool skill discovery.
- `.agents/memories/README.md` — single canonical durable memory.
- `.agents/plans/030-previous-plans-summary.md` — compacted pre-reset plan history.
- `.agents/plans/031-...md` onward — future plans, one file per effort, incrementing and retained separately; explicit lowercase-letter follow-ups remain in the same plan family.
- `.agents/contracts/` — frozen client-visible contract evidence.
- `.githooks/pre-commit` — mandatory local commit gate.

The repository intentionally has **no CI workflow** and **no unit-test suite**.

## Normal verification

### Mandatory before every commit

Every normal local commit must pass:

```sh
pnpm verify:commit
```

The tracked pre-commit hook runs the same command automatically after `pnpm install`. Never bypass it with `git commit --no-verify` or by disabling `core.hooksPath`.

`pnpm verify:commit` runs repository-policy checks, agent-doc integrity, architecture-boundary checks, deterministic maintainability budgets, `pnpm lint`, and `pnpm typecheck`. `pnpm lint` covers ESLint plus Rust formatting/Clippy. `pnpm typecheck` generates the Nuxt type project, runs direct generated-project Vue typing, and performs warnings-denied Rust `cargo check`. Maintainability policy is implemented by `scripts/check-maintainability.mjs`: >500 maintained-source lines and >15 direct maintained implementation files are hard failures unless an exact-path, reasoned exception exists; >400 lines and 13–15 files are explicit review findings.

There is no remote CI safety net. Change-request descriptions must record local verification performed; forge mergeability is not proof of quality.

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
