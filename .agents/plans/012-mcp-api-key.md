# 012 — API keys: expose our own MCP server, and actually use the ones we store

## Context

Two things are currently missing:

1. **Inbound.** There's no way for an external MCP client (Claude Desktop, another agent, etc.) to drive this app. No API keys exist at all today — auth is 100% session-cookie (`nuxt-auth-utils`).
2. **Outbound.** `mcp_servers` (schema.ts:145) already stores third-party server configs (name/transport/url/command) with full CRUD (`server/api/mcp-servers/*`), and the chat UI has `enabledToolIds`/`approvals` columns on `conversations` ready for tool use — but nothing actually connects to a stored MCP server or calls its tools. `chat.post.ts` only proxies to the router's `/chat/completions`, no tools included. It's a settings form that saves rows nobody reads.

Confirmed with the user: build both. Inbound MCP server runs as an HTTP endpoint inside this Nuxt app (idiomatic — Nitro already serves everything else here), not a separate stdio process.

## Decisions

- **Inbound auth**: new `api_keys` table, one-way hashed (SHA-256, same pattern as `verification_tokens.tokenHash`), scoped to a user. Presented once at creation, never stored/shown again — same UX as most SaaS API key flows. Sent as `Authorization: Bearer <key>` on the MCP endpoint only; session cookies remain the only auth for the existing browser-facing REST API.
- **Inbound transport**: MCP TypeScript SDK (`@modelcontextprotocol/sdk`), SSEServerTransport (Server-Sent Events) mounted at `server/api/mcp/index.ts`. Note: This deviates from the originally planned Streamable HTTP transport. Sessions are currently stored in a module-scoped in-memory `Map`, which is pragmatic for a single-operator deployment but will not survive across multi-worker Nitro scale-outs (would need Redis/DB state later).
- **Inbound tool scope** (phase 1, confirmed): settings (read/update), workspaces (list/create/update/delete), MCP servers (list/create/update/delete — meta), chat (send a message to a conversation, read conversation messages). *Update: all 10/10 tools implemented and live-verified, including `send_message`/`list_messages` (backed by the new `server/utils/messages.ts`, shared with `conversations/[id].get.ts`).*
- **Outbound wiring**: implemented via the AI SDK's own tool-calling primitives, not hand-rolled SSE parsing — `chat.post.ts` uses `@ai-sdk/openai-compatible` to address 9Router as a real `LanguageModel`, resolves `conv.enabledToolIds` into `ai@7` `tool()` objects backed by `server/utils/mcp-client.ts` MCP connections (`server/utils/mcp-tools.ts`), and passes `conv.approvals` as `streamText`'s `toolApproval` map (`'always'` → `'approved'`, `'never'` → `'denied'`, unset → `'user-approval'`). This is what makes multi-tool-call handling, the approval-pause/resume flow, and continuing the model with tool results all come from the SDK instead of being reimplemented — matches `.agents/memories/ai-sdk-native-features.md`'s standing warning against parallel state machines, and is what the already-built `app/components/ChatToolApproval.vue` (which reads the real `approval-requested` SDK state) was expecting all along.
- **Scope boundary**: outbound MCP client connections are per-request, not persistent background connections — no daemon, no reconnect logic. `status`/`tools` on `mcp_servers` refresh opportunistically (on use, and via `POST /api/mcp-servers/:id/test`), not via a polling job.
- **Security resolution (Phase 2, was open in Phase 1)**: the `stdio` transport is disabled outbound — `server/utils/mcp-client.ts` throws for any `mcp_servers` row with `transport: 'stdio'` instead of spawning it. Any authenticated user (including via the inbound `create_mcp_server` MCP tool) can still *store* a `stdio` row, but it fails closed the moment anything tries to connect to it. Chosen over allow-listing (no safe allow-list exists for arbitrary executable commands) or an admin-approval gate (no admin role exists in this app yet — would be new scope). Revisit if stdio support is ever actually needed; the schema (`McpTransport`) still allows the value, only the outbound connector rejects it.

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

