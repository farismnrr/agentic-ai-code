# Relay Agent Architecture

The Relay service follows Clean Architecture and treats a connected account as a separate lifecycle from future tool execution.

## Layers

- `interfaces/` owns HTTP routing, discovery JSON, redirects, query parsing, status codes, and cache headers.
- `application/` owns connection start, callback, and status use cases plus narrow ports.
- `domain/` owns verified principals and connection state.
- `infrastructure/` owns environment config, random tokens, in-memory connection storage, SSO URL construction, and signed assertion verification.
- `bootstrap/` is the composition root.

Dependency direction points inward. Domain and application do not depend on Axum, Tokio, URL parsing, crypto libraries, persistence implementations, or HTTP details.

## Discover + connect flow

1. A client discovers Relay at `/.well-known/relay.json`.
2. `/connections/start` creates a pending server-side connection with a high-entropy state.
3. Relay redirects to the configured SSO `/auth/github` endpoint with its client ID and the opaque state.
4. SSO resolves that client ID from its dashboard-managed connected-app registry.
5. SSO completes its existing GitHub OAuth and allowlist checks.
6. SSO signs a short-lived assertion using the registered client ID as audience and the registered TTL.
7. SSO redirects only to the callback URL stored in the registry.
8. Relay verifies the assertion and consumes a matching pending state exactly once.
9. The connection becomes `connected` and can be read through `/connections/{id}`.

Relay never supplies a return/callback URL to SSO.

## Security boundary

Raw callback input is never identity. A principal exists only after signature, issuer, audience, time, and pending-state validation. Pending connections are kept only in memory and expire; process restart invalidates them. Callback and status responses use `Cache-Control: no-store`.

The signed assertion is a handoff credential, not a Relay session. Relay session issuance remains intentionally absent.

## Configuration boundary

Relay defaults to client ID `relay-agent`; an optional `RELAY_CLIENT_ID` override can change it. That value must match the Client ID / audience registered in the SSO dashboard.

The HMAC signing secret remains environment-provided on both services. App callback, enabled state, display metadata, and assertion TTL are not Relay or SSO environment configuration.

## Deferred layers

Tool discovery, MCP execution, per-tool permissions, approval policy, durable Relay connection persistence, Relay sessions, and federated logout are future phases and must remain separate from this connection foundation.
