# Plan 035 Lane 6 — Remediation re-proof: Error path D (MCP tool-result failure, lane-4 fix)

## Steps performed (real, live traffic against `http://localhost:3334`)

1. Created an API key via `POST /api/api-keys` for the test account.
2. `GET /api/mcp` with `Authorization: Bearer <key>` (SSE) -> received
   `event: endpoint` / `data: /api/mcp?sessionId=04107304-c1ff-4312-ba66-7ad7980503b8`.
3. `POST /api/mcp?sessionId=<id>` with a real `tools/call` JSON-RPC request naming an unknown tool
   (`nonexistent_tool_xyz`) -> `202 Accepted` (JSON-RPC ack), with the actual tool-call result
   delivered asynchronously over the open SSE stream.

## Client-visible result (delivered on the SSE stream)

```
event: message
data: {"result":{"content":[{"type":"text","text":"Tool execution failed"}],"isError":true},"jsonrpc":"2.0","id":1}
```

Exactly matches the lane-4 fix's contract: generic `"Tool execution failed"` text, `isError: true`,
delivered as a normal (HTTP-200-class/JSON-RPC-success-envelope) SSE message — no tool name, no
error class, no stack, no filesystem/DB detail leaked to the client.

## Correlated private Loki record

Query: `{job="ai-code-server"} | json`, filtered to the request window:

```json
{
  "message": "mcp.tool.call",
  "attributes": {
    "service.name": "ai-code-server",
    "error.type": "Error",
    "error.message": "Unknown tool: nonexistent_tool_xyz",
    "request.id": "3d3004b2-a358-4d34-a58f-c8c7c45cf00e",
    "operation": "mcp.tool.call",
    "outcome": "error",
    "error.code": "mcp_tool_call_failed",
    "trace_id": "33dda806dc21c3874d21e345e8ae16f3",
    "span_id": "db0b76fd666f02ab"
  }
}
```

`event.name`/`operation: "mcp.tool.call"` and `error.code: "mcp_tool_call_failed"` are present
exactly as specified, with the real diagnostic (`Unknown tool: nonexistent_tool_xyz`) captured
privately — precisely the pattern `server/api/mcp/index.ts:96-105`'s handler comment describes
("mirror the same public/private split by hand"). Full raw Loki response saved as
`remediation-error-path-mcp-loki-raw.json`; full raw SSE transcript saved as
`remediation-error-path-mcp-sse-response.txt`.

## Verdict: PASS

Lane 4's MCP tool-result confidentiality fix re-confirmed with real live traffic and real Loki
correlation on the remediated commit — generic client-visible error, detailed private telemetry,
correct `error.code`/`operation` tagging.