10. **`server/utils/mcp-client.ts`** — given an `mcpServers` row, opens a client connection (`StreamableHTTPClientTransport` for `http`, `SSEClientTransport` for `sse`). `stdio` rows are rejected outright (see Security resolution above) rather than spawned. Per-request, not pooled.
11. **`server/utils/mcp-tools.ts`** — resolves a conversation's `enabledToolIds` against the user's enabled `mcp_servers` rows, connects each via step 10, and builds an `ai@7` `ToolSet` (`tool({ description, inputSchema: jsonSchema(...), execute })` per MCP tool) plus a matching `toolApproval` map from `conv.approvals`. Tool names are sanitized (`serverId.toolName` → `serverId_toolName`, truncated to 64 chars) since OpenAI-shaped tool names can't contain dots.
12. **`server/utils/router-model.ts`** — wraps `@ai-sdk/openai-compatible`'s `createOpenAICompatible(...).chatModel(modelId)` pointed at `routerBaseUrl`/`routerApiKey`, giving `streamText` a real `LanguageModel` for 9Router instead of a hand-rolled `fetch` + SSE parser.
13. **`chat.post.ts`** rewritten around `streamText({ model, tools, toolApproval, stopWhen: stepCountIs(5), messages: convertToModelMessages(...) })`, then `toUIMessageStream({ stream: result.stream, tools, originalMessages, onEnd })` → `createUIMessageStreamResponse`. `onEnd`'s `isContinuation` flag decides whether to `UPDATE` the last assistant row (an approval-response resuming the same in-flight message) or `INSERT` a new one — matches how `useChat()`'s `addToolApprovalResponse` re-sends the full history rather than starting a new request cycle.
14. **`server/api/mcp-servers/[id]/test.post.ts`** — "test connection" action: connects via step 10, updates `status`/`tools` on the row, surfaces errors to the UI instead of failing silently. (Note the nested-folder path — `[id].test.post.ts` as a flat filename maps to route `/api/mcp-servers/:id.test`, not `/api/mcp-servers/:id/test`, in this Nitro version; a sub-resource route needs `[id]/test.post.ts`.) `createMcpServer`'s default `status` also changed from an optimistic `'connected'` to `'disconnected'`, since this endpoint is now what actually earns that value.

## Out of scope

- No OAuth-based MCP client auth (only `Authorization: Bearer` API keys) — matches this app's existing auth story, revisit if a third-party server requires it.
- No persistent/pooled outbound MCP connections or reconnect/health-check daemon — per-request only, per the scope boundary above.
- No fine-grained per-key scopes/permissions (a key can do anything its owning user can) — single scope tier for now.
- No rate limiting on the MCP endpoint beyond whatever `server/utils/rate-limit.ts` already provides elsewhere — reuse it, don't build a second mechanism.

## Verification

