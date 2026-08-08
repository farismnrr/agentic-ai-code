---
name: 012-mcp-inbound-sse-transport
description: The inbound MCP server at server/api/mcp/index.ts uses SSE transport with an in-memory session Map, not Streamable HTTP as originally planned — single-instance only.
metadata:
  type: project
---

`server/api/mcp/index.ts` (plan [[012-mcp-api-key]]) uses the MCP SDK's `SSEServerTransport`, not `StreamableHTTPServerTransport` as the plan originally called for. Session state (`sessionId -> transport`) lives in a module-scoped `Map`.

**Why:** SSE was what actually shipped and got live-verified end to end (curl handshake → `initialize` → `tools/list` → `tools/call`, cross-checked against the REST API and a second user for IDOR). Streamable HTTP wasn't attempted in Phase 1.

**How to apply:**
- This only works for a single Nitro worker/instance. A session opened against one process is invisible to any other — no shared session store. Fine for the current single-operator deployment; would break the moment this runs behind more than one instance (serverless, multi-replica).
- If horizontal scaling is ever needed, either move session state to Redis/Postgres, or migrate to `StreamableHTTPServerTransport` (stateless per-request, no session Map needed at all) — check the SDK's current API first, since `Server`/`SSEServerTransport` are flagged deprecated in the SDK's own `.d.ts` in favor of `McpServer`.
- Don't "fix" this by adding a heartbeat/reconnect layer on top of the `Map` — that's solving the wrong problem. The fix is either shared state or a stateless transport, not making the in-memory map more resilient.
