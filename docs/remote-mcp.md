# Remote MCP deployment

This guide publishes the Linux coding relay for any compatible MCP client
without opening an inbound relay port on the coding machine. The relay stays
loopback-only; an HTTPS reverse proxy or outbound tunnel provides the public
edge.

## Security model

The supported production shape is:

```text
public HTTPS hostname
    -> operator-controlled reverse proxy or outbound tunnel
    -> 127.0.0.1:47821
    -> ai-tools relay --mode remote
```

Hard rules:

- keep the relay listener on loopback;
- do not router/NAT-forward port `47821`;
- do not bind the relay to `0.0.0.0`;
- run as a normal non-root owner;
- install/use Bubblewrap;
- keep the execution root explicit;
- keep OAuth issuer, audience, owner, signature, time, and scope validation
  enabled; and
- never expose the host Docker socket to the relay.

## 1. Build or install `ai-tools`

From source:

```bash
pnpm build:tools
./target/release/ai-tools --version
```

The stable release artifact is a Linux `x86_64-unknown-linux-gnu`
binary/archive.

## 2. Choose filesystem scope

For a single-owner coding machine:

```bash
export EXECUTION_ROOT="$HOME"
export RELAY_WORKING_DIR="$HOME/Projects/your-project"
```

`EXECUTION_ROOT` is the hard filesystem boundary. `RELAY_WORKING_DIR` is only
the starting `cwd`; other roots beneath the execution boundary still need
explicit `workspace_add` authorization.

If toolchains are installed under the home directory, allow only the required
directories through `RELAY_TOOLCHAIN_PATH`. Do not copy the entire interactive
shell PATH into the relay.

Provider-specific coding-CLI delegation is not part of the current relay
surface. Long-running eligible tools use standard MCP Tasks with explicit
`execution_mode=sync|async|auto`; `auto` selects async only when the client
advertises Tasks.

## 3. Configure OAuth values

Follow [oauth-provider.md](oauth-provider.md) and then set:

```bash
export REMOTE_MCP_URL='https://mcp.example.com/mcp'
export OAUTH_ISSUER='https://auth.example.com/tenant/example'
export OAUTH_AUDIENCE="$REMOTE_MCP_URL"
export OAUTH_OWNER_SUBJECT='<stable-owner-subject>'
```

The resource URL must be canonical HTTPS and include `/mcp`. The same exact
value must be present in the token audience.

