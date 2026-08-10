# CLI Refactor: JavaScript to Rust

## Goal

Refactor the existing JavaScript CLI into a Rust-based CLI while preserving the current user-facing behavior and keeping the migration incremental.

## Principles

- Preserve existing CLI commands and flags unless there is a deliberate breaking-change decision.
- Keep the Rust implementation modular and testable.
- Prefer explicit types and predictable error handling over implicit JavaScript behavior.
- Avoid a big-bang rewrite; migrate command-by-command.
- Keep the existing CLI usable until the Rust replacement reaches feature parity.

## Phase 0 — Inventory & Baseline

- [ ] Identify the current JavaScript CLI entrypoint.
- [ ] Inventory all commands, subcommands, flags, arguments, environment variables, and config files.
- [ ] Document current stdout/stderr behavior and exit codes.
- [ ] Record external integrations and filesystem/network side effects.
- [ ] Add or verify smoke tests for the current JavaScript CLI.
- [ ] Define feature-parity criteria for the Rust implementation.

## Phase 1 — Rust CLI Foundation

- [ ] Create the Rust CLI crate and workspace structure.
- [ ] Select and configure the CLI argument parser (e.g. `clap`).
- [ ] Add structured application errors and consistent exit-code handling.
- [ ] Add configuration loading and environment-variable handling.
- [ ] Add logging/diagnostic output with a clear quiet/verbose strategy.
- [ ] Set up Rust formatting, linting, and tests in CI.

## Phase 2 — Core Migration

- [ ] Migrate shared domain/business logic out of CLI-specific code.
- [ ] Implement the first low-risk command in Rust.
- [ ] Add unit tests for command parsing and core behavior.
- [ ] Add integration/smoke tests against real CLI invocation.
- [ ] Compare Rust output, exit codes, and side effects with the JavaScript implementation.

## Phase 3 — Command-by-Command Parity

- [ ] Migrate remaining commands incrementally.
- [ ] Preserve backwards-compatible flags and arguments where practical.
- [ ] Replace JavaScript-specific utilities with Rust equivalents.
- [ ] Centralize API/client, filesystem, serialization, and process execution concerns.
- [ ] Add regression tests for every migrated command.

## Phase 4 — Distribution & UX

- [ ] Define release targets and supported platforms.
- [ ] Produce reproducible release binaries.
- [ ] Decide how the Rust binary is installed/distributed.
- [ ] Update package scripts/documentation to use the Rust CLI.
- [ ] Add version reporting and `--help` documentation.
- [ ] Benchmark startup time and common command execution against the JavaScript CLI.

## Phase 5 — Cutover

- [ ] Reach feature parity with the JavaScript CLI.
- [ ] Run both implementations against the same smoke/regression suite.
- [ ] Switch the default entrypoint to Rust.
- [ ] Keep a rollback path to the JavaScript CLI for one release cycle.
- [ ] Remove the JavaScript CLI and obsolete dependencies after the migration is validated.
- [ ] Update project documentation and contributor instructions.

## Definition of Done

- All supported commands have Rust implementations.
- Existing documented CLI workflows continue to work.
- CLI tests cover parsing, success paths, failures, and exit codes.
- CI builds and tests the Rust CLI on supported platforms.
- Release artifacts are reproducible and documented.
- JavaScript CLI code and dependencies are removed only after successful cutover.
