# Plan 032: Packages Layered Architecture Refactor

## Status
**CLOSED** (Completed in commit 8130756)
## Goal
Refactor the `packages` target to follow DRY, KISS, SOLID principles, and a Layered Architecture. Update the directory structure to adhere to best practices.

## Current Context / Assumptions
The `packages` directory currently contains:
- TypeScript wrappers/tools (`curl-tool`, `relay-agent`, `searxng-search-tool`, `terminal-tool`) with `package.json` and `src/index.ts`.
- A monolithic `rust-tools` package containing multiple binaries (`src/bin/*.rs`) and shared modules in `src/relay_agent/`. 
The `rust-tools/src/relay_agent` directory mixes routing, domain logic, configuration, and infrastructure (transport, terminal policy, admission).

## Proposed Approach
1. **Layered Architecture for Rust Codebase:** 
   Restructure `rust-tools/src` to separate concerns strictly according to Layered Architecture (Domain, Application, Infrastructure, Presentation/Interfaces).
2. **SOLID & DRY:**
   - Extract common interfaces (traits) for executing commands or routing requests.
   - Separate admission control and terminal policy from transport and dispatcher logic (Single Responsibility Principle).
   - Reuse common configuration and error handling code across binaries (DRY).
3. **KISS:**
   Keep the Typescript wrappers thin. Ensure the Rust layer's complexity is contained and easily testable.
4. **Foldering Reorganization:**
   Separate workspaces or internal libraries for common core logic rather than a single massive module, enabling better testability and boundaries.

## Step-by-step Plan

### Phase 1: Rust Workspace and Layered Structure Setup
1. Convert `rust-tools` into a proper Cargo Workspace with separate internal crates:
   - `core` (Domain layer: models, errors, core policies like `TerminalPolicy`).
   - `application` (Use cases, `Dispatcher`, `Admission` logic).
   - `infrastructure` (Implementations for `Transport`, `Security`, `Pidfile`, external integrations).
   - `interfaces` or `api` (MCP layer, `TransportValidation`).
   - `bin` (The actual CLI wrappers: `curl-tool.rs`, `relay-agent.rs`, etc., wiring everything together).

### Phase 2: Refactoring `relay-agent` Logic
1. **Extract Domain & Application:** Move logic out of `dispatcher.rs` and `execution.rs` into the `application` crate. Define clear traits in `core` that `application` uses.
2. **Abstract Infrastructure:** Move `transport.rs`, `observability.rs`, `auth.rs`, and `security.rs` into the `infrastructure` crate. Ensure they implement traits defined in the domain/application layer (Dependency Inversion).
3. **Simplify & DRY:** Combine duplicated CLI argument parsing or initial setup across the different binaries into a shared `bootstrap` or `utils` module.

### Phase 3: TypeScript Packages Reorganization
1. Group the TypeScript MCP/Langchain wrappers into a unified structure or a single monorepo package with multiple entry points, or keep them as standard isolated npm workspaces but with a shared `core-ts` library if they share logic (e.g. executing the rust binaries).
2. Ensure TS `index.ts` files follow a clean standard, removing boilerplate where possible.

## Files Likely to Change
- `packages/rust-tools/Cargo.toml` -> Will become a workspace.
- `packages/rust-tools/src/*` -> Will be moved into respective workspace crates (`core`, `application`, `infrastructure`, `presentation`).
- `packages/*/package.json` -> Potentially adding a shared TS internal dependency.

## Validation
- **Lint/Typecheck:** Must pass `pnpm verify:commit` and `cargo clippy` without warnings.
- **Manual QA:** Verify that the tools (`relay-agent`, `curl-tool`, etc.) still function correctly when invoked via their Typescript entrypoints.
