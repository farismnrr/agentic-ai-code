# SSO Auth

Lightweight GitHub SSO service for Masih Awam AI Code.

## Runtime

- Rust + Axum backend
- Svelte + Vite frontend
- Tailwind CSS + DaisyUI
- Svelte builds to static assets served by the Rust process
- one runtime process and one port

## Current scope

The current milestone only establishes the service shell. GitHub OAuth, sessions, and MCP integration come next.

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md).

## Guardrail

Run from `sso-auth/`:

```sh
npm run guardrail
```

The guardrail enforces source-file budgets, folder density, and inward Clean Architecture dependency boundaries. It complements review; SOLID and DRY still require design judgment.
