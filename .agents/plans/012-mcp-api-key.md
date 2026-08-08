# 012 — API keys: expose our own MCP server, and actually use the ones we store

## Context

Two things are currently missing:

1. **Inbound.** There's no way for an external MCP client (external MCP client Desktop, another agent, etc.) to drive this app. No API keys exist at all today — auth is 100% session-cookie (`nuxt-auth-utils`).
2. **Outbound.** `mcp_servers` (schema.ts:145) already stores third-party server configs (name/transport/url/command) with full CRUD (`server/api/mcp-servers/*`), and the chat UI has `enabledToolIds`/`approvals` columns on `conversations` ready for tool use — but nothing actually connects to a stored MCP server or calls its tools. `chat.post.ts` only proxies to the router's `/chat/completions`, no tools included. It's a settings form that saves rows nobody reads.

Confirmed with the user: build both. Inbound MCP server runs as an HTTP endpoint inside this Nuxt app (idiomatic — Nitro already serves everything else here), not a separate stdio process.

## Decisions

- **Inbound auth**: new `api_keys` table, one-way hashed (SHA-256, same pattern as `verification_tokens.tokenHash`), scoped to a user. Presented once at creation, never stored/shown again — same UX as most SaaS API key flows. Sent as `Authorization: Bearer <key>` on the MCP endpoint only; session cookies remain the only auth for the existing browser-facing REST API.
- **Inbound transport**: MCP TypeScript SDK (`@modelcontextprotocol/sdk`), SSEServerTransport (Server-Sent Events) mounted at `server/api/mcp/index.ts`. Note: This deviates from the originally planned Streamable HTTP transport. Sessions are currently stored in a module-scoped in-memory `Map`, which is pragmatic for a single-operator deployment but will not survive across multi-worker Nitro scale-outs (would need Redis/DB state later).
- **Inbound tool scope** (phase 1, confirmed): settings (read/update), workspaces (list/create/update/delete), MCP servers (list/create/update/delete — meta), chat (send a message to a conversation, read conversation messages). *Update: all 10/10 tools implemented and live-verified, including `send_message`/`list_messages` (backed by the new `server/utils/messages.ts`, shared with `conversations/[id].get.ts`).*
- **Outbound wiring**: when a conversation has `enabledToolIds` referencing enabled rows in `mcp_servers`, `chat.post.ts` connects to each (HTTP or stdio per `transport`), lists their tools, passes them to the router's `/chat/completions` call as OpenAI-style `tools`, and executes any tool call the model returns before continuing the stream — respecting the existing `approvals` map (`always`/`never`) and falling back to the SDK's `tool-approval-request` flow (see `.agents/memories/ai-sdk-native-features.md`) when neither applies.
- **Scope boundary**: outbound MCP client connections are per-request, not persistent background connections — no daemon, no reconnect logic. `status`/`tools` on `mcp_servers` refresh opportunistically (on use, and via a manual "test connection" action), not via a polling job.
- **Security Warning (Phase 2)**: The outbound `stdio` transport spawns arbitrary commands from `mcp_servers.command`. Currently, `create_mcp_server` accepts any command from an external client, creating an RCE vulnerability in a multi-tenant setup. Phase 2 MUST address this (e.g. whitelist commands, disable stdio temporarily, or require strict admin approval).

## Changes

### Phase 1 — API keys + inbound MCP server

