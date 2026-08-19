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
NUXT_REMOTE_MCP_REQUEST_TIMEOUT_MS=45000
```

`NUXT_REMOTE_MCP_REQUEST_TIMEOUT_MS` is the first-party client's per-HTTP-round-trip deadline, not a durable task execution limit. Keep it bounded (1,000–120,000 ms); long-running work should continue through the MCP task lifecycle instead of relying on one HTTP request remaining open.

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
RELAY_ALLOWED_HOSTS=mcp.example.com
```

Execution policy can be tuned with:

```text
RELAY_DEFAULT_TERMINAL_TIMEOUT_MS
RELAY_MAX_TERMINAL_TIMEOUT_MS
RELAY_COMPLETED_JOB_TTL_MS
RELAY_MAX_RETAINED_OUTPUT_BYTES
RELAY_MAX_RUNNING_JOBS
RELAY_ALLOW_TERMINAL_NETWORK
RELAY_TOOLCHAIN_PATH
RELAY_ALLOW_DOCKER
RELAY_DOCKER_SOCKET
RELAY_ALLOW_TAILSCALE
RELAY_TAILSCALE_SOCKET
```

`timeout_ms: 0` means no command deadline unless `RELAY_MAX_TERMINAL_TIMEOUT_MS` imposes an operator maximum.

Terminal subprocesses use an isolated network namespace by default. Set `RELAY_ALLOW_TERMINAL_NETWORK=true` (or pass `--allow-terminal-network`) only for a trusted workflow that needs network-capable commands such as package installation or remote Git. Dedicated `http_fetch` and `web_search` remain separate network capabilities and are not enabled by this flag.

Conversation approval modes are `plan` (read-only), `workspace` (edits with review for risky operations), `autonomous` (low-risk bounded calls may proceed automatically), and `manual` (prompt-oriented). These modes never bypass relay hard boundaries. Remembered `always` decisions are narrowed to low-risk, non-opaque calls; shell/interpreter wrappers, network requests, destructive operations, and unknown commands still require review.

`RELAY_TOOLCHAIN_PATH` is a comma-separated set of reviewed user-owned executable directories appended to the relay safe PATH (the CLI equivalent is repeated `--toolchain-path`). Use it for version-manager/runtime directories such as Cargo, Bun, or the active fnm Node installation. The relay intentionally does not inherit the login-shell `$PATH`; this keeps executable discovery explicit and prevents unrelated user PATH entries from silently becoming agent capabilities.

`RELAY_ALLOW_TAILSCALE=true` exposes only the configured Tailscale local API Unix socket to sandboxed commands. `RELAY_TAILSCALE_SOCKET` defaults to `/var/run/tailscale/tailscaled.sock` and may be changed for alternate installations. Keep it disabled unless local-development commands need to query the host Tailscale daemon.

`RELAY_ALLOW_DOCKER=true` is an explicit local-development escape hatch. It permits the `docker` CLI and bind-mounts the host Docker daemon socket into the terminal sandbox. `RELAY_DOCKER_SOCKET` can point at a non-default/rootless Unix socket and defaults to `/var/run/docker.sock`. Docker daemon access can provide host-level authority, so the default remains disabled and it should only be enabled for a trusted single-owner coding relay.

In local mode, `127.0.0.1:<port>` and `localhost:<port>` are always allowed. Use repeated `--allowed-host` flags or the comma-separated `RELAY_ALLOWED_HOSTS` value for explicitly permitted external Host authorities. Entries may include an exact port; an entry without a port matches only a Host without a port, and never implicitly allows arbitrary ports. Wildcards and URL syntax are rejected.

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

# Deterministic lifecycle hooks

The relay keeps repository hook configuration disabled by default. An operator
may explicitly enable the vendor-neutral `.agents/hooks.json` file with
`--enable-agent-hooks` (or `RELAY_ENABLE_AGENT_HOOKS=true`). The file must carry
the canonical repository identity and each handler must use a direct executable
from the relay safe PATH. Shell indirection, absolute executable paths, network
access, optional host sockets, credentials, and raw tool payloads are not
available to hooks.

Hook failures are fail-closed for `security` handlers and explicitly fail-open
for `cosmetic` handlers. `pre_tool_use` can only block or request approval;
there is no hook result that grants authority. `after_file_change` runs only
after a committed native mutation. Stop gates are attempted at most twice.

Hook telemetry records only bounded event, decision, duration, and reason class.
## Parent-managed subagents and profiles

First-party agent-mode conversations expose a parent-only `delegate_task`
capability. Profiles are vendor-neutral Markdown files with YAML frontmatter
under `.agents/agents/`; the built-ins are `explore`, `plan`, `review`,
`verify`, and `general-purpose`. Profile instructions are not authority: the
effective tools, effects, working mode, model hint, and workspace are always
an intersection with the current parent/session and operator policy.

Child context is explicit and bounded to a task plus path references. Results
contain only a bounded status, summary, findings, evidence, validation, risks,
and budget usage; hidden reasoning and full child transcripts are not
persisted. Budgets cap turns, tool calls, output/context, wall time, and
depth. Parent cancellation reaches the child, and only one child may run for
one parent at a time. The existing stop control cancels a running child.

Plan 039F remains the sequential delegation path; background execution is
provided by the bounded Plan 039G task surface below.

### Background agents (Plan 039G)

Agent mode also exposes parent-managed `agent_task_start`, `agent_task_get`,
and `agent_task_cancel` tools. Background execution is opt-in and bounded to
four active tasks per process and two per parent session. Terminal task
metadata is retained only in a bounded in-memory registry; polling returns
structured summaries and evidence, never hidden reasoning or child
transcripts.

`shared_read` tasks are mechanically narrowed to read-only workspace and Git
effects, even when the parent is in workspace mode. A `worktree` task is only
available to `general-purpose`; it refuses a dirty parent checkout, creates a
task-owned branch and worktree below `.agents/worktrees`, and runs with that
path as its workspace root. Writer results include bounded Git status, diff
stat, commit, and validation evidence. The parent must review and integrate
changes explicitly; children do not merge, cherry-pick, push, force-push, or
rewrite history. Dirty, unowned, ambiguous, or uniquely committed worktrees
are preserved.

Parent-coordinated background tasks are the intentional 039G coordination
model. Peer-to-peer teams and shared task lists remain deferred because the
independent concurrent scenario is covered without introducing another
mutable coordination state or agent framework.

Task metadata is intentionally process-local in 039G. After a restart, an
existing worktree is not automatically adopted, cleaned, or reused because
ownership cannot be proven without introducing a persistence system.
