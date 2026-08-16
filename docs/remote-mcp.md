# Remote MCP deployment

This guide exposes the Linux coding relay to hosted Nuxt, external MCP client, and other compatible MCP clients without opening an inbound relay port on the laptop.

## Security model

The supported production shape is:

```text
public HTTPS
    -> outbound-established tunnel/edge
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
- keep OAuth issuer, audience, owner, signature, time, and scope validation enabled;
- never expose the host Docker socket to the relay.

## 1. Build or install `ai-tools`

From source:

```bash
pnpm build:tools
./target/release/ai-tools --version
```

The release artifact is a Linux `x86_64-unknown-linux-gnu` binary/archive when publishing stable releases.

## 2. Choose filesystem scope

For a single-owner coding machine, the recommended profile is:

```bash
export EXECUTION_ROOT="$HOME"
export RELAY_WORKING_DIR="$HOME/Projects/your-project"
```

`EXECUTION_ROOT` is the security boundary. `RELAY_WORKING_DIR` is only the starting `cwd`.

If your toolchains are installed under the home directory, explicitly allow only the required directories, for example through `RELAY_TOOLCHAIN_PATH`. Do not copy the entire interactive shell PATH into the relay.

## 3. Configure OAuth values

After following [keycloak.md](keycloak.md) or configuring another compatible Authorization Server:

```bash
export REMOTE_MCP_URL='https://mcp.example.com/mcp'
export OAUTH_ISSUER='https://auth.example.com/realms/masihawam'
export OAUTH_OWNER_SUBJECT='<stable-owner-sub>'
```

The URL must be canonical HTTPS and use the `/mcp` path.

## 4. Start the remote relay

Use the repository wrapper:

```bash
export AI_TOOLS_BIN="$PWD/target/release/ai-tools"
scripts/phase36-start-remote-relay.sh
```

The wrapper fixes the trusted proxy CIDR to IPv4 loopback and is equivalent to the important parts of:

```bash
ai-tools relay \
  --mode remote \
  --trusted-proxy \
  --trusted-proxy-cidr 127.0.0.1/32 \
  --port 47821 \
  --dir "$RELAY_WORKING_DIR" \
  --execution-root "$EXECUTION_ROOT" \
  --oauth-issuer "$OAUTH_ISSUER" \
  --oauth-audience "$REMOTE_MCP_URL" \
  --oauth-owner-subject "$OAUTH_OWNER_SUBJECT"
```

The relay itself receives HTTP over loopback from the tunnel. It accepts `X-Forwarded-Proto: https` only because trusted-proxy mode is explicit and the direct peer is inside `127.0.0.1/32`.

## 5. Publish through Cloudflare Tunnel

Cloudflare Tunnel is the reference edge, not a protocol dependency.

For a locally-managed tunnel:

```bash
export CLOUDFLARED_TUNNEL_ID='<tunnel-uuid>'
export CLOUDFLARED_CREDENTIALS_FILE="$HOME/.cloudflared/<tunnel-uuid>.json"
export REMOTE_MCP_URL='https://mcp.example.com/mcp'

scripts/phase36-cloudflared-config.sh > "$HOME/.cloudflared/config.yml"
cloudflared tunnel ingress validate
cloudflared tunnel run "$CLOUDFLARED_TUNNEL_ID"
```

Generated ingress is intentionally narrow:

```yaml
ingress:
  - hostname: mcp.example.com
    service: http://127.0.0.1:47821
  - service: http_status:404
```

For remotely-managed Cloudflare tunnels, configure the equivalent hostname -> `http://127.0.0.1:47821` route in Cloudflare instead of generating a local config file.

Do not put a separate Cloudflare Access login page in front of `/mcp` or the OAuth Protected Resource Metadata routes. MCP clients need to discover the relay's own OAuth challenge directly.

A cache-bypass policy for `/mcp` and `/.well-known/oauth-protected-resource*` is recommended during rollout so stale authorization metadata is not served.

## 6. Verify the public edge without executing tools

```bash
export REMOTE_MCP_URL='https://mcp.example.com/mcp'
scripts/phase36-public-mcp-smoke.sh
```

A successful unauthenticated run proves:

- public `/health` is reachable;
- Protected Resource Metadata is reachable and points at an HTTPS Authorization Server;
- `relay.coding` is advertised;
- unauthenticated MCP discovery receives a Bearer challenge.

It does **not** prove external MCP client OAuth or tool execution.

## 7. Verify with an owner token

Store one access token in a private file:

```bash
umask 077
printf '%s' "$ACCESS_TOKEN" > /tmp/ai-code-mcp-token
unset ACCESS_TOKEN
```

Then:

```bash
export REMOTE_MCP_ACCESS_TOKEN_FILE=/tmp/ai-code-mcp-token
scripts/phase36-public-mcp-smoke.sh
rm -f /tmp/ai-code-mcp-token
```

This additionally verifies authenticated `server/discover` and `tools/list` without executing a tool.

## 8. Hosted Nuxt client

For a single-owner hosted Nuxt deployment, configure:

```text
NUXT_REMOTE_MCP_URL=https://mcp.example.com/mcp
NUXT_REMOTE_MCP_OWNER_USER_ID=<ai-code-users-id>
NUXT_REMOTE_MCP_ACCESS_TOKEN=<owner-access-token>
```

Then, from the owner AI Code account, create/configure the matching MCP server and use the Settings MCP **Test** action.

The current path uses an externally issued access token and therefore needs normal token rotation/expiry handling. Do not solve expiry by creating a practically permanent bearer token.

## 9. Connect external MCP client

Once the public metadata and authenticated smoke checks pass, continue with [external-mcp.md](external-mcp.md).
