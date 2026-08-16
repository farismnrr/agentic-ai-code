# Authentication

AI Code has two separate authentication domains. Keeping them separate prevents a lot of setup confusion.

## 1. AI Code application login

The Nuxt application uses `nuxt-auth-utils` sealed-cookie sessions.

Supported application account flows include:

- email/password registration and login;
- email verification;
- password reset through SMTP;
- optional Google OAuth login;
- optional GitHub OAuth login.

Relevant configuration lives under the session, SMTP, and Google/GitHub sections of `.env.example`.

This authentication controls access to AI Code's UI and server APIs.

## 2. Remote MCP authorization

The Rust `ai-tools relay` has a different job: it protects coding-machine tools exposed over MCP.

In remote mode the relay is an OAuth **Resource Server**. It does not show a login page and does not issue tokens. A separate Authorization Server such as Keycloak handles the interactive user login and token lifecycle.

The relay validates:

- canonical HTTPS issuer;
- expected MCP audience/resource;
- asymmetric JWT signature through OIDC discovery/JWKS;
- token time validity;
- a stable allowed owner `sub`;
- the `relay.coding` scope.

See [keycloak.md](keycloak.md) for the reference Authorization Server setup.

## Two owner identifiers

A hosted Nuxt deployment may use a server-side first-party relay token. This creates two independent ownership checks:

| Identifier | Meaning |
| --- | --- |
| `NUXT_REMOTE_MCP_OWNER_USER_ID` | AI Code's own `users.id`; decides which application account may cause Nitro to attach the private first-party MCP token. |
| `OAUTH_OWNER_SUBJECT` | Authorization Server JWT `sub`; decides which external OAuth identity the Rust relay accepts. |

Do not substitute one for the other.

## ChatGPT authentication

ChatGPT does not use the Nuxt application's private relay token. It connects directly to the public MCP resource and completes its own OAuth flow with the Authorization Server.

That means a valid ChatGPT setup requires all three pieces to agree on the same resource contract:

```text
ChatGPT OAuth client
      -> Authorization Server
      -> access token for https://mcp.example.com/mcp
      -> ai-tools relay validates token
```

Continue with [keycloak.md](keycloak.md), [remote-mcp.md](remote-mcp.md), and [chatgpt.md](chatgpt.md).
