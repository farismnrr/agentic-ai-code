# Plan 030 — Previous Plans Summary

**Status: COMPLETED / HISTORICAL SNAPSHOT**  
**Compacted: 2026-08-12**

This file is a **one-time compaction of every plan that existed before this snapshot**. The user explicitly closed all of them — including work that an older file still called in-flight — so future planning can start from current repository/external reality instead of inheriting stale checklists.

This summary consumes plan number **030**. The **next new plan is 031**.

> Future plans are different: create `031-...md`, `032-...md`, and so on as separate files and keep them. **Do not automatically compact post-030 plans into this file.** Only another explicit user-requested compaction may change that policy.

## What this snapshot means

- Plans `001` through `029b` are closed historical context, not an active backlog.
- Unchecked boxes in the old files are not automatically outstanding work after this reset.
- When an old concern becomes relevant again, inspect current source/config and current external behavior first, then create a fresh numbered plan.
- Current operational rules live in `AGENTS.md`, `.agents/knowledge/`, and the canonical [`../memories/README.md`](../memories/README.md).
- Git history retains the original long-form plans if exact implementation chronology is ever needed.

## Current carry-forward outcomes

The historical plans collectively produced the current architecture:

- Nuxt 4/Vue application with authenticated chat, workspaces, provider/model configuration, MCP integrations, persistence, telemetry, and responsive UI.
- Workspace-scoped coding behavior with server-owned data shaping and a configured filesystem root.
- AI SDK/LangGraph chat orchestration, explicit tool approval paths, direct `@search`, reasoning output, and long-context compaction.
- Native Rust executable CLIs for terminal, HTTP, and SearXNG search; no supported JavaScript executable fallback.
- Native Rust `relay-agent` using MCP `2026-07-28` Streamable HTTP, Linux Bubblewrap containment, non-root execution, explicit execution root, and local/remote security modes.
- Remote relay OAuth Resource Server behavior using JWKS, canonical resource/audience/issuer policy, owner binding, `relay.coding`, explicit trusted-proxy boundaries, and deterministic protocol/security checks.
- Docker intentionally unsupported/deferred unless an isolated execution backend exists.
- Repository policy is now **no CI + no unit-test suite + mandatory local pre-commit lint/typecheck gate**. Any CI-era text in historical work is superseded.

## Plan-by-plan compact history

