# OAuth/OIDC Authorization Server for the remote MCP relay

The relay is an OAuth **Resource Server**. It validates access tokens but does
not display a login page, issue tokens, register clients, or store signing
private keys. A separate standards-compatible OAuth/OIDC Authorization Server
owns those responsibilities.

The examples below use placeholders:

```text
Authorization Server: https://auth.example.com/tenant/example
MCP resource:         https://mcp.example.com/mcp
required scope:       relay.coding
```

The exact discovery path is determined by the Authorization Server. Its OIDC
discovery document must publish an `issuer` that exactly matches
`OAUTH_ISSUER`, plus a JWKS URI containing an asymmetric signing key.

## Token contract

The access token presented to the relay must contain:

- `iss` equal to the configured public issuer;
- `aud` containing the exact canonical MCP resource URL, including `/mcp`;
- a stable `sub` equal to `OAUTH_OWNER_SUBJECT`;
- `relay.coding` in `scope` (or the provider's equivalent scope claim);
- a valid asymmetric signature whose key is discoverable through JWKS; and
- valid `exp`, `nbf`, and related time claims.

Keep the resource URL and audience exact. A hostname without the MCP path is
not equivalent to the MCP resource.

## 1. Publish the Authorization Server through HTTPS

Use a stable public HTTPS issuer. The Authorization Server may stay on a
private network or loopback when an approved reverse proxy or outbound tunnel
publishes its canonical issuer. The issuer configured in the relay must be the
public URL advertised by discovery, not an internal address.

The edge must forward the discovery, authorization, token, and JWKS routes
without rewriting their public URLs. Do not put a second interactive login
page in front of the MCP resource's OAuth metadata routes.

## 2. Create an owner identity

Create or select the one human/service identity allowed to operate the relay.
Record its stable subject claim privately:

```text
OAUTH_OWNER_SUBJECT=<stable-owner-subject>
```

The relay intentionally supports a single configured owner subject. Do not
place a real subject or token in source, documentation, shell history, or
issue discussions.

## 3. Define the relay permission scope

Create or reserve the scope:

```text
relay.coding
```

Allow only the intended MCP client registration to request this scope. The
relay rejects an otherwise valid token without it.

## 4. Bind the MCP resource to the token audience

Configure an audience/resource mapping so tokens issued for this client carry:

```text
https://mcp.example.com/mcp
```

Keep this mapping scoped to the MCP client or permission scope. Do not add the
coding resource to unrelated tokens globally.

## 5. Configure authorization code + PKCE

Interactive clients should use authorization code with PKCE S256. Public
clients must not need a static client secret embedded in the client UI.

Client registration may use one of these standards-compatible models:

- pre-register a client with its exact redirect URI;
- use a Client ID Metadata Document when supported; or
- enable dynamic client registration with strict scope, redirect, and rate
  limits.

If dynamic registration is enabled, allow only approved redirect origins and
the required MCP scope. Never accept wildcard redirects or arbitrary client
metadata without policy review. Register the exact callback URI presented by
the connecting client for the current environment.

## 6. Configure refresh-token behavior

For persistent connections, allow refresh tokens according to the
Authorization Server's normal policy. A client may request `offline_access`
when supported. This is a token-lifecycle capability, not a replacement for
`relay.coding`.

## 7. Verify the Authorization Server

Verify independently before starting the relay:

1. discovery is reachable over public HTTPS;
2. discovery's `issuer` exactly matches `OAUTH_ISSUER`;
3. JWKS is reachable and has an asymmetric key;
4. the owner can request `relay.coding`;
5. the resulting token audience is the exact MCP resource; and
6. the token subject is the intended owner.

Use a mode-0600 token file for smoke tests. Do not place a token in command
arguments or logs.

## 8. Configure the relay

The relay needs only the validation contract:

```bash
export RELAY_AGENT_MODE=remote
export OAUTH_ISSUER='https://auth.example.com/tenant/example'
export OAUTH_AUDIENCE='https://mcp.example.com/mcp'
export OAUTH_OWNER_SUBJECT='<stable-owner-subject>'
```

`OAUTH_AUDIENCE` must match both the public MCP URL and the token audience.
Keep the relay loopback-only and configure the trusted proxy CIDR narrowly as
described in [remote-mcp.md](remote-mcp.md).

## Application login versus relay authorization

The web application's session/OAuth login and the relay's resource-server
authorization are separate domains. A web-account identifier must not be used
as the relay token's `sub` unless the same identity system intentionally
issues that claim.
