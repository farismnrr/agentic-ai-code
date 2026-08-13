# Plan 034: Server Layered Architecture Refactor

## Objective
Align the Nuxt `server/` directory with strict Layered Architecture, DRY, KISS, and SOLID principles. The primary focus is eliminating architectural bypasses (e.g., infrastructure logic hiding in Nuxt `utils/` auto-imports) and properly stratifying the domain, application, infrastructure, and presentation layers.

## Current Anti-Patterns
1. **Auto-Import Coupling:** Nuxt's `server/utils/` is being used to house infrastructure concerns (like `db.ts`, `mailer.ts`, `mcp-client.ts`, `langgraph-*.ts`). Because these are auto-imported, any layer can (and likely does) bypass dependency inversion and call infrastructure directly.
2. **Scattered Domain Logic:** Core logic like custom errors (`error.ts`, `http-errors.ts`) are floating at the root or in utils.
3. **Inconsistent Infrastructure:** Some infrastructure is properly abstracted under `server/infrastructure/`, while other pieces live in `server/utils/` or `server/database/`.

## Phased Execution

### Phase 1: Establish the Domain/Core Layer (`server/core/`)
- Create `server/core/` to house business rules, core entities, and custom errors.
- Move `server/error.ts` and `server/utils/http-errors.ts` into `server/core/errors/`.
- Goal: Create a pure inner layer that does not depend on Nuxt, Nitro, or external infrastructure.

### Phase 2: Purge Infrastructure from Auto-Imports (`server/utils/`)
Move all infrastructure implementations out of `utils/` to prevent implicit coupling, placing them into explicit infrastructure submodules:
- `utils/db.ts` -> `infrastructure/database/connection.ts`
- `utils/is-unique-violation.ts` -> `infrastructure/database/errors.ts`
- `utils/fs-browse.ts` -> `infrastructure/filesystem/browse.ts`
- `utils/mailer.ts` -> `infrastructure/mail/mailer.ts`
- `utils/mcp-client.ts` -> `infrastructure/mcp/client.ts`
- `utils/otel.ts` -> `infrastructure/observability/otel.ts`
- `utils/rate-limit.ts` -> `infrastructure/network/rate-limit.ts`
- `utils/ssrf-guard.ts` -> `infrastructure/security/ssrf-guard.ts`
- `utils/token.ts` -> `infrastructure/security/token.ts`
- `utils/langgraph-*.ts` -> `infrastructure/ai/langgraph/`
- `utils/models.ts` -> Move to either `core/` or `application/` depending on its contents.

### Phase 3: Enforce Dependency Inversion in the Presentation Layer (`api/` & `routes/`)
- Review all API endpoints and routes.
- Ensure they *only* interact with the system via the composition root (`event.context.application`).
- Any direct imports of `infrastructure/` (formerly auto-imported from `utils/`) by the presentation layer must be refactored into use-cases within the `application/` layer.

### Phase 4: Validation & Cleanup
- Ensure Drizzle schemas and migrations remain contained in `server/database/` (as tooling configuration) but are only accessed via repositories in `infrastructure/database/`.
- Run `pnpm typecheck` to fix any broken explicit imports.
- Run `pnpm lint` and `pnpm verify:commit` to ensure architectural boundaries pass.
- Delete the `server/utils/` directory entirely if it becomes empty to prevent future violations.

## Target Folder Structure (Post-Refactor)

```text
server/
├── core/                       <-- Domain logic, Custom Errors, and Interfaces
│   └── errors/
│       ├── index.ts            (formerly server/error.ts)
│       └── http.ts             (formerly server/utils/http-errors.ts)
├── application/                <-- Use cases / business logic
│   ├── account-data.ts
│   ├── auth.ts
│   ├── ...
├── infrastructure/             <-- Implementations of interfaces / external systems
│   ├── database/
│   │   ├── connection.ts       (formerly server/utils/db.ts)
│   │   ├── errors.ts           (formerly server/utils/is-unique-violation.ts)
│   │   ├── ... (repositories like auth.ts, models.ts, etc.)
│   ├── mail/
│   │   └── mailer.ts           (formerly server/utils/mailer.ts)
│   ├── mcp/
│   │   └── client.ts           (formerly server/utils/mcp-client.ts)
│   ├── network/
│   │   └── rate-limit.ts       (formerly server/utils/rate-limit.ts)
│   ├── observability/
│   │   └── otel.ts             (formerly server/utils/otel.ts)
│   ├── security/
│   │   ├── ssrf-guard.ts       (formerly server/utils/ssrf-guard.ts)
│   │   ├── token.ts            (formerly server/utils/token.ts)
│   ├── ai/
│   │   └── langgraph/          (formerly server/utils/langgraph-*.ts)
│   └── composition/
│       └── application.ts      (Composition Root)
├── database/                   <-- DB specific definitions (Schema, Migrations)
│   ├── schema.ts
│   └── migrations/
├── api/                        <-- Presentation layer (Nuxt Nitro API)
├── routes/                     <-- Presentation layer (Nuxt Nitro Routes)
├── middleware/                 <-- Presentation layer (Nuxt Nitro Middleware)
└── plugins/                    <-- Nuxt Server Plugins
```

