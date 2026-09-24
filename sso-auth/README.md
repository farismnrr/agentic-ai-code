# SSO Auth

Lightweight GitHub SSO service for Masih Awam AI Code.

## Runtime

- Rust + Axum backend
- Svelte + Vite frontend
- Tailwind CSS + DaisyUI
- frontend assets are embedded into the release Rust binary
- one runtime binary, one process, and one port
- production-only Docker workflow; no hot-reload runtime

## Docker build cache

Docker's normal BuildKit layer cache is used by default.

The Dockerfile is arranged so unchanged work is reused:

- Node dependencies are cached until `package.json` changes.
- npm's download cache is persisted with a BuildKit cache mount.
- Rust dependencies are compiled separately with `cargo-chef`.
- Rust dependency compilation is reused while `Cargo.toml` dependency metadata is unchanged.
- a frontend-only change rebuilds the Svelte assets and application binary, not all Rust dependencies.
- an application Rust change recompiles the application while reusing dependency layers.
- there are no source bind mounts or hot reload.

Do not use `--no-cache` for normal rebuilds.

## Run

From the repository root:

```sh
docker compose up -d --build --force-recreate sso-auth
```

Docker automatically reuses every valid cached layer and rebuilds only invalidated steps.

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md).

## Guardrail

Run from `sso-auth/`:

```sh
npm run guardrail
```

The guardrail enforces source-file budgets, folder density, and inward Clean Architecture dependency boundaries. It complements review; SOLID and DRY still require design judgment.