To record this relay in the hosted workspace activity ledger, separately
enroll/bind a source as described in [configuration.md](configuration.md#workspace-activity-ledger),
then add the `RELAY_ACTIVITY_*` values to the relay service environment. The
activity sink is an authenticated Nuxt API; it is independent of OAuth tool
authorization and does not grant access to normal application APIs. Keep the
relay state directory outside `EXECUTION_ROOT`/workspace mounts where
possible, and set `RELAY_ACTIVITY_MODE=required` when silent pre-execution
gaps are unacceptable.

### Optional task-completion Telegram notification

The relay can send one plain-text notification after one complete task/plan
reaches its successful terminal state. This is not a Telegram MCP tool and it
does not send a message for individual tools, activity rows, or stream chunks.
The relay imports the existing Hermes Telegram configuration from its
owner-only `.env`:

```text
RELAY_TELEGRAM_ENABLED=true
# Optional; default is $HOME/.hermes/.env
RELAY_TELEGRAM_HERMES_ENV=/home/owner/.hermes/.env
```

On each startup, only `TELEGRAM_BOT_TOKEN` and `TELEGRAM_HOME_CHANNEL` are
imported from Hermes. The token is encrypted in the relay-owned SQLite
database; the encryption key stays in a separate owner-only relay state file.
`TELEGRAM_ALLOWED_USERS` is not a delivery target. The home channel must be a
channel identifier (`-100...` or `@channel_username`); a private Hermes home
chat is rejected and never used as a fallback. The token and recipient are
never accepted in MCP arguments, and the relay only calls Telegram's fixed
`sendMessage` endpoint. The durable relay ledger deduplicates by logical
`taskId`; Nuxt uses a separate server-side outbox so temporary relay
unavailability does not turn a completed task into a failed task.

## 4. Start the remote relay

Use the repository's Primary fast-path remote-relay launcher or an equivalent reviewed service definition:

```bash
export AI_TOOLS_BIN="$PWD/target/release/ai-tools"
ops/remote-mcp/start-relay.sh
```

The important effective arguments are:

```bash
RELAY_TOOL_PROFILE=primary \
ai-tools relay \
  --mode remote \
  --trusted-proxy \
  --trusted-proxy-cidr 127.0.0.1/32 \
  --port 47821 \
  --dir "$RELAY_WORKING_DIR" \
  --execution-root "$EXECUTION_ROOT" \
  --oauth-issuer "$OAUTH_ISSUER" \
  --oauth-audience "$OAUTH_AUDIENCE" \
  --oauth-owner-subject "$OAUTH_OWNER_SUBJECT"
```

The relay accepts forwarded HTTPS metadata only when the direct peer is in
the explicitly trusted loopback CIDR. The repository launcher pins Primary for
the public fast-path surface. Full remains the canonical static superset for
deployments that intentionally need the larger catalog.

## 5. Publish through an HTTPS edge

Configure the selected reverse proxy or outbound tunnel with one narrow route:

```text
https://mcp.example.com/mcp  ->  http://127.0.0.1:47821/mcp
```

The edge must preserve the `/mcp` path, forward the original HTTPS scheme in
the reviewed proxy header, and avoid caching MCP responses or OAuth metadata.
If the edge supports an allowlist, permit only the public hostname and the
loopback upstream. Do not put a separate interactive login page in front of
`/mcp` or `/.well-known/oauth-protected-resource*`; clients must receive the
relay's own OAuth challenge and metadata.

The Authorization Server may use a separate HTTPS hostname and edge. Its
public issuer and discovery/JWKS routes must remain stable and must match
`OAUTH_ISSUER` exactly.

## 6. Verify the public edge without executing tools

```bash
export REMOTE_MCP_URL='https://mcp.example.com/mcp'
ops/remote-mcp/public-smoke.sh
```

A successful unauthenticated run proves:

- public `/health` is reachable;
- Protected Resource Metadata is reachable and names an HTTPS Authorization
  Server;
- `relay.coding` is advertised; and
- unauthenticated MCP discovery receives a Bearer challenge.

It does not prove OAuth completion or tool execution.

## 7. Verify with an owner token

Store one access token in a private file:

```bash
umask 077
printf '%s' "$ACCESS_TOKEN" > /tmp/ai-code-mcp-token
unset ACCESS_TOKEN
export REMOTE_MCP_ACCESS_TOKEN_FILE=/tmp/ai-code-mcp-token
ops/remote-mcp/public-smoke.sh
rm -f /tmp/ai-code-mcp-token
```

This additionally verifies authenticated `server/discover` and `tools/list`
without executing a tool. Never put a real token in command arguments, logs,
documentation, or source control.

## 8. Hosted first-party client

For a single-owner hosted application, configure the matching private values:

```text
NUXT_REMOTE_MCP_URL=https://mcp.example.com/mcp
NUXT_REMOTE_MCP_OWNER_USER_ID=<application-user-id>
NUXT_REMOTE_MCP_ACCESS_TOKEN=<owner-access-token>
```

The application must attach that token only when both the stored MCP URL and
authoritative owner match. Token expiry and rotation remain the operator's
responsibility; do not use a practically permanent bearer token.

## 9. Connect any compatible MCP client

After public metadata and authenticated smoke checks pass, use
[mcp-client.md](mcp-client.md). Configure the public HTTPS resource URL,
select OAuth/OIDC, complete authorization as the configured owner, and let
the client rediscover `tools/list`.

The relay can connect to any compatible client that satisfies these transport
and authorization contracts. It is not an open proxy: filesystem roots,
subprocess capabilities, network access, delegation providers, and tool
profiles remain controlled by the operator configuration.
