# Plan 029 freezes the ChatGPT MCP target around stateless `POST /mcp`, Auth0-backed OAuth, and `relay.coding`.

Plan 029 should stay anchored to the current MCP `2026-07-28` transport and the relay's existing Resource Server model instead of reviving the old SSE story.

## Durable decisions

- ChatGPT write-capable E2E should target a Business workspace with developer mode enabled and a custom MCP app, but only if the live tenant actually exposes full MCP write/modify actions.
- If the selected Business tenant does not expose write/modify actions, that is a blocker, not something to paper over with a read-only confirmation flow.
- The external OAuth/OIDC provider is frozen as Auth0, using a user-defined OAuth client as the baseline registration mode.
- The relay stays a JWKS-backed Resource Server and does not grow its own OAuth client-registration database.
- The canonical MCP resource identifier is the externally reachable HTTPS `/mcp` URL, not localhost and not an SSE message endpoint.
- `relay.coding` is the default full-coding resource scope for the complete toolset, including `terminal_exec`.
- Optional narrow scopes are not worth supporting in the first production profile because they add complexity without materially isolating a toolset that already includes terminal execution.
- Current ChatGPT setup facts that are exact: developers create/test/deploy MCP apps in developer mode; setup asks for an endpoint and required metadata; OAuth flows require callback URL configuration when applicable; app permissions and Action control influence when ChatGPT asks before using actions.
- Current ChatGPT setup facts that are only inferred until captured from the live UI: literal field labels, field ordering, conditional visibility, the connector callback URI, CIMD/DCR selectors, and any auto-discovered values.

## Rationale

- A single explicit `relay.coding` grant is more honest than pretending `terminal_exec` can be tightly partitioned into weaker OAuth scopes.
- Auth0 is a pragmatic baseline because it provides a conventional external Authorization Server shape and keeps the relay out of client-registration complexity.
- The protected-resource `resource` value must match the actual public relay endpoint so issuer/audience/resource checks stay consistent.
- The prior SSE assumption is stale for this plan: the relay already implements stateless MCP over `POST /mcp`, and `/.well-known/oauth-protected-resource` is the discovery surface it owns.
