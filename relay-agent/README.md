# Relay Agent

Relay service for Masih Awam AI Code MCP connectivity.

## Current scope

This milestone establishes only the Relay-side authentication handoff skeleton:

- `GET /auth/login` redirects to the configured SSO GitHub login route;
- the redirect includes a Relay-owned `return_to` target derived from `RELAY_PUBLIC_URL`;
- `GET /auth/callback` requires an `assertion` query parameter;
- raw callback input is never trusted as a user identity;
- assertion verification and Relay session issuance are intentionally not implemented yet.

The existing SSO service currently ignores `return_to` after GitHub OAuth. Issuing a short-lived signed Relay assertion and redirecting back to Relay are separate follow-up work.

## Configuration

```text
PORT=3100
SSO_BASE_URL=https://sso.farismnrr.com
RELAY_PUBLIC_URL=http://localhost:3100
```

`RELAY_PUBLIC_URL` is the only source used to build the callback target. The login route does not accept a user-controlled callback URL.

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md).
