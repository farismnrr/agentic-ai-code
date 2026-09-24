# SSO Auth

Lightweight GitHub SSO service for Masih Awam AI Code.

## Runtime

- Rust + Axum backend
- Svelte + Vite frontend
- Tailwind CSS + DaisyUI
- frontend assets are embedded into the release Rust binary
- one runtime binary, one process, and one port
- production-only Docker workflow; no hot-reload runtime

## Current scope

The current milestone establishes the service shell. GitHub OAuth, sessions, and MCP integration come next.

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md).

## Docker

The Docker build compiles Svelte first, embeds its generated assets into the Rust release binary, and copies only the binary plus CA certificates into the runtime image.

From the repository root:

```sh
docker compose build sso-auth
docker compose up -d --force-recreate sso-auth
```

To rebuild and replace the running service after source changes:

```sh
docker compose up -d --build --force-recreate sso-auth
```

## Guardrail

Run from `sso-auth/`:

```sh
npm run guardrail
```

The guardrail enforces source-file budgets, folder density, and inward Clean Architecture dependency boundaries. It complements review; SOLID and DRY still require design judgment.
