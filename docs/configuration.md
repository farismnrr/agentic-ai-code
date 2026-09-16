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
| `NUXT_ACTIVITY_PAYLOAD_SECRET` | Dedicated server-only key for encrypting historical activity evidence; use at least 32 characters. |
| `NUXT_ACTIVITY_RETENTION_DAYS` | Bounded product-history retention in days; defaults to 90 and accepts 1–3650. |
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

Optional application OAuth login uses the provider-specific client ID and
secret variables documented in `.env.example`.

These are unrelated to the Authorization Server configuration used by the remote MCP relay. See [authentication.md](authentication.md).

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

Nuxt chat-mode network tools also use this first-party MCP connection; they do
not execute a local Rust binary. The access token is server-only and must be a
short-lived/rotatable OAuth token issued for the relay resource. External MCP
clients perform their own interactive OAuth flow; the Nuxt application's
normal application login is a separate boundary and is not forwarded to the
relay.

## Workspace activity ledger

Plan 050 activity is product history, not OpenTelemetry/Loki telemetry. Set
`NUXT_ACTIVITY_PAYLOAD_SECRET` in every Nuxt instance that receives activity;
source-bearing exact evidence is AES-256-GCM encrypted before PostgreSQL
storage. The default retention is 90 days and cleanup runs in bounded batches;
`NUXT_ACTIVITY_RETENTION_DAYS` can be set from 1 through 3650.

Enroll a relay source from an authenticated server-side/admin workflow:

```text
POST /api/activity/sources        { "label": "owner relay", "kind": "relay" }
POST /api/activity/bindings       { "sourceId": "<returned id>", "workspaceId": "<owned workspace id>" }
```

The enrollment response contains the high-entropy bearer token once. Store it
in the relay process configuration only; the database stores its hash and safe
prefix. Configure the relay with the matching sink and local state settings:

```text
RELAY_ACTIVITY_MODE=required
RELAY_ACTIVITY_STATE_DIR=/home/owner/.local/state/ai-tools
RELAY_ACTIVITY_SINK_URL=https://app.example.com/api/activity/ingest
RELAY_ACTIVITY_SOURCE_TOKEN=<one-time-enrollment-token>
RELAY_ACTIVITY_SPOOL_QUOTA_BYTES=67108864
RELAY_ACTIVITY_ACK_RETENTION_MS=86400000
```

For optional explicit Telegram messaging, configure the fixed recipient only on
the relay host. These are server environment values, not browser or MCP request
fields:

```bash
RELAY_TELEGRAM_ENABLED=true
```

Before enabling the service, provision the relay-owned database once from a
compatible owner-controlled dotenv file. An existing Hermes `.env` can be
used as that input; the relay reads only `TELEGRAM_BOT_TOKEN`,
`TELEGRAM_HOME_CHANNEL`, and the optional
`TELEGRAM_HOME_CHANNEL_THREAD_ID` during this explicit bootstrap command and
never reads Hermes at runtime:

```bash
ai-tools telegram \
  --env-file /home/owner/.hermes/.env \
  --state-dir /home/owner/.local/state/ai-tools
```

The command validates that the destination is a Telegram channel or supergroup
(`-100...` or `@channel_username`) and stores the token encrypted in the
relay-owned SQLite database. For a forum supergroup, the optional thread ID
selects the topic; an empty value uses the root/general topic.
`TELEGRAM_ALLOWED_USERS` remains an inbound Hermes allowlist and is never used
as the destination. The running relay reads only its own database and never
depends on Hermes code, process, or files. The encryption key is kept in a
separate owner-only relay state file.

When enabled, MCP discovery exposes `telegram_send_message`. Every call requires
exactly two fields: an absolute `working_directory` and a non-empty `message`.
The relay canonicalizes the directory and requires it to be inside the current
authorized workspace allowlist before queueing any delivery. Token, chat ID,
topic ID, Bot API endpoint, and arbitrary Telegram methods are not accepted as
tool arguments.

