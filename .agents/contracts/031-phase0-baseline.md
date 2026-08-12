# Plan 031 Phase 0 — Baseline and Architectural Contracts

Recorded 2026-08-12 from the working tree before any Plan 031 runtime changes.
This is an inventory and preservation contract for later phases; it is not a
second implementation of any runtime rule.

## Current feature boundaries and dependency direction

| Area | Current entrypoints | Current owned concerns | Refactor boundary to preserve |
| --- | --- | --- | --- |
| Presentation | `app/pages/`, `app/layouts/`, `app/components/` | route/layout composition, rendering, user intent | no server/database/provider imports from presentation |
| Client application | `app/composables/`, `app/utils/` | reactive state, lifecycle, client API orchestration, editor behavior | request-safe Nuxt state and public composable entrypoints |
| Shared contracts | `shared/types/`, `shared/schemas/`, `shared/utils/` | runtime-neutral client/server types and pure values | no server-only, filesystem, or Vue runtime dependencies |
| HTTP transport | `server/api/`, `server/routes/`, `server/middleware/` | auth/session, request parsing, response/stream adapters | public route paths, methods, payloads, and response shapes |
| Server application/integration (currently co-located) | `server/utils/`, `server/plugins/` | persistence, provider/model orchestration, chat, MCP, filesystem, telemetry | later extraction must use narrow ports without changing security or persistence semantics |
| Database | `server/database/` | Drizzle connection, schema, migrations | schema and migration authority; no migration in this refactor phase |
| Native tools | `packages/rust-tools/src/bin/` | terminal, curl, search CLI behavior | arguments, stdout/stderr, exit codes, timeout and security behavior |
| Relay | `packages/rust-tools/src/relay_agent/` | HTTP transport, MCP protocol, auth/security, execution | transport-independent protocol and fail-closed security ordering |
| TypeScript package facades | `packages/{terminal,curl,searxng-search}-tool/` | application integration APIs and schemas | package exports remain valid; executable ownership stays Rust |

The current server layer is intentionally not claimed to already satisfy the
target folder layout. Later work may introduce `server/application/`,
`server/domain/`, and `server/infrastructure/` only alongside a concrete
responsibility migration.

## Frozen public contracts

### Web/API and shared data

- API route paths and HTTP methods under `server/api/` remain unchanged.
- Server-shaped joined responses remain server-shaped; clients must not replace
  them with multi-fetch orchestration for one screen.
- `shared/types/chat.ts` remains the contract for `UIMessage`, provider/model,
  MCP server/tool, approval, workspace, and conversation values. In particular,
  conversation modes are `chat | agent`, reasoning effort is `low | medium |
  high | max`, and MCP tool IDs are `<serverId>.<name>`.
- Provider types remain `openai_compatible | anthropic_compatible | vertex_ai`;
  secrets and internal persistence columns must not enter DTO projections.
- Workspace paths remain relative to the configured workspace root and are not
  widened into unrestricted filesystem access.
- Chat streaming, assistant persistence, continuation, approval/resume,
  bounded-history/context-summary, and measured-token behavior are preserved.

### MCP and relay

- The reviewed tool catalog is frozen in
  [`029-tool-catalog-v1.json`](029-tool-catalog-v1.json): `terminal_exec`,
  `http_fetch`, and `web_search`, including schemas, annotations, and the
  `relay.coding` OAuth security scheme.
- The relay serves stateless Streamable HTTP at `POST /mcp` with `/health` as
  the probe. Current MCP protocol is `2026-07-28`; legacy negotiation values
  remain `2025-06-18`, `2025-03-26`, and `2024-11-05`.
- JSON-RPC/MCP result envelopes, `resultType: "complete"`, tool error
  semantics (`isError: true` inside a completed tool result), protocol/header/
  `_meta` validation, and server-info metadata remain compatible.
- Authorization, issuer/audience/resource/owner/scope checks, trusted-proxy
  decisions, execution-root containment, Bubblewrap isolation, timeout/process
  cleanup, SSRF redirect checks, and output limits remain fail-closed and in
  their current ordering. Client annotations or confirmation UI are not
  security controls.
- Relay production support remains Linux + Bubblewrap; Docker and host-socket
  access are out of scope.

## Duplication ledger

