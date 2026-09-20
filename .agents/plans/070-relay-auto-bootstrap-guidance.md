# Plan 070 — Relay Auto-Bootstrap Guidance

Created: 2026-09-20
Updated: 2026-09-20

Status: **SOURCE IMPLEMENTATION COMPLETE / COMMIT READY — relay-side bootstrap advertisement and first-party consumption are implemented locally; focused Rust/web trust-boundary acceptance passes, and operator-run `pnpm typecheck` plus `pnpm guardrail:fast` both pass. Local commit is next; release build/install/restart and live connector acceptance remain afterward.**

## Goal

Make the workspace-agnostic Masih Awam bootstrap/reporting contract available automatically whenever the first-party relay is used, without requiring the operator to copy the bootstrap into every repository/client manually and without allowing untrusted third-party MCP servers to inject model instructions.

## Current contract

- `ai-self/BOOTSTRAP.md` remains the human-readable portable base contract.
- Rust MCP `server/discover.instructions` and legacy `initialize.instructions` advertise a bounded universal bootstrap summary covering fresh workspace verification, approved guidance, the Task Execution Report structure, truthful verification/delivery reporting, and foreground handoff for work beyond public tool deadlines.
- `workspace://<repo>/agent-guidance` includes `ai-self/BOOTSTRAP.md` when present, before repository `AGENTS.md` and `.agents/knowledge/resources.md`.
- The first-party Nuxt modern MCP client stores discovered server instructions only for provenance `first-party-relay`.
- Chat and delegated subagents inject only those trusted first-party relay instructions into model system context.
- Instructions from external/third-party MCP servers must never be promoted into model system instructions by this mechanism.

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
- [ ] Commit the Plan 070 implementation locally.
- [ ] Release-build/install/restart the relay and reconnect clients.
- [ ] Live acceptance proves the connected client receives/uses the bootstrap contract.
- [ ] Push/PR/reviewed merge only with explicit user authorization.

## Client limitation

MCP server instructions are the standard relay-side automatic advertisement, but an arbitrary external MCP client may choose not to surface or apply server instructions/resources to its model. The first-party Nuxt path is wired explicitly and is therefore under repository control. Manual client-level bootstrap installation remains a fallback only for external clients that ignore the server-provided contract.
