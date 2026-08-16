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
Public MCP resource  <---------------- ChatGPT / other MCP clients
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
- owns terminal timeout, cancellation, job concurrency, bounded output retention, and process-tree cleanup;
- exposes bounded native workspace inspection/search/read/edit/write operations without routing routine filesystem work through a shell.

For the single-owner profile, `--execution-root` may be the canonical non-root home directory while `--dir` points at a particular starting repository. A request may change `cwd` between directories under the execution root without reconnecting the MCP client. Workspace path resolution canonicalizes existing read targets and validates write parents against this boundary. Recursive native traversal does not follow symlink directories; edit/write operations additionally use no-follow directory/file descriptors and same-directory atomic replacement semantics so validation-time containment is not treated as sufficient mutation safety.

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

## ChatGPT and hosted Nuxt

ChatGPT and the hosted Nuxt application can consume the same public MCP resource, but they authenticate independently.

- **ChatGPT** performs its own interactive OAuth flow against the Authorization Server.
- **Hosted Nuxt** currently uses a private server-side access token only for the configured first-party MCP URL and only for the configured AI Code owner user ID.

Those two owner identities are intentionally different concepts: the Nuxt owner is an `ai_code.users.id`; the relay owner is the Authorization Server's stable `sub` claim.

## Tailscale boundary

The coding relay does not expose the host Tailscale local API socket by default. A local operator may opt in with `RELAY_ALLOW_TAILSCALE=true`, which bind-mounts only `RELAY_TAILSCALE_SOCKET` (default `/var/run/tailscale/tailscaled.sock`) into the Bubblewrap sandbox so commands such as `tailscale ip -4` can query the host daemon.

## Docker boundary

The coding relay does **not** expose the host Docker socket by default. For trusted single-owner local development, an operator may explicitly opt in with `RELAY_ALLOW_DOCKER=true`; that escape hatch permits the `docker` CLI and bind-mounts the daemon socket selected by `RELAY_DOCKER_SOCKET` (default `/var/run/docker.sock`) into the Bubblewrap sandbox. Docker daemon access is effectively host-level authority and therefore weakens the filesystem boundary. Production/remote deployments should keep it disabled unless the operator deliberately accepts that trust expansion.
