# Architecture

AI Code has two primary runtime components and several external dependencies.

```text
Browser
  |
  | HTTPS / session cookie
  v
Nuxt / Nitro application
  |-- PostgreSQL               users, workspaces, conversations, providers, MCP config
  |-- model providers          AI generation
  |-- SMTP / Google / GitHub   application login options
  |-- Loki / Jaeger            optional telemetry
  |
  | MCP client
  v
Public MCP resource  <---------------- external MCP client / other MCP clients
  |
  | HTTPS edge / outbound tunnel
  v
127.0.0.1:47821
ai-tools relay
  |
  | OAuth resource-server validation + Bubblewrap
  v
owner-scoped coding filesystem / subprocesses
```

## Web application boundary

The Nuxt application owns:

- user sessions and application accounts;
- email/password verification and reset flows;
- optional Google/GitHub OAuth login;
- workspaces and conversations;
- model providers and encrypted provider credentials;
- MCP server configuration and tool orchestration;
- telemetry emitted by the web/server application.

The server follows a layered structure:

```text
server/api
  -> server/application
      <- server/infrastructure
```

HTTP routes compose and adapt requests, application modules own use-case semantics/contracts, and infrastructure modules implement database/provider/MCP/network integrations.

## Native execution boundary

`ai-tools relay` is the authority for coding-machine execution. The web server must not grow an alternate direct shell path.

The relay:

- implements MCP `2026-07-28` over Streamable HTTP (`POST /mcp`);
- binds to loopback rather than a public interface;
- refuses UID 0/root;
- uses Bubblewrap on Linux for filesystem/process containment;
- validates server-side tool authorization independently of client UI;
- supports both local and OAuth-protected remote modes;
- owns terminal timeout, cancellation, job concurrency, bounded output retention, and process-tree cleanup.

For the single-owner profile, `--execution-root` may be the canonical non-root home directory while `--dir` points at a particular starting repository. A request may change `cwd` between directories under the execution root without reconnecting the MCP client.

## Remote MCP and OAuth

The remote relay is an **OAuth Resource Server**, not an Authorization Server.

The external Authorization Server—currently Keycloak in the reference deployment—owns login, authorization codes, PKCE, token issuance, client registration, signing keys, and identity lifecycle. The relay only validates the presented access token:

- issuer (`iss`);
- resource/audience (`aud`);
- signature through asymmetric JWKS;
- expiry/time validity;
- single allowed owner subject (`sub`);
- required `relay.coding` scope.

The canonical production pattern is:

```text
Internet
  -> public HTTPS hostname
  -> outbound-established tunnel/edge
  -> http://127.0.0.1:47821
  -> ai-tools relay --mode remote
```

Do not expose port `47821` through router/NAT forwarding and do not bind the relay to `0.0.0.0`.

## external MCP client and hosted Nuxt

external MCP client and the hosted Nuxt application can consume the same public MCP resource, but they authenticate independently.

- **external MCP client** performs its own interactive OAuth flow against the Authorization Server.
- **Hosted Nuxt** currently uses a private server-side access token only for the configured first-party MCP URL and only for the configured AI Code owner user ID.

Those two owner identities are intentionally different concepts: the Nuxt owner is an `ai_code.users.id`; the relay owner is the Authorization Server's stable `sub` claim.

## Docker boundary

The coding relay intentionally does **not** expose the host Docker socket. Docker access is deferred until there is an isolated worker/broker design. A relay command failing because `/var/run/docker.sock` is unavailable is therefore expected, not a missing permission to work around.