The final plain-text message is formatted server-side as:

```text
Working directory: /absolute/authorized/workspace

<bounded redacted message>
```

The working directory is intentionally sent to the fixed operator-controlled
Telegram destination on every explicit call. Message text remains bounded to
Telegram's 4096-byte limit and credential-shaped values are redacted. If the
full canonical directory plus redacted message cannot fit, the relay rejects the
call rather than truncating either field. Calls are queued independently, so two
legitimate identical explicit sends are not deduplicated merely because their
text matches. Legacy automatic task-completion
rows are not replayed by the new explicit-message queue. Orchestration lifecycle
completion no longer emits Telegram messages automatically.

The relay source ID is stable in `source-id` inside the owner-only state
directory. Sink outages leave encrypted unacknowledged rows queued for retry;
401/403 stops delivery and reports degraded state without printing the token.
The quota never overwrites unacknowledged rows, so required mode rejects a new
workspace operation when its start cannot be admitted. Clearing a workspace's
history removes retained metadata/evidence while preserving source enrollment;
the source sequence watermark prevents delayed pre-clear rows from returning.

List/live activity responses contain bounded metadata only. Historical diffs
are lazy and available only for exact structured text mutations (`file_edit`,
`file_write`, and applied `apply_patch`); opaque process/Git work is
shown as summary or unavailable evidence. `clientInfo` is display metadata,
not authorization: absent or unsupported client identity is shown as
**External MCP client**.

## Relay configuration

The relay has one default workspace root: `RELAY_WORKSPACE_ROOT` or
`--workspace-root` (`--dir` remains an alias), defaulting to
`$HOME/Documents/Projects`. This root is the primary authorized workspace and,
unless explicitly overridden, the hard execution ceiling too. Every child
directory under `Projects` is in scope and can be selected with `cwd`; no
per-repository `workspace_add` is needed. A narrower primary workspace may use
an explicit `--execution-root` ceiling plus `workspace_add` for additional
roots inside that ceiling. A broader ceiling does not authorize sibling paths
by itself.

