# Configuration

`.env.example` is the complete repository-level inventory of Nuxt environment keys. Copy it to `.env` and set only the values required by the subsystems you use.

This page explains the groups and security intent; it intentionally does not duplicate every comment from `.env.example`.

## Core web configuration

| Variable | Purpose |
| --- | --- |
| `NUXT_PORT` | Nuxt dev port; defaults to `3333`. |
| `NUXT_HOST` | Optional bind address. Leave unset for localhost-only development. |
| `NUXT_PUBLIC_SITE_URL` | Browser-visible canonical site URL. |
| `NUXT_DATABASE_URL` | PostgreSQL connection string. |
| `NUXT_SESSION_PASSWORD` | Seals the `nuxt-auth-utils` session cookie; must be at least 32 characters. |
| `NUXT_MODEL_PROVIDER_SECRET_KEY` | 32-byte hex AES key for encrypting provider secrets at rest. |
| `NUXT_WORKSPACES_ROOT` | Operator-owned filesystem boundary used by workspace features. |

Avoid setting `NUXT_HOST=0.0.0.0` casually. If another trusted device must reach development, bind to a specific trusted interface.

## Model/router configuration

`NUXT_ROUTER_BASE_URL` and `NUXT_ROUTER_API_KEY` configure the optional router-backed provider path. User-managed provider credentials are configured in the application and encrypted using `NUXT_MODEL_PROVIDER_SECRET_KEY`.

## Application authentication

SMTP keys enable email verification and password reset:

```text
NUXT_SMTP_HOST
NUXT_SMTP_PORT
NUXT_SMTP_SECURE
NUXT_SMTP_USER
NUXT_SMTP_PASSWORD
NUXT_SMTP_FROM
```

Optional application OAuth login uses:

```text
NUXT_OAUTH_GOOGLE_CLIENT_ID
NUXT_OAUTH_GOOGLE_CLIENT_SECRET
NUXT_OAUTH_GITHUB_CLIENT_ID
NUXT_OAUTH_GITHUB_CLIENT_SECRET
```

These are unrelated to the Keycloak configuration used by the remote MCP relay. See [authentication.md](authentication.md).

## First-party remote MCP configuration

The hosted Nuxt application may call one configured first-party public MCP relay using:

```text
NUXT_REMOTE_MCP_URL
NUXT_REMOTE_MCP_OWNER_USER_ID
NUXT_REMOTE_MCP_ACCESS_TOKEN
```

Security rules:

- the URL must exactly match the stored first-party MCP server URL;
- the MCP row must belong to `NUXT_REMOTE_MCP_OWNER_USER_ID`;
- only then may Nitro attach `NUXT_REMOTE_MCP_ACCESS_TOKEN`;
- the token stays server-side and is not written to the MCP database row or returned to the browser.

`NUXT_REMOTE_MCP_OWNER_USER_ID` is an AI Code database user ID. It is **not** the OAuth `sub` used by the Rust relay.

## Relay configuration

The Rust relay accepts CLI flags and matching environment variables. Important remote-mode values include:

```text
RELAY_AGENT_MODE=remote
OAUTH_ISSUER=https://auth.example.com/realms/example
OAUTH_AUDIENCE=https://mcp.example.com/mcp
OAUTH_OWNER_SUBJECT=<stable-owner-sub>
EXECUTION_ROOT=/home/owner
RELAY_AGENT_TRUSTED_PROXY=true
RELAY_AGENT_TRUSTED_PROXY_CIDR=127.0.0.1/32
```

Execution policy can be tuned with:

```text
RELAY_DEFAULT_TERMINAL_TIMEOUT_MS
RELAY_MAX_TERMINAL_TIMEOUT_MS
RELAY_COMPLETED_JOB_TTL_MS
RELAY_MAX_RETAINED_OUTPUT_BYTES
RELAY_MAX_RUNNING_JOBS
RELAY_TOOLCHAIN_PATH
RELAY_ALLOW_DOCKER
RELAY_DOCKER_SOCKET
```

`timeout_ms: 0` means no command deadline unless `RELAY_MAX_TERMINAL_TIMEOUT_MS` imposes an operator maximum.

`RELAY_ALLOW_DOCKER=true` is an explicit local-development escape hatch. It permits the `docker` CLI and bind-mounts the host Docker daemon socket into the terminal sandbox. `RELAY_DOCKER_SOCKET` can point at a non-default/rootless Unix socket and defaults to `/var/run/docker.sock`. Docker daemon access can provide host-level authority, so the default remains disabled and it should only be enabled for a trusted single-owner coding relay.

## Telemetry

Optional telemetry keys:

```text
NUXT_OTEL_ENABLED
NUXT_OTEL_SERVICE_NAME
NUXT_OTEL_JAEGER_ENDPOINT
NUXT_OTEL_LOKI_PUSH_URL
```

Telemetry is designed to remain useful without carrying secrets/PII. Do not weaken sanitization to make debugging easier; use bounded classifications and request/trace IDs instead.

## Docker Compose

`docker-compose.yml` expects external networks named `masihawam-net` and `shared-network` and can override the database URL with `NUXT_DATABASE_URL_DOCKER` so a container does not incorrectly resolve host `localhost` as itself.

The workspace directory is mounted at the same absolute path inside the container. Keep `NUXT_WORKSPACES_ROOT` consistent with that mount.

The Docker Compose stack is for the web application/observability topology. It does not grant Docker access to the Rust coding relay.
