# Plan 036 — Reference Deployment Profile

Status: provider-neutral source contract implemented; the current operator profile uses Cloudflare Tunnel + Keycloak, with hosted-Nuxt and remaining OAuth/negative-case evidence still open.

## Purpose

Plan 036 keeps the protocol and security boundaries provider-neutral, but implementation needs one concrete deployment profile that can be exercised end to end. The reference profile is:

- public edge / outbound tunnel: **Cloudflare Tunnel (`cloudflared`)**;
- Authorization Server / identity provider: **external OAuth/OIDC provider; current operator deployment uses Keycloak 26.7.0**;
- MCP Resource Server: the existing Rust `ai-tools relay`;
- canonical resource: `https://mcp.farismunir.my.id/mcp`;
- hosted application client: Nuxt/Nitro using the first-party MCP 2026 adapter;
- external interactive client: ChatGPT using its MCP OAuth flow.

Cloudflare and Keycloak are the current concrete operator providers, not protocol dependencies. Another tunnel or Authorization Server is acceptable only if it preserves the same contracts below.

## Network contract

The laptop must create the public path outbound-only:

```text
ChatGPT / hosted Nuxt
        |
        | HTTPS
        v
mcp.farismunir.my.id
        |
        | Cloudflare Tunnel
        v
cloudflared on laptop
        |
        | HTTP over IPv4 loopback only
        v
127.0.0.1:47821
        |
        v
ai-tools relay --mode remote
```

The relay must not bind a public interface. `scripts/phase36-start-remote-relay.sh` fixes the trusted-proxy CIDR to `127.0.0.1/32`. The Cloudflare config generator deliberately emits `http://127.0.0.1:<port>` rather than `localhost` so the tunnel cannot unexpectedly connect over `::1` while the relay trusts only IPv4 loopback.

The public edge terminates HTTPS. The relay sees a loopback peer and accepts `X-Forwarded-Proto: https` only because all three conditions are true:

1. remote mode is explicit;
2. trusted-proxy mode is explicit;
3. the direct peer address is inside `127.0.0.1/32`.

A caller reaching the relay through any other peer cannot make an arbitrary forwarded-proto header count as HTTPS.

## Cloudflare Tunnel profile

Generate a locally-managed tunnel config with:

```bash
export CLOUDFLARED_TUNNEL_ID='<tunnel uuid>'
export CLOUDFLARED_CREDENTIALS_FILE="$HOME/.cloudflared/<tunnel uuid>.json"
export REMOTE_MCP_URL='https://mcp.farismunir.my.id/mcp'

scripts/phase36-cloudflared-config.sh > "$HOME/.cloudflared/config.yml"
cloudflared tunnel ingress validate
cloudflared tunnel run "$CLOUDFLARED_TUNNEL_ID"
```

The resulting ingress shape is intentionally small:

```yaml
ingress:
  - hostname: mcp.farismunir.my.id
    service: http://127.0.0.1:47821
  - service: http_status:404
```

Operational rules:

- do not create a router/NAT port-forward to `47821`;
- do not change relay bind host to `0.0.0.0`;
- do not point cloudflared at another LAN address;
- do not put a Cloudflare Access login page in front of `/mcp` or the OAuth well-known routes: MCP OAuth must remain discoverable directly by ChatGPT and other MCP clients;
- WAF, DDoS controls, request-size limits, and rate controls may protect the edge as long as they preserve MCP HTTP semantics and OAuth discovery/challenges;
- prefer a cache-bypass rule for `/mcp` and `/.well-known/oauth-protected-resource*` so authorization metadata changes are observed immediately during rollout;
- keep tunnel credentials outside the repository.

## Relay startup profile

Required runtime values:

```text
REMOTE_MCP_URL=https://mcp.farismunir.my.id/mcp
OAUTH_ISSUER=<canonical HTTPS Authorization Server issuer>
OAUTH_OWNER_SUBJECT=<the one allowed human subject>
EXECUTION_ROOT=<user-owned project/workspace root>
```

