# Relay Agent

Relay service for Masih Awam AI Code connectivity.

## Current scope

This milestone establishes the discover + connect foundation, modeled after a connected-app lifecycle:

- `GET /.well-known/relay.json` exposes Relay discovery metadata and the connection entry point.
- `GET /connections/start` creates a short-lived pending connection and redirects to the configured SSO service.
- Relay identifies itself as `relay-agent` and sends only an opaque `connection_state`; it never sends a caller-controlled callback target.
- SSO resolves `relay-agent` from its authenticated dashboard-managed SQLite registry.
- SSO completes the existing GitHub OAuth and allowlist flow, then returns a short-lived signed assertion to the registered callback.
- `GET /connections/callback` verifies signature, issuer, audience, expiry, and the pending Relay state before marking the connection connected.
- `GET /connections/{id}` exposes the in-memory connection status/read model.
- `/auth/login` and `/auth/callback` remain compatibility aliases for the connection start/callback routes.

Relay does not create a local login/session cookie in this milestone.

## Relay environment

Relay now keeps only deployment/bootstrap values in env:

- `PORT=3100`
- `RELAY_PUBLIC_URL=http://localhost:3100`
- `SSO_BASE_URL=https://sso.farismnrr.com`
- `RELAY_ASSERTION_SECRET=<shared secret, at least 32 bytes>`

The Relay client ID/audience is fixed to `relay-agent` for this service. Callback URL, enabled state, display metadata, and assertion TTL are stored in SSO SQLite and managed from the Connected Apps dashboard.

The SSO SQLite database lives at `/app-data/sso.sqlite3` and is persisted by the compose data volume.

## Fast local image flow

Fast AMD64 CI publishes both:

- `ghcr.io/farismnrr/agentic-ai-code-sso-auth:fast`
- `ghcr.io/farismnrr/agentic-ai-code-relay-agent:fast`

`docker-compose.fast.yml` consumes those images directly, so local development does not need to build Relay from source after CI succeeds.

## Deliberately deferred

Tool discovery, MCP execution, tool permissions, approval UI, Relay-local sessions, logout federation, durable Relay connection persistence, and deployment changes are outside this scope.

See [ARCHITECTURE.md](./ARCHITECTURE.md).
