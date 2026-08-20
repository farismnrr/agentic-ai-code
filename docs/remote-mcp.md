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

For delegated coding providers, configure their executable directory. The public
Primary fast path exposes delegation only when startup capability discovery
finds at least one usable provider. The
relay prefers the local CLI login already present for the operator and checks
capability at startup. Provider labels are fixed by the relay; the placeholders
below must be replaced with a provider name actually advertised by the live
catalog:

```bash
export RELAY_TOOLCHAIN_PATH="$HOME/.local/bin"
export RELAY_ALLOW_AGENT_NETWORK=true
export RELAY_AGENT_ENV='<supported-provider>=AUTH_ENV_NAME'
export RELAY_AGENT_AUTH_ROOT='<supported-provider>=/home/owner/.provider-auth'
```

`RELAY_AGENT_ENV` and `RELAY_AGENT_AUTH_ROOT` are explicit process/root
configuration. They are not required for a normal local login when the CLI has
a supported status command; a CLI without one needs the explicit auth-root
mapping. The relay does not generate or infer API keys. Only authenticated or
explicitly rooted providers are included in the live Full/Primary tool schema;
restart after changing a CLI login.

Delegation runs providers serially and falls back only for classified
quota/authentication/availability failures. A bounded metadata-only snapshot
covers the selected writable workspace; fallback stops when that snapshot
changes or cannot be completed safely. Delegated providers do not
receive sibling-workspace mounts, and agent network authorization remains
independent from terminal network permission. The relay supplies bounded
provider arguments inside Bubblewrap and never adds host-level
permission-bypass flags.
When delegation completes successfully, its structured result includes the
provider's bounded final stdout in `output` (maximum 64 KiB), with
credential-shaped values redacted and applicable truncation/redaction
indicators. It does not expose hidden chain-of-thought.

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

## 4. Start the remote relay

Use the repository's Primary fast-path remote-relay launcher or an equivalent reviewed service definition:

```bash
export AI_TOOLS_BIN="$PWD/target/release/ai-tools"
scripts/phase36-start-remote-relay.sh
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
the public fast-path surface; authenticated `agent_delegate` remains available
there when a supported local CLI session is detected. Full remains the
canonical static superset for deployments that intentionally need the larger
catalog.

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
scripts/phase36-public-mcp-smoke.sh
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
scripts/phase36-public-mcp-smoke.sh
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
