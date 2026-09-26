# Relay Agent

Relay service for Masih Awam AI Code connectivity.

## Current scope

This milestone establishes the discover + connect foundation, modeled after a connected-app lifecycle:

- `GET /.well-known/relay.json` exposes Relay discovery metadata and the connection entry point.
- `GET /connections/start` creates a short-lived pending connection and redirects to the configured SSO service.
- Relay sends only its client ID and an opaque `connection_state`; it never sends a caller-controlled callback target.
- SSO resolves the client ID from its authenticated dashboard-managed app registry.
- SSO completes the existing GitHub OAuth and allowlist flow, then returns a short-lived signed assertion to the registered callback.
- `GET /connections/callback` verifies signature, issuer, audience, expiry, and the pending Relay state before marking the connection connected.
- `GET /connections/{id}` exposes the in-memory connection status/read model.
- `/auth/login` and `/auth/callback` remain compatibility aliases for the connection start/callback routes.

Relay does not create a local login/session cookie in this milestone.

## Configuration

Required Relay settings:

- `PORT=3100`
- `RELAY_PUBLIC_URL=http://localhost:3100`
- `SSO_BASE_URL=https://sso.farismnrr.com`
- `RELAY_ASSERTION_SECRET=<shared secret, at least 32 bytes>`
- `CONNECTION_ATTEMPT_TTL_SECONDS=300`

`RELAY_CLIENT_ID` is optional and defaults to `relay-agent`.

On SSO, sign in normally and register the Relay app from the Connected Apps dashboard. For local development the dashboard is prefilled with client ID `relay-agent`, callback `http://localhost:3100/connections/callback`, and a 90-second assertion TTL.

The app registry is server-side and persisted in the SSO data volume. The signing secret remains environment configuration.

## Fast local image flow

Fast AMD64 CI publishes both:

- `ghcr.io/farismnrr/agentic-ai-code-sso-auth:fast`
- `ghcr.io/farismnrr/agentic-ai-code-relay-agent:fast`

`docker-compose.fast.yml` consumes those images directly, so local development does not need to build Relay from source after CI succeeds.

## Deliberately deferred

Tool discovery, MCP execution, tool permissions, approval UI, Relay-local sessions, logout federation, durable Relay connection persistence, and deployment changes are outside this scope.

See [ARCHITECTURE.md](./ARCHITECTURE.md).
