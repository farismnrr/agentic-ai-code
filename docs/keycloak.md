# Keycloak for the remote MCP relay

Keycloak is the reference external OAuth/OIDC Authorization Server for the remote `ai-tools relay` deployment. The relay itself stays provider-neutral.

The repository's current operator deployment has been proven with Keycloak 26.7.0, but these steps describe the required contract rather than requiring one exact hosting layout.

## Goal

For examples below, assume:

```text
Authorization Server: https://auth.example.com/realms/masihawam
MCP resource:          https://mcp.example.com/mcp
required scope:        relay.coding
```

The final access token presented to the relay must have:

- `iss` equal to the configured Keycloak realm issuer;
- `aud` containing the exact MCP resource URL;
- `sub` equal to the single owner configured in `OAUTH_OWNER_SUBJECT`;
- scope containing `relay.coding`;
- a valid asymmetric signature discoverable through the realm JWKS endpoint;
- valid time claims.

## 1. Publish Keycloak through HTTPS

Run Keycloak behind a stable public HTTPS hostname. The current reference pattern keeps Keycloak itself on loopback and publishes it through an outbound tunnel/edge.

Do not configure the relay with an internal/loopback issuer. `OAUTH_ISSUER` must be the exact public canonical issuer that Keycloak advertises in its OIDC discovery document.

Check the realm discovery endpoint in a browser or with curl:

```text
https://auth.example.com/realms/masihawam/.well-known/openid-configuration
```

Its `issuer` must exactly match the value you will configure on the relay.

## 2. Create a realm

Create a dedicated realm, for example `masihawam`.

Use Keycloak's normal asymmetric realm signing keys (RS256 works with the current deployment). The relay discovers the signing keys through standard OIDC discovery/JWKS; do not copy a private signing key into AI Code.

## 3. Create the owner user

Create or select the human account allowed to operate the coding relay.

The relay is intentionally single-owner. Obtain the stable subject (`sub`) issued for this account and store it privately as:

```text
OAUTH_OWNER_SUBJECT=<subject>
```

Do not commit the subject/token to the repository docs or examples.

## 4. Create the `relay.coding` scope

Create a client scope named:

```text
relay.coding
```

Ensure clients used for the MCP flow can request/receive this scope. The relay rejects otherwise-valid tokens that do not carry it.

## 5. Map the MCP resource into `aud`

Configure an audience mapper so access tokens intended for the relay contain the exact canonical MCP resource:

```text
https://mcp.example.com/mcp
```

In current Keycloak terminology, create/select the client scope used for MCP, add an **Audience** protocol mapper, put the exact MCP URL in **Included Custom Audience**, and enable **Add to access token**. Then link that scope to the external MCP client/MCP client as default or optional according to your policy.

Keycloak's current server-administration guide documents this under token audience / Audience protocol mapper configuration: [Keycloak Server Administration Guide](https://www.keycloak.org/docs/latest/server_admin/).

Do not use only the hostname and do not omit `/mcp`; the relay compares the token audience against the configured resource contract.

Keep the audience mapper tied to the MCP client/scope rather than globally adding the coding resource to unrelated tokens.

## 6. Enable authorization code + PKCE S256

Interactive clients such as external MCP client must use an authorization-code flow with PKCE. Configure Keycloak/client policy so public/interactive MCP clients can use PKCE S256 and do not require exposing a static client secret inside external MCP client.

The Rust relay does not implement this flow; it only receives and validates the resulting access token.

## 7. Configure client identification/registration

external MCP client's exact supported client-registration UI can evolve, so use the mode supported by the current external MCP client connection flow:

- Client ID Metadata Document (preferred when supported by the Authorization Server/client combination);
- an intentionally pre-registered client; or
- OIDC dynamic client registration (DCR) when enabled and policy-restricted.

The proven operator profile uses Keycloak's public dynamic-registration endpoint for MCP compatibility. If you enable anonymous DCR, restrict it aggressively:

- allow only the required MCP scope;
- require consent where appropriate;
- restrict trusted redirect/callback hosts to the current external MCP client hosts shown by the live connection UI;
- reject arbitrary/untrusted redirect origins;
- do not guess a callback URI from an old session and permanently allowlist it.

Use the **exact callback URI displayed by external MCP client when creating the connection**.

### Refresh-token support for external MCP client

For persistent external MCP client connectivity, configure Keycloak so the client can obtain refresh tokens when external MCP client requests them. OpenAI's current OIDC guidance recommends `offline_access` be supported/advertised for this purpose. Keep `relay.coding` as the relay permission scope; `offline_access` is a client-session/token-lifecycle capability.

## 8. Verify Keycloak before starting external MCP client

Verify these independently:

1. OIDC discovery is reachable over public HTTPS.
2. The discovery document's `issuer` matches exactly.
3. JWKS is reachable and contains an asymmetric signing key.
4. An owner login can request `relay.coding`.
5. The resulting token is audience-bound to the exact MCP resource.
6. The token subject is the intended owner.

Do not paste a real token into issue comments, shell history, documentation, or chat transcripts. For repository smoke tests, put the token in a mode-0600 file and use `REMOTE_MCP_ACCESS_TOKEN_FILE`; the provided script deliberately keeps it out of process arguments and output.

## 9. Relay values derived from Keycloak

Once Keycloak is correct, the relay needs only the validation contract:

```bash
export OAUTH_ISSUER='https://auth.example.com/realms/masihawam'
export OAUTH_OWNER_SUBJECT='<stable-owner-sub>'
export REMOTE_MCP_URL='https://mcp.example.com/mcp'
```

`scripts/phase36-start-remote-relay.sh` uses `REMOTE_MCP_URL` as the relay's OAuth audience.

## What Keycloak is not used for

Keycloak is not currently the primary login mechanism for the AI Code Nuxt UI. Application login remains session/email plus optional Google/GitHub OAuth. Keycloak's role here is specifically the remote MCP Authorization Server.
