# Relay Agent Architecture

The Relay service follows Clean Architecture and treats a connected account as a separate lifecycle from tool execution.

## Layers

- `interfaces/` owns HTTP routing, MCP/discovery JSON, redirects, query parsing, status codes, and cache headers.
- `application/` owns connection start, callback, status, and narrow ports.
- `application/shared/` owns application contracts and errors reused across use cases. Feature-specific behavior must not be moved there.
- `domain/` owns verified principals and connection state.
- `infrastructure/` owns environment config, random tokens, in-memory connection storage, SSO URL construction, signed assertion verification, and MCP access-token verification.
- `bootstrap/` is the composition root.

Dependency direction points inward. Domain and application do not depend on Axum, Tokio, URL parsing, crypto libraries, persistence implementations, or HTTP details.

Backend layer roots must stay small. Split growth by feature/responsibility; generic `utils`, `helpers`, `common`, and `misc` source folders are forbidden. `shared/` is reserved for code with concrete reuse across modules.

## Test layout

Rust tests stay outside production `src/`. Relay integration and contract tests live in top-level `tests/`. Inline/co-located test bodies in `src/` are rejected by the structural guardrail.

## Discover + connect flow

1. A client discovers Relay connection metadata at `/.well-known/relay.json`.
2. OpenAI-compatible MCP clients discover OAuth protected-resource metadata from `/.well-known/oauth-protected-resource` or the resource-specific `/.well-known/oauth-protected-resource/mcp` URL advertised by the challenge.
3. `/connections/start` creates a pending server-side connection with a high-entropy state.
4. Relay redirects to the configured SSO `/auth/github` endpoint with client ID `relay-agent` and the opaque state.
5. SSO resolves that client ID from its dashboard-managed SQLite registry.
6. SSO completes its existing GitHub OAuth and allowlist checks.
7. SSO signs a short-lived assertion using the registered client ID as audience and the registered TTL.
8. SSO redirects only to the callback URL stored in SQLite.
9. Relay verifies the assertion and consumes a matching pending state exactly once.
10. The connection becomes `connected` and can be read through `/connections/{id}`.

Relay never supplies a return/callback URL to SSO.

## MCP + OAuth discovery

Relay exposes the MCP resource at `/mcp`, declares per-tool OAuth `securitySchemes`, and returns a Bearer `WWW-Authenticate` challenge that advertises protected-resource metadata. SSO publishes OAuth authorization-server metadata with PKCE `S256`, the `resource` parameter contract, CIMD support, and RFC 9207 issuer identification used by current OpenAI clients.

The shared `contracts/chatgpt-discovery.json` contract keeps Relay and SSO aligned on issuer, resource, scope, ChatGPT CIMD client ID, redirect URI, and protocol version.

## Security boundary

Raw callback input is never identity. A principal exists only after signature, issuer, audience, time, and pending-state validation. Pending connections are kept only in memory and expire; process restart invalidates them. Callback and status responses use `Cache-Control: no-store`.

The signed assertion is a handoff credential, not a Relay session. Relay session issuance remains intentionally absent.

## Configuration boundary

Relay's current connected-app identity is fixed to `relay-agent`. Audience, callback URL, enabled state, display metadata, and assertion TTL live in the SSO SQLite registry and are managed from the dashboard.

Relay environment configuration is limited to service addresses, port, and the shared HMAC signing secret. The pending connection attempt TTL is an internal default.
