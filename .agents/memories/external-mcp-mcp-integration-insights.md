# external MCP client Native MCP Integration Insights

This note is superseded in the key transport and discovery assumptions by Plan 029 Phase 0.

The current relay baseline is stateless MCP `2026-07-28` over `POST /mcp`, with `/.well-known/oauth-protected-resource` as the relay-owned discovery surface and Remote-mode JWT validation backed by JWKS.

The stale SSE requirement from older notes is no longer the working assumption for this repo. Preserve that distinction when planning external MCP client integration work.
