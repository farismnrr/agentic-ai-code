# Authentication

AI Code has two separate authentication domains. Keeping them separate prevents a lot of setup confusion.

## 1. AI Code application login

The Nuxt application uses `nuxt-auth-utils` sealed-cookie sessions.

Supported application account flows include:

- email/password registration and login;
- email verification;
- password reset through SMTP;
- optional external OAuth login.

Authenticated browser sessions are owner-scoped server records whose bearer
secret is sealed in the httpOnly cookie and hashed in PostgreSQL. Users can
revoke individual sessions or sign out other browsers from Account settings.
Password reset and confirmed email changes invalidate existing sessions.
TOTP and single-use recovery codes are available as an MFA foundation; their
seeds/codes are never stored in plaintext. See [security.md](security.md) for
the exact invariants and the remaining deployment-role acceptance boundary.

Relevant configuration lives under the session, SMTP, and application OAuth sections of `.env.example`.

This authentication controls access to AI Code's UI and server APIs.

## 2. Remote MCP authorization

The Rust `ai-tools relay` has a different job: it protects coding-machine tools exposed over MCP.

In remote mode the relay is an OAuth **Resource Server**. It does not show a login page and does not issue tokens. A separate standards-compatible Authorization Server handles interactive login and token lifecycle.

The relay validates:

- canonical HTTPS issuer;
- expected MCP audience/resource;
- asymmetric JWT signature through OIDC discovery/JWKS;
- token time validity;
- a stable allowed owner `sub`;
- the `relay.coding` scope.

See [oauth-provider.md](oauth-provider.md) for the provider-neutral Authorization Server setup.

## Two owner identifiers

A hosted Nuxt deployment may use a server-side first-party relay token. This creates two independent ownership checks:

| Identifier | Meaning |
| --- | --- |
| `NUXT_REMOTE_MCP_OWNER_USER_ID` | AI Code's own `users.id`; decides which application account may cause Nitro to attach the private first-party MCP token. |
| `OAUTH_OWNER_SUBJECT` | Authorization Server JWT `sub`; decides which external OAuth identity the Rust relay accepts. |

Do not substitute one for the other.

## MCP client authentication

An external MCP client does not use the Nuxt application's private relay token. It connects directly to the public MCP resource and completes its own OAuth flow with the Authorization Server.

That means a valid external-client setup requires all three pieces to agree on the same resource contract:

```text
MCP OAuth client
      -> Authorization Server
      -> access token for https://mcp.example.com/mcp
      -> ai-tools relay validates token
```

Continue with [oauth-provider.md](oauth-provider.md), [remote-mcp.md](remote-mcp.md), and [mcp-client.md](mcp-client.md).