This profile still uses Bubblewrap and a rebuilt minimal environment. It does
not inherit login-shell credentials or PATH. Credential files, session/keyring
stores, relay state and discovered Unix sockets remain masked; `.env.example`
is the intentional non-secret exception. Discovery must complete within
500,000 entries per visible tree, including dependency/build/cache directories,
or execution fails closed. A larger home should use narrower explicit roots.
See [security](security.md#terminal-filesystem-and-credential-boundary).

Tools use dedicated MCP capabilities first for operations they cover. Terminal
is the fallback for builds, tests, package managers, interpreters, scripts,
pipelines and unsupported CLI operations. Host `systemctl --user` / `journalctl
--user` operations are intentionally unavailable through generic terminal
execution: HOME scope does not expose the host user bus or journals.

The Rust relay accepts CLI flags and matching environment variables. Important remote-mode values include:

```text
RELAY_AGENT_MODE=remote
OAUTH_ISSUER=https://auth.example.com/realms/example
OAUTH_AUDIENCE=https://mcp.example.com/mcp
OAUTH_OWNER_SUBJECT=<stable-owner-sub>
RELAY_WORKSPACE_ROOT=/home/owner/Documents/Projects
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

`RELAY_WORKSPACE_ROOT` is the single default filesystem root. If unset, the CLI uses `$HOME/Documents/Projects`; it supplies the primary workspace and defaults the hard execution ceiling to the same path. Child repositories stay inside both boundaries. The optional `--execution-root` flag is an explicit advanced override; there is no separate `EXECUTION_ROOT` environment setting. Regardless of scope, Bubblewrap enforces read-only system runtime mounts (`/usr`, `/lib`, `/etc`, `/bin`, `/sbin`), isolated tmpfs `/tmp`, separate `/proc` and `/dev`, and masks all known credential directories and Unix domain sockets regardless of nesting depth.

`timeout_ms: 0` means no command deadline unless `RELAY_MAX_TERMINAL_TIMEOUT_MS` imposes an operator maximum.

`terminal_exec`, `http_fetch`, and `web_search` accept `execution_mode`:
`sync` waits for the direct result, `async` returns an MCP task and requires a
client that advertises Tasks, and `auto` uses async only when the client
advertises Tasks. Primary and Full advertise the same Tasks capability. An
explicit async request from an incompatible client is rejected; it is never
silently converted to sync. Mutating HTTP methods remain synchronous until a
request-level idempotency layer is available.

Terminal subprocesses use an isolated network namespace (`--unshare-net`) by default, preventing outbound TCP/UDP connects and raw sockets at the kernel level. Set `RELAY_ALLOW_TERMINAL_NETWORK=true` (or pass `--allow-terminal-network`) only for trusted workflows requiring network-capable CLI commands (e.g. package management, dependency installation, or loopback service communication). Dedicated Git, `http_fetch` and `web_search` remain separate network capabilities subject to their own SSRF, private-network, and domain allowlist policies; they do not require or influence this flag. Generic `ssh`, `scp` and `sftp` remain blocked; remote diagnostics use `ssh_readonly_exec`.

Conversation approval modes are `plan` (read-only), `workspace` (edits with review for risky operations), `autonomous` (low-risk bounded calls may proceed automatically), and `manual` (prompt-oriented). These modes never bypass relay hard boundaries. Remembered `always` decisions are narrowed to low-risk, non-opaque calls; shell/interpreter wrappers, network requests, destructive operations, and unknown commands still require review.

`RELAY_TOOLCHAIN_PATH` is a comma-separated set of reviewed user-owned executable directories prepended to the relay safe PATH (the CLI equivalent is repeated `--toolchain-path`). Use it for version-manager/runtime directories such as Cargo, Bun, or the active fnm Node installation. The relay intentionally does not inherit the login-shell `$PATH`; this keeps executable discovery explicit, gives operator-selected runtimes precedence, and prevents unrelated user PATH entries from silently becoming agent capabilities.

Provider-specific coding-CLI delegation is not part of the current relay
surface. Long-running eligible tools use the standard MCP Tasks contract and
the explicit `execution_mode` described above.

### Creative execution bindings

Creative production remains disabled unless `RELAY_ENABLE_CREATIVE=true` (or
`--enable-creative`) is set before relay startup. Public binding discovery and
private execution configuration are intentionally separate. A discoverable
binding descriptor contains semantic capabilities and bounded constraints only;
it must not contain endpoints, credentials, executable paths, command payloads,
or a product-wide default provider/model. Register descriptors with repeated
`--creative-binding` flags or the semicolon-separated
`RELAY_CREATIVE_BINDING` value.

A concrete implementation is mapped separately with repeated
`--creative-binding-backend binding_id=backend_kind` flags or
`RELAY_CREATIVE_BINDING_BACKEND`. Built-in reviewed backends are:

- `local_raster`: a pure-Rust contained PNG conformance/utility backend for the
  reviewed image capabilities. It creates candidate Assets under the selected
  Creative project's contained `creative/<project_id>/assets/generated/`
  subtree and preserves job, parent-Asset, and optional Element lineage. It is
  not an AI image model and is not a subjective quality guarantee.
- `local_static_game`: a private authenticated deployment backend for
  `game.deploy`. It snapshots one accepted build into relay-owned contained
  deployment storage and exposes it under `/creative-deploy/<deployment_id>/`.
  This is a deploy target, not public publication; `published` remains false.

External quality-capable media providers remain explicit operator integrations;
client-visible descriptors never carry provider endpoints or credentials.

Example operator configuration:

```text
RELAY_ENABLE_CREATIVE=true
RELAY_CREATIVE_BINDING={"binding_id":"binding_local_raster","binding_version":"local-raster-v1","capabilities":["image.generate","image.reference_generate","image.edit","image.inpaint","image.upscale","image.remove_background","image.outpaint"],"media_roles":["image","reference_image","mask"],"extension_schema":{"type":"object","additionalProperties":false},"constraints":{"max_width":4096,"max_height":4096,"estimate":{"base_compute_units":10,"base_output_bytes":4096}},"estimate_available":true,"availability":"available"}
RELAY_CREATIVE_BINDING_BACKEND=binding_local_raster=local_raster
```

Optional private local Game deploy binding:

```text
RELAY_CREATIVE_BINDING={"binding_id":"binding_local_game","binding_version":"local-static-game-v1","capabilities":["game.deploy"],"media_roles":["deployment"],"extension_schema":{"type":"object","additionalProperties":false},"constraints":{"estimate":{"base_compute_units":10,"base_output_bytes":4096}},"estimate_available":true,"availability":"available"}
RELAY_CREATIVE_BINDING_BACKEND=binding_local_game=local_static_game
```

When image and local Game deploy bindings are both enabled, join both repeated
environment values with `;` as documented by the relay CLI parser.

Creative admission limits are separately operator-controlled through
`RELAY_CREATIVE_APPROVAL_COMPUTE_UNITS`,
`RELAY_CREATIVE_JOB_HARD_COMPUTE_UNITS`,
`RELAY_CREATIVE_PROJECT_HARD_COMPUTE_UNITS`,
`RELAY_CREATIVE_MAX_JOB_OUTPUT_BYTES`,
`RELAY_CREATIVE_MAX_CONCURRENT_JOBS`, and `RELAY_CREATIVE_MAX_RETRIES`.
Approval can cross only the soft approval threshold; it never overrides a hard
limit.

### Blender production engine

Plan 069 uses one operator activation switch for the complete Creative production
platform: `RELAY_ENABLE_CREATIVE=true` / `--enable-creative`. The same flag
controls Scene, Anime/Blender, Game, graph, delivery, and related tool exposure;
there is no separate Blender enable flag. The relay speaks directly to the
official Blender Lab loopback TCP bridge; v1 deliberately has **no configurable host**.
`RELAY_BLENDER_PORT` / `--blender-bridge-port` defaults to `9876`, and
`RELAY_BLENDER_TIMEOUT_MS` / `--blender-bridge-timeout-ms` defaults to 30000 ms with a
120000 ms hard maximum.

`RELAY_BLENDER_EXECUTABLE` / `--blender-executable` is optional operator-only
executable authority used by explicit `blender_session start`. It may point to a
reviewed Blender installation outside the Projects workspace; callers cannot
supply or override an executable, process ID, host, port, or launch arguments.
When no executable is configured, session lifecycle code may resolve only a
bounded reviewed standard Blender location/PATH entry. Relay-owned sessions use
one execution-root-owned runtime profile `.masihawam/blender-runtime-home`, shared
across Creative projects beneath that relay execution root, and invoke the
installed official Blender Lab extension headlessly with the fixed CLI
shape `--background --online-mode --command blender_mcp --host localhost
--port <operator-port>`. The host is fixed loopback and the port remains
operator-owned. The runtime profile must be prepared before relay start; the
relay never downloads or installs Blender extensions implicitly. External
interactive Blender sessions remain attach-only and never gain relay stop/kill
ownership.

Prepare the profile from a reviewed checkout of the official Blender Lab MCP
repository with Blender's own extension tooling:

```bash
blender --background --command extension build \
  --source-dir /path/to/blender_mcp/addon/blender_mcp_addon \
  --output-dir /tmp/blender-mcp-build
HOME="$PWD/.masihawam/blender-runtime-home" \
  blender --online-mode --background --command extension install-file \
  /tmp/blender-mcp-build/mcp-1.0.0.zip --repo user_default --enable
HOME="$PWD/.masihawam/blender-runtime-home" \
  blender --background --command help | grep blender_mcp
```

The last command is the pre-restart readiness check and must print
`blender_mcp`. Prepare this profile once per relay execution root; individual
Creative projects do not require separate extension installations. If the
profile or command is unavailable, `blender_session start` fails closed instead
of silently launching a process with no bridge. An already
running official loopback bridge may still be attached as an external session
and is never given relay stop/kill ownership.

Every workflow-owned Blender production artifact is project data and must stay
beneath the selected project's canonical subtree:

```text
blender/
  scenes/
  assets/
  references/
  renders/preview/
  renders/final/
  animations/
  exports/
  checkpoints/
  tmp/
```

The frozen 11-tool v1 contract accepts contained Asset IDs, reviewed enums, and
bounded leaf file names rather than arbitrary URLs or host paths. The single
runtime catalog composes those tools only for the Full profile when both
Creative and Blender are enabled. When disabled, `creative_status` reports the
required Blender activation step and the optional Blender capability resource is
absent. When enabled, that read-only resource documents the canonical project
layout and structured-first routing: prefer session/inspect/docs/import/render/
checkpoint tools, and use `blender_execute_python` only for explicit high-risk
host-user authoring that the structured tools cannot express.

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

### Debugging one agent/tool action

Use `request.id` / trace correlation rather than raw payload logging. For one failed or blocked tool action:

1. find `chat.tool.policy` to see the bounded `tool.name`, effect set, policy outcome, and policy source; a denied action is observable even though tool execution never starts;
2. if execution was allowed, follow `chat.tool.action` for duration and bounded result classification (`cancelled`, `timeout`, a reviewed runtime/provider code, `unclassified`, or the normal success size class);
3. follow `chat.subagent.*` / `chat.background.*` only when the same turn delegated work; and
4. correlate provider/request spans by trace/request ID instead of enabling raw arguments, provider responses, source, paths, credentials, or exception text.

All direct application request spans and structured logs use the same allowlist sanitizer. Unknown semantic fields are dropped, and exception text remains reduced to secret-safe static classifications.

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

## MCP tool profiles (Plan 045)

The relay supports `RELAY_TOOL_PROFILE=full|primary` (or `--tool-profile`). `full` is the default and canonical superset; `primary` is the smaller public routing/UX fast path and does not change the underlying authorization or filesystem boundaries. The repository remote launcher pins Primary.

Primary has a 15-tool retained core for the common coding fast path: terminal execution and job lifecycle plus structured workspace inspection/editing and workspace authorization. Full has a 52-tool retained base, adding remote Git transport, HTTP/web, SSH diagnostics, forge/issues/workflows, alerts, and Telegram integration. The client-visible runtime catalog is composed once from the selected profile plus explicit optional-capability flags; numbered catalog snapshots under `.agents/contracts/` are historical audit artifacts only. Local Git wrappers and LSP wrappers are intentionally removed from the public catalog; use terminal fallback when no retained structured capability covers an operation. Eligible asynchronous tools accept `execution_mode=sync|async|auto`. `ssh_readonly_exec` is Full-only: clients provide only `{ alias, command, args, timeout_ms, execution_mode }`; SSH config/key resolution stays relay-owned.

Plan 069 creative production is disabled by default with `RELAY_ENABLE_CREATIVE=false` / no `--enable-creative`. `creative_status` remains discoverable so clients can inspect activation state. When creative production is enabled, the additional creative mutation/discovery tools are composed into the Full runtime catalog through the same catalog builder; there is no second catalog version. Changing this process-start configuration requires an operator-controlled relay restart, not an implicit tool action.

A simultaneous public Full + Primary deployment is a separate operator decision because separate endpoints may require reviewed OAuth/resource configuration. Where a client can hide actions client-side, that can be used for A/B testing without a second endpoint.
