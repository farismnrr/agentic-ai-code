# AI Code documentation

This directory is the operator and contributor handbook for AI Code.

If you are setting up a fresh installation, follow the pages in this order:

1. **[Getting started](getting-started.md)** — prerequisites, database, `.env`, install, migrations, and first run.
2. **[Configuration](configuration.md)** — understand the environment variables before exposing anything externally.
3. **[Authentication](authentication.md)** — distinguish AI Code's own user login from the OAuth boundary used by the remote MCP relay.
4. **[OAuth/OIDC provider](oauth-provider.md)** — configure the external Authorization Server used by the relay.
5. **[Remote MCP deployment](remote-mcp.md)** — start `ai-tools relay` in remote mode and publish it through an outbound HTTPS tunnel.
6. **[Connect an MCP client](mcp-client.md)** — add the public MCP endpoint to any compatible client and verify discovery/tool execution.

Reference pages:

- **[Architecture](architecture.md)** — trust boundaries and request flows.
- **[Security](security.md)** — account, session, MFA, HTTP policy, audit, and database operating boundaries.
- **[Development](development.md)** — repository workflow, stack-aware guardrails, tests, maintainability policy, and branches.
- **[Releases](releases.md)** — versioning and release artifact publication.
- **[Troubleshooting](troubleshooting.md)** — common setup and runtime failures.

## Documentation boundaries

`docs/` is written for humans operating or contributing to the project. It explains current behavior and links to implementation/configuration when that is the stronger source of truth.

`.agents/` serves a different purpose: it contains agent-facing knowledge, plans, frozen contracts, verification evidence, and durable implementation memory. Historical plan files are useful for archaeology, but they should not be treated as installation instructions when they disagree with current source or this handbook.

## Security rule

Never copy real credentials, bearer tokens, session secrets, private callback data, tunnel credentials, database passwords, or private keys into documentation. Keep secrets in ignored local environment files or the platform's secret manager.
