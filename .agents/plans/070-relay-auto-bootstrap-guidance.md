# Plan 070 — Relay Auto-Bootstrap Guidance

Created: 2026-09-20
Updated: 2026-09-20

Status: **FOLLOW-UP VERIFIED / COMMIT READY — the first deployed Plan 070 build proved relay advertisement and first-party Nuxt consumption, but a fresh ChatGPT external-connector acceptance did not follow the Task Execution Report despite successful tool use, proving `server/discover.instructions` alone is best-effort for that host. The same Plan 070 now adds bounded model-visible compatibility reinforcement by appending the compact reporting contract to every MCP wire tool description while leaving internal catalog definitions and structured tool-result contracts unchanged. An attempted tool-result reminder was rejected during security regression because structured tools intentionally return empty `content` plus `structuredContent`; that experiment was fully reverted. Resource 11/11, Full/Primary wire-security tests, Rust check, repo/agent/architecture guards, `git diff --check`, and `pnpm guardrail:fast` all pass. Follow-up commit, rebuild/restart, reconnect, and fresh external acceptance remain.**

## Goal

Make the workspace-agnostic Masih Awam bootstrap/reporting contract available automatically whenever the first-party relay is used, without requiring the operator to copy the bootstrap into every repository/client manually and without allowing untrusted third-party MCP servers to inject model instructions.

## Current contract

- `ai-self/BOOTSTRAP.md` remains the human-readable portable base contract.
- Rust MCP `server/discover.instructions` and legacy `initialize.instructions` advertise a bounded universal bootstrap summary covering fresh workspace verification, approved guidance, the Task Execution Report structure, truthful verification/delivery reporting, and foreground handoff for work beyond public tool deadlines.
- `workspace://<repo>/agent-guidance` includes `ai-self/BOOTSTRAP.md` when present, before repository `AGENTS.md` and `.agents/knowledge/resources.md`.
- The first-party Nuxt modern MCP client stores discovered server instructions only for provenance `first-party-relay`.
- Chat and delegated subagents inject only those trusted first-party relay instructions into model system context.
- Instructions from external/third-party MCP servers must never be promoted into model system instructions by the Nuxt mechanism.
- External hosts such as ChatGPT may ignore `server/discover.instructions`; therefore the Rust relay additionally appends a compact reporting contract to every wire tool description. Internal catalog descriptions and tool-result envelopes remain unchanged, preserving existing structured-output semantics.

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
- [x] Wire tool descriptions append the compact reporting contract without mutating internal catalog definitions.
- [x] A tool-result-content reminder experiment was rejected and fully reverted after the Primary wire-security regression proved structured tools intentionally require empty `content` alongside `structuredContent`; external compatibility must not weaken that contract.
- [x] Re-verify the external-client compatibility follow-up: resource platform 11/11 PASS; Full and Primary wire-security acceptance PASS; Rust check, repo-policy, agent-docs, architecture, `git diff --check`, and `pnpm guardrail:fast` PASS.
- [ ] Commit the external-client compatibility follow-up.
- [ ] Rebuild/install/restart the relay, reconnect the client, and prove a fresh ChatGPT external-connector tool task follows the report contract.
- [ ] Push/PR/reviewed merge only with explicit user authorization.

## Client limitation

MCP server instructions remain the standard relay-side automatic advertisement, but live ChatGPT acceptance proves an arbitrary external MCP host may ignore them. The first-party Nuxt path is wired explicitly and remains under repository control. For external hosts, Plan 070 additionally places the compact report contract in first-party MCP wire tool descriptions, which are already model-visible during tool selection. Tool-result payloads are deliberately left unchanged. This is compatibility reinforcement, not a claim that an external host can be forced to obey arbitrary server policy. Manual client-level bootstrap installation remains the final fallback if the host also ignores tool descriptions.
