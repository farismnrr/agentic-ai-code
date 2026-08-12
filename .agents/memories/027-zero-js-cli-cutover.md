# Plan 027 cut over the executable tool CLIs to Rust with no permanent JavaScript CLI fallback.

Plan 027 migrated only the executable command-line layer for `terminal-tool`, `curl-tool`, and `searxng-search-tool`. The TypeScript packages remain application-facing LangChain/AI SDK APIs; keeping those factories does **not** violate the Rust CLI cutover.

## Hard invariant

For these three tools, the supported standalone CLI path is the Rust workspace under `packages/rust-tools/`:

- `packages/rust-tools/src/bin/terminal-tool.rs`
- `packages/rust-tools/src/bin/curl-tool.rs`
- `packages/rust-tools/src/bin/searxng-search-tool.rs`

The old JavaScript executable layer was removed:

- no `packages/*-tool/bin/cli.mjs` entrypoint for the migrated CLIs;
- no npm `bin` mapping that launches those JavaScript CLIs;
- no permanent JavaScript fallback selector is part of the supported architecture.

Package documentation must therefore keep the distinction explicit: TypeScript tool factory APIs are valid, while standalone CLI examples must point at the Rust binaries rather than `npx @ai-code/*`.

## Why there is no JS fallback

A permanent fallback would create two executable implementations whose argument parsing, security checks, error behavior, and process lifecycle could drift. The migration deliberately chose one executable source of truth after parity/cutover evidence was collected.

If a native CLI regression requires rollback, use a known-good Rust artifact or revert the Rust integration/release change. Do not restore an indefinitely supported JavaScript CLI path merely as a compatibility escape hatch.

## Scope boundary

This decision does **not** mean the Nuxt application or the tool packages must become Rust. Nuxt/Vue/server code and TypeScript LangChain/AI SDK factories remain separate application concerns and were explicitly out of scope for Plan 027.

See [`../plans/027-cli-rust-refactor.md`](../plans/027-cli-rust-refactor.md) and [`027-final-closeout.md`](027-final-closeout.md) for the migration evidence and closeout.