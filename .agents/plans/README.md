# Plans

Implementation plans for multi-step work, one Markdown file per effort.

**Naming: `NNN-kebab-case.md`** — a zero-padded 3-digit sequence number, then a short descriptive name (`001-chat-ui.md`, `002-auth-flow.md`). Take the next unused number; never reuse one, even after a plan is deleted. The number is the stable handle — it makes plans easy to reference in conversation ("lanjut 002") and keeps plans in creation order in the listing.

Write a plan here when a task spans several sessions or several files, so the next agent can pick it up without re-deriving the approach. A plan should state the goal, the steps in order, the files each step touches, and how to verify it worked. Mark steps done as you go.

**Plan mode output lands here too.** When a plan is produced in plan mode, write it to this folder under the next number — don't leave it in the harness's scratch location where it's invisible to the repo.

Keep shipped plans; move them to the Done list below rather than deleting, so the numbering stays meaningful and past decisions stay readable.

## In Flight
- [029b-chatgpt-mcp-production-hardening.md](029b-chatgpt-mcp-production-hardening.md) — focused follow-up for unresolved Plan 029 production issues only: trusted-proxy/HTTPS boundary, OAuth challenge semantics, safe Docker coding backend, black-box conformance, abuse admission, authoritative tool snapshot, correlation metadata, and live ChatGPT acceptance.
- [029-chatgpt-native-mcp-integration.md](029-chatgpt-native-mcp-integration.md) — native ChatGPT/Claude remote MCP integration: current stateless MCP transport, OAuth discovery/resource-server validation, explicit tool scopes, Secure MCP Tunnel/local connectivity, client E2E, observability, and strict conformance/CI/release gates.
- [028-relay-agent-rust-rewrite.md](028-relay-agent-rust-rewrite.md) — full rewrite of `packages/relay-agent` from Node.js/TypeScript + `@yao-pkg/pkg` to a standalone Rust binary, with strict Nuxt HTTP/WebSocket parity, localhost security, lifecycle parity, native release CI, and published-binary E2E verification.

## Completed
- [027-cli-rust-refactor.md](027-cli-rust-refactor.md) — migrate terminal-tool, curl-tool, and searxng-search-tool CLIs from JavaScript to Rust; full parity, SSRF/security hardening, deterministic test suites, release CI pipeline, benchmarks, and zero-JS cutover. PR #99.
- [026-local-cli-relay-agent.md](026-local-cli-relay-agent.md) — Relay agent: browser-to-localhost bridge, no internet in the data path.
- [025-skeleton-lazy-loading.md](025-skeleton-lazy-loading.md) — convert blocking `useFetch`/`useAsyncData` calls (sidebar/layout, settings pages, chat conversation load) to Nuxt's lazy pattern with `USkeleton` loading states and inline retry on error, so a slow/failed API degrades one panel instead of crashing the whole app.
- [024-context-compaction.md](024-context-compaction.md) — trim/summarize long chat histories against the model's `contextWindow` instead of sending the full message array unbounded every turn; four phases (heuristic then real usage-accounting compaction, bounded DB reads, context-usage indicator/freeze fix/tool-approval race fix) merged via PR #84 and #85.
- [023-user-configurable-model-providers.md](023-user-configurable-model-providers.md) — user-owned providers (OpenAI Compatible, Anthropic Compatible, Vertex AI Express Mode) and models replacing the hardcoded 9Router-only list; live model-ID discovery for the two Compatible types, a curated+curl-verified list for Vertex AI, per-model context/output/thinking overrides. Shipped well past the original plan: fixed `vertex_ai` actually calling the Gemini Developer API instead of real Vertex AI, custom request headers, provider/model dropdowns going stale until a reload, server errors never reaching Loki, and a docker-compose/Dockerfile bug (no workspace-root mount, missing CLI tools) that silently broke the terminal tool for every conversation.
- [017-explicit-workspace-targeting.md](017-explicit-workspace-targeting.md) — explicit workspace picker in the chat prompt form and per-workspace "New chat" in the sidebar, so starting a chat doesn't silently depend on whichever workspace happened to be "active."
- [016-workspace-grouped-sidebar.md](016-workspace-grouped-sidebar.md) — restructure the sidebar to group chats by workspace (not just the single active one), a workspace indicator in the chat header, and a "View details" action to see a workspace's full folder path without changing its short display name.
- [015-persist-active-workspace.md](015-persist-active-workspace.md) — the active workspace now survives a closed browser/new device via a server-side `users.lastActiveWorkspaceId` column, not just a session cookie; fixed two rounds of real bugs including an SSR composable-context break (NUXT_E1001) and a page/layout Suspense-boundary race.
- [014-reasoning-effort-and-model-cleanup.md](014-reasoning-effort-and-model-cleanup.md) — a low/medium/high/max reasoning-effort control for the "High Thinking" model via 9Router's `reasoning_effort`, plus fixing the stray hardcoded `gpt-4o-mini` default that doesn't match any real model.
- [013-chat-thinking-and-animations.md](013-chat-thinking-and-animations.md) — real reasoning output from the router model (`extractReasoningMiddleware`, gated by model capability) plus a ChatGPT-style motion pass on message/reasoning entrance.
- [012-mcp-api-key.md](012-mcp-api-key.md) — API keys + an MCP server this app exposes (settings/workspace/chat tools), plus wiring stored third-party `mcp_servers` rows into chat via `streamText`'s native tool-approval flow instead of just persisting them.
- [010-workspace-folders.md](010-workspace-folders.md) — workspaces become real folders (OpenClaw-style configured root) instead of just names, fixing the mismatch with opencode/Claude Code/Antigravity's actual model.
- [009-workspace-picker.md](009-workspace-picker.md) — require picking/creating a workspace on first `/chat` visit, and fix the uncaught "No active workspace" race.
- [008-remove-dummy-data.md](008-remove-dummy-data.md) — remove leftover fixture/seed data and the destructive demo-reset endpoint now that auth, persistence, and the model are real.
- [007-terminal-workspace-identity.md](007-terminal-workspace-identity.md) — opencode-web/OpenClaw-inspired rebrand + workspace grouping + real model wiring via 9Router.
- [006-error-handling.md](006-error-handling.md) — centralize server error handling (RFC 9457 Problem Details) and audit every 4xx/5xx to match its real failure scenario.
- [005-backend-auth.md](005-backend-auth.md) — real backend auth (cookie session, Postgres, OAuth, email verification) then chat/settings/MCP persistence.
- [004-ui-responsiveness.md](004-ui-responsiveness.md): Audit and resolve responsiveness on Mobile S through Desktop 2K across all UI pages.

- [001-chat-ui.md](001-chat-ui.md) — ChatGPT-like AI chat UI, frontend only
- [002-landing-auth-interactive.md](002-landing-auth-interactive.md) — landing → login → app, and closing the interaction gaps
- [003-instrument-design.md](003-instrument-design.md) — "Instrument": give the product a visual identity