- `pnpm build`, `vue-tsc -p .nuxt/tsconfig.json --noEmit`, `pnpm lint`, `pnpm audit` clean on every phase (see `.agents/memories/007-typecheck-gate-was-silent.md` for why `pnpm typecheck` alone isn't enough).
- Live test: create an API key, call `POST /api/mcp` (or use an actual MCP client, e.g. `mcp-inspector` or Claude Desktop config) to list tools, call `update_settings` and `create_workspace`, confirm both show up through the normal browser session afterward.
- Live test: a revoked key is rejected immediately.
- Live test: enable a real MCP server on a conversation, send a message that requires a tool call, confirm the approval flow fires and the tool result reaches the model.
- `/security-review` before opening each phase's PR — this plan adds a new unauthenticated-by-cookie surface (the MCP endpoint) and a credential-issuing flow, both classic targets.

**Phase 2 live-verified** end to end against a real local MCP server (SSE transport, two tools: `get_time`, `add_numbers`) plus the real 9Router model, through a running dev server:
- `POST /api/mcp-servers/:id/test` connected, listed both tools, and persisted `status: 'connected'` + real tool definitions (previously defaulted to an untested `'connected'`, now correctly starts `'disconnected'`).
- **`'always'` approval**: the model called `get_time`, the SDK auto-approved it (`isAutomatic: true`), the real tool executed against the real server, and the model's final answer used the real result — all persisted as one well-formed assistant `UIMessage` with proper `tool-*` parts.
- **No approval decision set**: the model's tool call correctly paused at `tool-approval-request` (no `isAutomatic`, no auto-response) and the turn ended there waiting for a real client answer — exactly the state `app/components/ChatToolApproval.vue` already expects.
- **`'never'` approval**: confirmed at the wire level that the underlying tool was **never actually invoked** on the test server (0 calls logged) despite the model retrying the same call 5 times before `stopWhen: stepCountIs(5)` cut the turn off with no text reply. The security property (a denied tool cannot execute) holds; the UX gap (no explanation surfaces to the user when the step cap is hit on a denial) is a model/prompting quirk, not a wiring bug, and is noted as a known limitation rather than silently left undocumented.
- IDOR-relevant: all of `mcp-tools.ts`/`mcp-client.ts`/`test.post.ts` resolve `mcp_servers` rows scoped to `and(eq(id), eq(userId))`, consistent with Phase 1.

**`/security-review` (required by this plan before merging) found one real HIGH finding, fixed before merge**: `mcpServers.url` is a bare, unvalidated user-supplied string (`v.optional(v.string())`), and `createMcpClient` passed it straight into `new URL(...)` and opened a real outbound HTTP/SSE connection — any authenticated, non-privileged user could point a server at `http://169.254.169.254/...` (cloud metadata), `http://localhost:<port>`, or an RFC1918 address, then trigger the connection via `POST /api/mcp-servers/:id/test` (which echoed the raw error back) or by enabling the tool in a chat. Fixed in `server/utils/mcp-client.ts` with `assertSafeMcpUrl()`: rejects non-http(s) schemes, resolves the hostname via DNS and rejects loopback/RFC1918/link-local/cloud-metadata addresses, re-checked on every connection (not just at server registration) to also cover DNS rebinding. Live-verified: cloud-metadata IP, `localhost`, an RFC1918 address, and a `file://` scheme all correctly rejected with 400s; a real public host (`example.com`) still connects through (fails at the MCP protocol level, not the guard) — confirming the fix discriminates correctly rather than over-blocking.

## On completion — Phase 1

- [x] Plan file updated with status and real verification results.
- [x] `.agents/memories/012-mcp-inbound-sse-transport.md` written — records the SSE-vs-Streamable-HTTP deviation and the in-memory session Map's scaling limit, and indexed in `memories/README.md`.
- [x] Branch pushed, PR opened against `dev` (`feat/012-p1-mcp-api-key-phase1`), CI green, merged as `13ae69c` (PR #45, squash-merged), branch/worktree cleaned up per `.agents/knowledge/git.md`.

## On completion — Phase 2

- [x] Outbound wiring (steps 10-14) implemented and live-tested against a real third-party MCP server — see Verification above.
- [x] The stdio-transport RCE risk flagged in Phase 1 resolved: outbound connections fail closed on `transport: 'stdio'` (see Decisions → Security resolution).
- [x] `.agents/memories/012-mcp-outbound-tool-loop.md` written — records the SDK-native `streamText`/`toolApproval` architecture decision and the denial-retry live-test finding.
- [x] Plan moved to Completed in `.agents/plans/README.md` — merged as PR #47 (`3579707`).
- [x] Branch pushed, PR opened against `dev` (`feat/012-p2-mcp-outbound-v2`), CI green, `/security-review` run and its one HIGH finding (SSRF via `mcp_servers.url`) fixed before merge, merged, branch/worktree cleaned up per `.agents/knowledge/git.md`.