| Category | Current evidence | Candidate later phase | Constraint |
| --- | --- | --- | --- |
| Knowledge duplication | chat mode/reasoning/model capability concepts appear in both chat pages and related controls; provider/base-URL policy appears across provider helpers and routes | Phase 1 | centralize only same-concept source-of-truth; preserve screen-specific copy |
| Knowledge duplication | shared chat/provider/workspace concepts are consumed by client and server | Phase 1 | shared modules stay runtime-neutral |
| Behavior duplication | `default.vue` owns several sidebar/workspace/conversation actions; `chat.post.ts` owns several chat-turn stages; Rust transport/execution each own multiple independent gates | Phases 2, 6, 10, 11 | extract by reason-to-change, not line count; preserve ordering and lifecycle |
| Behavior duplication | model/provider settings lists share some modal/action lifecycle but have different domain forms | Phase 8 or later | extract only a genuinely identical contract; no flag-heavy generic editor |
| Presentation-only duplication | chat page controls and message actions have similar visual patterns | Phases 3–4 | reusable component only with stable props/events; side effects stay outside |

This ledger deliberately records candidates, not approved abstractions. Every
future extraction must name a concrete caller and contract before editing.

## Hotspot responsibility map

- `app/layouts/default.vue`: shell composition, sidebar grouping/navigation,
  workspace and conversation mutations, search, shortcuts, account/logout,
  and modal state. Treat as a composition root; do not split by file size alone.
- `app/pages/chat/index.vue`: new-chat workspace restore, editor, model/mode/
  effort/tool configuration, creation, and pending-prompt handoff.
- `app/pages/chat/[id].vue`: existing-chat loading, configuration mutation,
  editor, message actions, approvals, and rendering.
- `app/composables/useConversationChat.ts`: AI SDK transport, seed lifecycle,
  terminal/approval flow, durable ledger, store mirror, persistence/refetch,
  and error handling.
- `server/api/chat.post.ts`: request/session handling, DB reads/writes,
  turn mutation, context/workspace resolution, tool composition, approval
  policy, streaming branches, token accounting, and assistant persistence.
- `server/utils/providers.ts`, `server/utils/models.ts`, and `server/utils/`
  broadly: persistence, policy, integrations, telemetry, filesystem, and chat
  concerns are currently co-located and are migration hotspots.
- `packages/rust-tools/src/relay_agent/transport.rs`: HTTP router/middleware,
  headers, auth/JWKS integration, admission/observability, dispatch, and
  handlers. `execution.rs` combines dispatch, validation, translation, and
  process invocation. `mcp.rs` is the protected protocol/pure-logic boundary.

## Initial enforceable dependency rules

These rules are the first architectural target; they are intentionally not
enabled as broad lint restrictions until the corresponding migration exists:

1. `shared/**` may not import `server/**`, Nuxt/Nitro server runtime, Drizzle,
   provider SDKs, filesystem implementations, or Vue application state.
2. `app/**` may not import server infrastructure or database schema directly;
   presentation components do not own persistence or policy.
3. `server/api/**` may depend on application functions and transport helpers,
   but migrated routes should not own multi-step persistence/provider
   orchestration or import feature Drizzle tables directly.
4. Future `server/domain/**` must remain independent of H3/Nitro, Drizzle,
   provider SDKs, filesystem, and OAuth/JWKS implementations.
5. `relay_agent::mcp` remains transport-independent; HTTP concerns stay in
   transport, and execution/security gates retain explicit ordering.

## Baseline confirmation

- No product feature is included in this refactor inventory.
- No runtime source, dependency, schema, migration, or generated file was
  changed for Phase 0.
- Pre-existing working-tree edits were present in Rust relay files,
  `scripts/phase4-black-box.sh`, and agent sync state; they are unrelated and
  were preserved.

## Phase 0 verification

The narrow Phase 0 checks pass: `git diff --check`, all six Phase 0 checklist
items are checked, the frozen tool-catalog contract exists, and the referenced
source boundaries exist. No runtime or dependency-manifest check is required
for this documentation-only phase.

The repository-wide `pnpm verify:commit` gate remains blocked by pre-existing
repository state and was not “fixed” here: `scripts/check-agent-docs.sh`
rejects the already-tracked vendor-specific `.external-mcp` path, and the gate's
dependency-status step cannot resolve the existing manifest requirement
`@opentelemetry/sdk-node@^2.10.0` because that version is not published.
These failures are outside Phase 0's scope. Phase 0 itself is complete.