1. **`server/database/schema.ts`** — new `apiKeys` table: `id` (uuid pk), `userId` (fk, cascade), `name` (text, user-given label), `keyHash` (text, unique, sha-256 hex), `keyPrefix` (text, first 8 chars shown in the UI list so users can tell keys apart without re-revealing them), `lastUsedAt` (nullable timestamp), `createdAt`. Migration via `drizzle-kit generate`.
2. **`server/utils/api-key.ts`** — `generateApiKey()` (crypto-random, prefixed e.g. `aic_live_...`), `hashApiKey()` (sha-256, reuse the pattern in `server/utils/token.ts`), `verifyApiKey(event)` — reads `Authorization: Bearer`, hashes, looks up `apiKeys` by `keyHash`, returns the owning user or throws `unauthorized`. Bumps `lastUsedAt`.
3. **`server/api/api-keys/index.get.ts`** — session-gated, lists the user's keys (name, prefix, lastUsedAt, createdAt — never the hash or raw key).
4. **`server/api/api-keys/index.post.ts`** — session-gated, creates a key, returns the **raw key once**.
5. **`server/api/api-keys/[id].delete.ts`** — session-gated, revokes a key.
6. **Settings UI** — new section in `app/pages/settings/` (check existing structure/tabs first) to list, create, and revoke API keys, with a one-time "copy this now, it won't be shown again" reveal on creation. Follow existing settings page patterns and semantic color tokens.
7. **`pnpm add @modelcontextprotocol/sdk`**, then `server/api/mcp/index.ts` — Streamable HTTP transport, auth via `verifyApiKey`, registers the tools below via the SDK's `server.tool(...)`.
8. **MCP tools**, each backed by shared `server/utils/` logic (not raw HTTP calls to our own REST API):
   - `get_settings`, `update_settings`
   - `list_workspaces`, `create_workspace`, `update_workspace`, `delete_workspace`
   - `list_mcp_servers`, `create_mcp_server`, `update_mcp_server`, `delete_mcp_server`
   - `send_message` (conversationId, text — reuses `chat.post.ts`'s send path but returns the full assistant reply rather than streaming, since MCP tool calls are request/response), `list_messages` (conversationId)
9. Refactor REST handlers that now share logic with a tool (e.g. `workspaces/index.post.ts`'s create-and-validate path) to call the same `server/utils/` function, so there's exactly one implementation of each operation. **Done for settings, workspaces, mcp-servers, and messages** — `conversations/[id].get.ts` now calls `listConversationMessages()` from `server/utils/messages.ts` instead of duplicating the query.

**Phase 1 status: complete, live-verified, ready for PR.** `pnpm build`, `vue-tsc --noEmit -p .nuxt/tsconfig.json`, `pnpm lint`, `pnpm audit` all clean. Curl-tested end to end against a running dev server with two real registered users: SSE handshake, `initialize`, `tools/list` (all 10 tools), and `tools/call` for `get_settings`, `create_workspace`, `send_message` (hit the real 9Router model and got a real reply), `list_messages` (output matched `GET /api/conversations/:id` exactly), unknown-tool error path, revoked-key rejection, and a cross-user IDOR attempt (`delete_workspace` with another user's workspace id correctly returned "not found" and left the row untouched).

### Phase 2 — Outbound: actually use stored MCP servers in chat

10. **`server/utils/mcp-client.ts`** — given an `mcpServers` row, opens a client connection (`StreamableHTTPClientTransport` for `http`/`url` rows, `StdioClientTransport` for `command` rows), lists tools, closes on completion. Per-request, not pooled.
11. **`chat.post.ts`** — before calling the router, resolve `conv.enabledToolIds` to enabled `mcp_servers` rows, connect via step 10, merge their tool schemas into the `/chat/completions` request body. When the router returns a tool call, check `conv.approvals[toolName]`; if `always`/`never` isn't set, emit the SDK's `tool-approval-request` chunk and wait for `addToolApprovalResponse` before executing. Execute approved calls through the same MCP client, feed results back to the router, continue streaming.
12. **`mcp-servers/[id].put.ts` or a new `test.post.ts`** — "test connection" action: connects via step 10, updates `status`/`tools` on the row, surfaces errors to the UI instead of failing silently.

## Out of scope

- No OAuth-based MCP client auth (only `Authorization: Bearer` API keys) — matches this app's existing auth story, revisit if a third-party server requires it.
- No persistent/pooled outbound MCP connections or reconnect/health-check daemon — per-request only, per the scope boundary above.
- No fine-grained per-key scopes/permissions (a key can do anything its owning user can) — single scope tier for now.
- No rate limiting on the MCP endpoint beyond whatever `server/utils/rate-limit.ts` already provides elsewhere — reuse it, don't build a second mechanism.

## Verification

- `pnpm lint`, `pnpm typecheck`, `pnpm audit` clean on every phase.
- Live test: create an API key, call `POST /api/mcp` (or use an actual MCP client, e.g. `mcp-inspector` or external MCP client Desktop config) to list tools, call `update_settings` and `create_workspace`, confirm both show up through the normal browser session afterward.
- Live test: a revoked key is rejected immediately.
- Live test: enable a real MCP server (e.g. a local filesystem or GitHub MCP server) on a conversation, send a message that requires a tool call, confirm the approval flow fires and the tool result reaches the model.
- `/security-review` before opening each phase's PR — this plan adds a new unauthenticated-by-cookie surface (the MCP endpoint) and a credential-issuing flow, both classic targets.

## On completion — Phase 1

- [x] Plan file updated with status and real verification results.
- [x] Stays in the In Flight list in `.agents/plans/README.md` — Phase 2 (outbound wiring) hasn't started, so the plan as a whole isn't done yet. Move to Completed only once both phases have shipped.
- [x] `.agents/memories/012-mcp-inbound-sse-transport.md` written — records the SSE-vs-Streamable-HTTP deviation and the in-memory session Map's scaling limit, and indexed in `memories/README.md`.
- [x] Branch pushed, PR opened against `dev` (`feat/012-p1-mcp-api-key-phase1`), CI green, merged as `13ae69c` (PR #45, squash-merged), branch/worktree cleaned up per `.agents/knowledge/git.md`.

## On completion — Phase 2 (not started)

- [ ] Outbound wiring (steps 10-12) implemented and live-tested against a real third-party MCP server.
- [ ] The stdio-transport RCE risk flagged in Decisions above resolved one way or another (whitelist, disable, or explicit approval gate) before this ships — not carried forward silently.
- [ ] Plan moved to Completed in `.agents/plans/README.md` once this phase's PR merges.