Start with:

```bash
scripts/phase36-start-remote-relay.sh
```

The repository wrapper explicitly supplies `RELAY_TOOL_PROFILE=full` for the
ChatGPT-facing remote relay, overriding an inherited `primary` setting so the
client discovers the complete reviewed tool catalog. The Rust CLI uses the
same `full` default for direct and other deployments.

That wrapper launches the existing binary equivalent of:

```text
ai-tools relay
  --mode remote
  --trusted-proxy
  --trusted-proxy-cidr 127.0.0.1/32
  --oauth-issuer <issuer>
  --oauth-audience https://mcp.farismunir.my.id/mcp
  --oauth-owner-subject <owner sub>
  --execution-root <root>
```

The existing Rust startup checks still remain authoritative: Linux/Bubblewrap, non-root execution, safe execution root, loopback bind, canonical HTTPS issuer/audience, and trusted proxy validation.

## Authorization Server compatibility profile

The relay is provider-neutral. Auth0 was the initial source-level reference because it is a documented MCP-compatible identity-provider option; the current operator deployment uses the existing Keycloak 26.7.0 service instead. Either profile is acceptable only when it preserves the same OAuth/OIDC, PKCE, resource/audience, owner, scope, JWKS, and client-registration contracts.

Required Authorization Server behavior:

- issuer and discovery metadata are public HTTPS;
- asymmetric signing keys are published through JWKS;
- the MCP API/resource identifier is exactly `https://mcp.farismunir.my.id/mcp`;
- `relay.coding` is an allowed/requestable scope;
- the interactive owner's token has the stable `sub` configured as `OAUTH_OWNER_SUBJECT`;
- authorization code + PKCE S256 is enabled for ChatGPT;
- the `resource` value sent by ChatGPT is honored/bound to the resulting access token audience;
- ChatGPT client identification uses a supported standard path (prefer CIMD when the tenant/profile supports it; a predefined client or DCR is acceptable when intentionally configured);
- only the callback URI shown by the current ChatGPT connection UI is allowlisted; do not guess or permanently hardcode a callback identifier from another connection.

The relay remains generic and contains no provider-specific token parsing. Tokens from the selected Authorization Server must satisfy the relay's normal issuer, audience, signature, time, owner-subject, and `relay.coding` checks.

## Hosted Nuxt credential profile

The current source slice accepts an externally-issued access token through private Nitro runtime config:

```text
NUXT_REMOTE_MCP_URL=https://mcp.farismunir.my.id/mcp
NUXT_REMOTE_MCP_OWNER_USER_ID=<ai_code.users.id for the single app owner>
NUXT_REMOTE_MCP_ACCESS_TOKEN=<owner access token>
```

Two independent owner identities are intentional:

- `NUXT_REMOTE_MCP_OWNER_USER_ID` is the ai-code application's database user id. It controls which authenticated ai-code account is allowed to make Nitro use the private first-party relay credential.
- `OAUTH_OWNER_SUBJECT` is the external Authorization Server's `sub` claim. It controls which OAuth identity the Rust relay will accept.

The access token is attached only when **both** the stored HTTP MCP server URL exactly matches the configured resource and the row's authoritative `userId` matches `NUXT_REMOTE_MCP_OWNER_USER_ID`. This multi-tenant guard is required: URL matching alone would let another authenticated ai-code user create their own row for the same resource and cause Nitro to reuse the operator's token.

The token and owner binding remain private runtime config. They are never written into the MCP server database row or returned to the browser.

This is the first deployable single-owner path, not the final token-lifecycle solution. Short-lived access-token expiry/rotation remains operational work until a reviewed refresh/linking flow is implemented. Do not work around expiry by issuing an effectively permanent bearer token.

