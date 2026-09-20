# Plan 070 — Relay Auto-Bootstrap Guidance

Created: 2026-09-20
Updated: 2026-09-20

Status: **CLOSED FOR CURRENT PLAN 070 SCOPE — implementation, local verification, release build/install/restart, reconnect, Skill packaging, live external `/init` acceptance, repository-required full closure validation, and the local closure commit are complete. The retained `workspace_bootstrap` capability is manual-only: `inspect` is read-only; `reconcile` creates missing portable governance and refreshes only Masih Awam-managed generated files after re-detecting the current stack. Existing unowned guidance/configuration and durable memory are preserved. Live acceptance proves first initialization, idempotent no-op reconciliation, Node→Node+Rust stack growth, managed-file-only refresh, final clean inspect state, and a subsequent coding task with verification/guardrail execution under the installed ChatGPT Skill. Focused repository tests additionally cover stack shrink/removal of stale managed checks. External ChatGPT follows the workflow/report structure but may normalize Markdown heading punctuation or completion-sentinel wording; exact presentation formatting is host behavior, not a deterministic relay contract. `pnpm guardrail:full` passes on 2026-09-20 after the maintainability split and retained-base catalog expectation update. Push/PR/reviewed merge remain separately authorized and were not performed as part of closure.**

## Goal

Make the workspace-agnostic Masih Awam bootstrap/reporting contract available automatically whenever the first-party relay is used, without requiring the operator to copy the bootstrap into every repository/client manually and without allowing untrusted third-party MCP servers to inject model instructions.

## Current contract

- `ai-self/BOOTSTRAP.md` remains the human-readable portable base contract.
- Rust MCP `server/discover.instructions` and legacy `initialize.instructions` advertise a bounded universal bootstrap summary covering fresh workspace verification, approved guidance, the Task Execution Report structure, truthful verification/delivery reporting, and foreground handoff for work beyond public tool deadlines.
- `workspace://<repo>/agent-guidance` includes `ai-self/BOOTSTRAP.md` when present, before repository `AGENTS.md` and `.agents/knowledge/resources.md`.
- The first-party Nuxt modern MCP client stores discovered server instructions only for provenance `first-party-relay`.
- Chat and delegated subagents inject only those trusted first-party relay instructions into model system context.
- Instructions from external/third-party MCP servers must never be promoted into model system instructions by the Nuxt mechanism.
- External hosts such as ChatGPT may ignore `server/discover.instructions`. MCP tool descriptions remain factual capability metadata; ChatGPT behavioral policy is supplied by the installed Masih Awam Workspace Workflow Skill instead of prompt-like tool-description suffixes.

## Manual `/init` workspace-governance reconciliation

- Governance provisioning must never be an implicit side effect of ordinary repository work.
- An agent should invoke `workspace_bootstrap` only when the user explicitly requests `/init` or asks to initialize/reconcile workspace governance.
- `inspect` is read-only and resolves the verified Git root, current governance state, Node/Rust/Python/Go stack markers, package manager, and safely adopted existing validation commands.
- `reconcile` is idempotent and ownership-aware. It creates missing portable `ai-self/` + `.agents/` files and a local stack-aware guardrail, then refreshes only files carrying the Masih Awam managed marker.
- Existing unowned `AGENTS.md`, governance, scripts/configuration, and durable canonical memory must be preserved.
- Re-running `/init` after the repository evolves must re-detect the stack. Newly introduced stacks/checks are added to managed governance and stale generated checks are removed when they are no longer detected.
- Reconcile must not install dependencies, replace existing project tooling, commit, push, create a PR, merge, deploy, or restart anything.
- The generated local maintainability baseline is stack-aware and applies only to detected maintained-source extensions; it is a governance baseline, not a license for metric-only splits.

## Acceptance

