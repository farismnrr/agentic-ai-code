# Relay Agent

Relay service for Masih Awam AI Code connectivity.

## Current scope

This milestone establishes the discover + connect foundation, modeled after a connected-app lifecycle:

- `GET /.well-known/relay.json` exposes Relay discovery metadata and the connection entry point.
- `GET /connections/start` creates a short-lived pending connection and redirects to the configured SSO service.
- Relay sends only an opaque `connection_state`; it never sends a caller-controlled callback target.
- SSO completes the existing GitHub OAuth and allowlist flow, then returns a short-lived signed assertion to the configured Relay callback.
- `GET /connections/callback` verifies signature, issuer, audience, expiry, and the pending Relay state before marking the connection connected.
- `GET /connections/{id}` exposes the in-memory connection status/read model.
- `/auth/login` and `/auth/callback` remain compatibility aliases for the connection start/callback routes.

Relay does not create a local login/session cookie in this milestone.

## Configuration

Required Relay settings:

- `PORT=3100`
- `RELAY_PUBLIC_URL=http://localhost:3100`
- `SSO_BASE_URL=https://sso.farismnrr.com`
- `RELAY_AUDIENCE=relay-agent`
- `RELAY_ASSERTION_SECRET=<shared secret, at least 32 bytes>`
- `CONNECTION_ATTEMPT_TTL_SECONDS=300`

The SSO service owns the trusted Relay callback through `RELAY_CALLBACK_URL`; Relay no longer supplies `return_to`.

## Deliberately deferred

Tool discovery, MCP execution, tool permissions, approval UI, Relay-local sessions, logout federation, durable connection persistence, and deployment changes are outside this scope.

See [ARCHITECTURE.md](./ARCHITECTURE.md).