ChatGPT does not use the Nuxt token. ChatGPT independently performs OAuth against the same Authorization Server and presents its own owner-authorized access token to the same MCP Resource Server.

## Acceptance order

Provisioning should be proven in this order so failures stay attributable:

1. start relay locally in remote mode;
2. start the outbound tunnel;
3. confirm public `/health`;
4. confirm public Protected Resource Metadata;
5. confirm unauthenticated `server/discover` gets a Bearer challenge;
6. issue an owner token and run `scripts/phase36-public-mcp-smoke.sh` with `REMOTE_MCP_ACCESS_TOKEN_FILE`;
7. configure hosted Nuxt with the same resource, the single owner's ai-code user id, and an owner token, then use Settings -> MCP servers -> Test from that owner account;
8. verify a different ai-code user cannot use the first-party credential even if they create an MCP row containing the same public URL;
9. connect ChatGPT in developer mode, complete OAuth, inspect tools, then test a safe tool call before terminal execution;
10. only after those pass, run a deliberately approved `terminal_exec` inside the configured execution root.

A metadata-only pass is not ChatGPT interoperability evidence, and a successful tool list is not terminal-execution evidence.

## Current operator deployment observation

As of 2026-08-15, the operator's existing Cloudflare Tunnel is the remotely
managed `farismunir-tunnel` (`3ea77293-142c-449f-9e4c-69d383ab4626`), reported
healthy by Wrangler. Its `mcp.farismunir.my.id` ingress is configured for
`http://127.0.0.1:47821` and retains the unrelated routes plus the final
`http_status:404` catch-all. Its existing `auth.farismunir.my.id` ingress is
configured for `http://127.0.0.1:8082` and serves the local Keycloak instance.
The local `cloudflared.service` owns the tunnel; this deployment does not use
a local `~/.cloudflared/config.yml`.

The `0.0.8-beta` `ai-tools` binary is installed at the operator's
`~/.local/bin/ai-tools` and runs as the unprivileged `farismnrr` user through
the persistent user service `ai-tools-relay.service`. The service keeps the
listener on `127.0.0.1:47821`, requires Bubblewrap, and reads its protected
issuer/owner configuration from a mode-0600 environment file.

The operator's existing Keycloak 26.7.0 deployment is the concrete
Authorization Server for this laptop deployment. It runs in Docker, listens
locally only on `127.0.0.1:8082`, and is published through the existing
remotely-managed tunnel route `auth.farismunir.my.id`. Realm `masihawam`
advertises issuer `https://auth.farismunir.my.id/realms/masihawam`, uses
RS256/JWKS, and has the `relay.coding` scope mapped to the canonical MCP
audience. The existing `farismnrr` human account supplies the stable owner
subject configured for the relay; the subject value is intentionally kept out
of this contract.

Keycloak's public OIDC dynamic-registration endpoint is enabled for MCP
compatibility. Anonymous registration is consent-protected, limited to the
requested `relay.coding` scope, and has a trusted-host policy allowing the
ChatGPT callback hosts (`chatgpt.com` and `chat.openai.com`) while rejecting an
untrusted synthetic redirect. An interactive ChatGPT connection was subsequently completed on 2026-08-16 and the live session successfully discovered the relay tools and invoked `terminal_exec`. That proves the connected client/tool path, but this repository does not expose the callback exchange or decoded token claims, so detailed callback/resource/audience assertions remain separately unproven.

## Remaining external evidence

This contract does not claim any of the following until they are observed against real accounts/deployment:

- an exposed Cloudflare tunnel token being rotated through the dashboard or a
  token with the required tunnel-write permission;
- an issued token with the exact expected audience/subject/scope;
- correct `NUXT_REMOTE_MCP_OWNER_USER_ID` ownership binding in the hosted deployment;
- hosted Nuxt -> public relay execution;
- negative proof that another ai-code user cannot consume the owner credential;
- independently captured ChatGPT OAuth callback/token-claim evidence beyond the successful connected session.