| Plan | Historical outcome |
| --- | --- |
| **001 — Chat UI** | Established the initial external MCP client-like frontend chat experience and core interaction structure. |
| **002 — Landing/Auth Interactive** | Connected landing/login/app navigation and closed early interaction gaps. |
| **003 — Instrument Design** | Established the product's visual identity and design direction. |
| **004 — UI Responsiveness** | Audited/resolved responsive behavior from small mobile layouts through large desktop screens. |
| **005 — Backend Auth** | Added real session/auth flows, PostgreSQL/Drizzle persistence, OAuth/email verification, and persisted chat/settings/MCP data. |
| **006 — Error Handling** | Centralized server error behavior around Problem Details and aligned failure responses with actual scenarios. |
| **007 — Terminal/Workspace Identity** | Introduced the workspace-oriented coding-assistant identity, grouping, and the then-current 9Router model wiring. Provider assumptions from this plan are now historical. |
| **008 — Remove Dummy Data** | Removed demo/fixture leftovers and destructive demo reset behavior after real persistence/auth became authoritative. |
| **009 — Workspace Picker** | Required selecting/creating a workspace before chat and fixed the no-active-workspace race. |
| **010 — Workspace Folders** | Made workspaces represent real folders under an operator-configured root rather than unrestricted filesystem browsing. |
| **011 — Chat Prompt Sticky Footer** | Diagnosed/fixed dashboard minimum-height/footer clipping; the resulting layout override remains in current UI config. |
| **012 — MCP + API Keys** | Added API keys, the application's MCP surface, persisted third-party MCP servers, and native tool-approval integration. The old inbound SSE/session design is historical. |
| **013 — Chat Thinking & Animations** | Added real reasoning output for capable models and a motion pass for messages/reasoning UI. |
| **014 — Reasoning Effort & Model Cleanup** | Added reasoning-effort controls and removed a stray hardcoded model default. |
| **015 — Persist Active Workspace** | Persisted active workspace across browser/device state and fixed SSR composable-context/page-layout races discovered during rollout. |
| **016 — Workspace-Grouped Sidebar** | Grouped conversations by workspace, added workspace indication/details, and moved toward server-shaped sidebar data. |
| **017 — Explicit Workspace Targeting** | Added explicit workspace targeting for new chats so behavior no longer depended on whichever workspace happened to be active. |
| **018 — Chat Mode + LangGraph** | Established LangGraph/LangChain chat orchestration and AI SDK UI-stream bridging for tool-capable chat. |
| **019 — Search Mention Trigger** | Added explicit `@search`; provider-level forced tool choice proved unreliable, so the shipped path directly invokes search and emits compatible tool UI state. |
| **020 — Tools as Local Packages** | Established workspace packages/skills for curl/search. The original JavaScript CLI/bin design was later superseded by Plan 027. |
| **021 — Terminal Tool** | Added terminal-tool chat/agent integration. Original JavaScript executable details were later superseded by Rust migration/relay work. |
| **022 — OpenTelemetry Telemetry** | Introduced the OpenTelemetry/Jaeger/Loki direction and observability surfaces later used by the application. |
| **023 — User-Configurable Model Providers** | Replaced the hardcoded provider/model list with user-owned OpenAI-compatible, Anthropic-compatible, and Vertex-oriented provider configuration plus discovery/overrides. |
| **024 — Context Compaction** | Added bounded long-chat context handling, usage-aware compaction, bounded DB reads, context indicators, and related tool-approval race fixes. |
| **025 — Skeleton Lazy Loading** | Converted blocking application data loads to lazy/skeleton/error-retry patterns so slow panels degrade locally rather than crashing whole screens. |
| **026 — Local CLI Relay Agent** | First-generation local browser-to-relay plan. Its Node/WebSocket and weaker boundary assumptions were later superseded by the Rust relay in Plan 028. |
| **027 — CLI Rust Refactor** | Migrated terminal/curl/SearXNG standalone executable CLIs to Rust with parity, process-safety, SSRF, supply-chain, benchmark, release, and zero-JS-CLI cutover work. |
| **028 — Relay Agent Rust Rewrite** | Replaced the legacy Node/WebSocket relay with the native Rust MCP relay. Phase 12 removed the legacy relay surface; Phase 19 hardened Bubblewrap filesystem/process/privilege/OAuth security. |
| **029 — external MCP client Native MCP Integration** | Froze the stateless `POST /mcp` target, external OAuth Resource Server model, `relay.coding`, tool contract, protected-resource metadata, and deterministic conformance/security gates. |
| **029b — MCP Production Hardening** | Hardened trusted proxy handling, OAuth challenges/metadata, rate/admission controls, runtime tool snapshots, correlation metadata, MCP result conformance, and external MCP client-facing OAuth metadata. Docker stayed safely disabled without an isolated backend; live external external MCP client/OAuth acceptance was historically unverified. **Closed here for planning refresh.** |

## Important supersessions

Do not resurrect these merely because they appear in old plan history:

- GitHub Actions CI or automated native release workflows — intentionally removed; current quality/release posture is local/manual.
- Unit-test-suite requirements — current repository policy intentionally has no unit-test suite.
- Node/WebSocket relay or pairing-token relay architecture — superseded by the native Rust MCP relay.
- JavaScript standalone terminal/curl/search executables or npm bin mappings — superseded by Rust binaries.
- Old application-level inbound MCP SSE/session assumptions — current native relay target is stateless Streamable HTTP `POST /mcp`.
- Unrestricted/no-jail local execution assumptions — current relay requires Linux Bubblewrap containment, non-root execution, and an explicit execution root.
- Automatic trust of forwarded proxy headers — trust is explicit and peer-scoped.
- Raw host Docker socket access as a coding feature — explicitly rejected; Docker requires a genuinely isolated backend.
- Hardcoded 9Router-only provider assumptions — provider/model configuration evolved beyond them.

## Historically unproven external state

The old 029/029b stream did **not** have an approved deployed relay + real external MCP client workspace/app + OAuth tenant/client available to prove live Scan Tools, callback, PKCE, refresh, and end-to-end coding behavior. Repository/static/black-box checks were not represented as proof of that external state.

That fact is preserved only as history. Because all pre-030 plans were explicitly closed for refresh, it is **not an active gate now**. A future external MCP client/OAuth effort must re-check the current product UI/spec/tenant behavior from scratch and create a new plan (031+) if work is needed.

## Future planning rules

1. The next new plan number is **031**; never reuse `001`–`030`.
2. New multi-step work gets its own `NNN-kebab-case.md` file under `.agents/plans/`.
3. Keep the plan's status inside that file; there is no separate plan index file.
4. Completed post-030 plans stay as their own files for durable history.
5. **Do not fold future plans into this Plan 030 summary automatically.** This file is the one-time archive of the pre-reset plan set.
6. A new plan must start from current repository evidence and current external facts, not from an unchecked item copied out of the old plans.