- [x] Relay discovery/initialize share one `SERVER_INSTRUCTIONS` contract.
- [x] Agent-guidance resource includes `ai-self/BOOTSTRAP.md` when present.
- [x] First-party modern client exposes discovered instructions; external provenance exposes none.
- [x] Top-level agent chat includes trusted relay instructions.
- [x] Delegated subagents preserve trusted relay instructions.
- [x] Rust resource/discovery focused regressions pass.
- [x] Targeted TypeScript modern-client, MCP scoping, chat authority, and top-level/subagent composition regressions pass (combined acceptance: 14/14).
- [x] Focused Rust platform resource suite passes (10/10), including discover bootstrap and ordered `agent-guidance` composition.
- [x] Current docs describe automatic relay bootstrap semantics and client limitations truthfully.
- [x] Targeted ESLint on all changed TypeScript/test files passes; Rust fmt/check and `git diff --check` pass.
- [x] Repo-policy, agent-doc, and architecture guards pass.
- [x] `pnpm guardrail:fast` passes after the complete delta (operator foreground, 2026-09-20).
- [x] Full typecheck passes after the complete delta (operator foreground, 2026-09-20).
- [x] Commit the Plan 070 implementation locally as `e3a343a` (`feat(relay): auto-bootstrap trusted guidance`).
- [x] Release-build/install/restart the relay: operator foreground build PASS; installed binary SHA256 matches release artifact (`261a3a9e4d35998843a8cf8a86693a90b75a98507a9fb0eee96abb5a9daf3867`); `ai-tools-relay.service` is active; post-restart MCP `terminal_exec true` succeeds.
- [x] First external ChatGPT reconnect acceptance proves tool transport works but `server/discover.instructions` alone is insufficient: tool call succeeds, yet the final response remains prose instead of the required Task Execution Report.
- [x] The temporary wire tool-description reporting suffix was removed after the ChatGPT Skill proved to be the correct external behavioral layer; wire descriptions again match canonical factual catalog descriptions.
- [x] A tool-result-content reminder experiment was rejected and fully reverted after the Primary wire-security regression proved structured tools intentionally require empty `content` alongside `structuredContent`; external compatibility must not weaken that contract.
- [x] Re-verify the external-client compatibility follow-up: resource platform 11/11 PASS; Full and Primary wire-security acceptance PASS; Rust check, repo-policy, agent-docs, architecture, `git diff --check`, and `pnpm guardrail:fast` PASS.
- [x] Commit the external-client compatibility follow-up as `da579e5` (`fix(relay): reinforce external reporting guidance`).
- [x] Installed ChatGPT Skill auto-invocation is observed in a fresh external-client tool task: the report sections appear without explicit skill invocation; exact sentinel/header wording still needs Skill refinement.
- [x] Add retained `workspace_bootstrap` to Primary and Full with structured output and workspace-write effect only for `reconcile`.
- [x] `workspace_bootstrap inspect` is read-only and stack/package-manager aware.
- [x] `workspace_bootstrap reconcile` creates missing governance, refreshes only Masih Awam-managed files, preserves unowned guidance and canonical memory, and installs no dependencies.
- [x] Focused acceptance proves first initialization creates plans/skills infrastructure, a second reconcile after Node→Node+Rust stack growth updates managed tooling/guardrail while preserving memory/user guidance, a later Rust removal removes stale managed Rust checks, and a no-change reconcile is idempotent.
- [x] Re-run broader verification after the manual `/init` extension: workspace tools 6/6 PASS; resource/discovery 11/11 PASS; Full and Primary wire-security acceptance PASS; `cargo check -p ai-tools`, `cargo fmt --all -- --check`, repo-policy, agent-docs, architecture, `git diff --check`, and `pnpm guardrail:fast` PASS.
- [x] Rebuild/install/restart relay and reconnect the client after the manual `/init` extension. The installed binary matches the release artifact SHA256 `ecb9890bbfc1f6abb988fe2f38b35a0ed70548197fc506e99033d953ecc92f1f`; `ai-tools-relay.service` is active; fresh external ChatGPT sessions discover and execute `workspace_bootstrap`.
- [x] Refine, track, package, and re-upload the ChatGPT Skill so `/init` maps to the manual bootstrap workflow. Fresh external acceptance proves `/init` reconciliation, post-state verification, subsequent coding work, `pnpm run smoke`, and repository guardrail execution. The host emits the Task Execution Report sections but may normalize Markdown heading punctuation and completion-sentinel wording; this presentation variance is documented as host-dependent and is not a relay correctness blocker.
- [x] Final closure validation passes with `pnpm guardrail:full` on 2026-09-20 after splitting bootstrap acceptance into `tests/workspace_bootstrap.rs` and updating the retained Full base expectation from 50 to 51. The final run passes maintainability, test layout, fmt, clippy `-D warnings`, check, and the complete Rust test suite with only the documented operator/network-dependent ignores.
- [ ] Push/PR/reviewed merge only with explicit user authorization.

## Client limitation

MCP server instructions remain the standard relay-side automatic advertisement, but live ChatGPT acceptance proves an arbitrary external MCP host may ignore them. The first-party Nuxt path is wired explicitly and remains under repository control. External ChatGPT behavior is supplied by the installed Masih Awam Workspace Workflow Skill; MCP tool descriptions stay factual and tool-result payloads remain unchanged. Live ChatGPT also demonstrates that behavioral guidance and structured reporting can be followed while exact Markdown heading punctuation and completion-sentinel wording are still normalized by the host. Treat those presentation details as host-dependent rather than weakening MCP contracts to force them. Other external clients need their own supported global/skill-style instruction mechanism when they ignore server instructions/resources.
