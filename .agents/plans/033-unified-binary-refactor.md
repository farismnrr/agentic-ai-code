# Plan 033: Unified CLI Binary Refactor

## Status
**CLOSED**

## Goal
Consolidate the separate Rust CLI tools (`curl-tool`, `relay-agent`, `searxng-search-tool`, `terminal-tool`) into a single executable binary. This will drastically simplify the consumption pattern for the Nuxt application and external consumers, requiring only a single binary artifact to be distributed and invoked.

## Current Context
The `packages/rust-tools/cli/src/bin/` directory currently outputs four distinct binaries. Each tool parses its own CLI arguments (using `clap`) and runs in its own context. The Nuxt backend and other integrations have to resolve and call these different binaries separately.

## Proposed Approach
1. **Single Entry Point**: Create a `src/main.rs` in `packages/rust-tools/cli`.
2. **Subcommand Architecture**: Use `clap`'s `Subcommand` feature to route commands. The interface will look something like:
   - `ai-tools relay-agent [ARGS]`
   - `ai-tools curl [ARGS]`
   - `ai-tools searxng [ARGS]`
   - `ai-tools terminal [ARGS]`
3. **Module Refactoring**: 
   - Rename the files in `src/bin/` to regular modules in `src/commands/` (e.g., `src/commands/curl.rs`, `src/commands/relay.rs`, etc.).
   - Wrap their `main()` functions into async functions like `pub async fn run(args: CurlArgs) -> anyhow::Result<()>`.
4. **TypeScript Consumption Updates**: 
   - Update the TypeScript wrappers in `packages/curl-tool`, `packages/relay-agent`, `packages/terminal-tool`, and `packages/searxng-search-tool` to call the unified binary with the corresponding subcommand prefix instead of looking for distinct binary names.
5. **Legacy Code Cleanup**: Aggressively remove the old `src/bin/*.rs` files, any unused legacy wrappers or scripts, and redundant boilerplate that causes confusion. Adjust `Cargo.toml` to build the unified `[[bin]]` only.

## Files Likely to Change
- `packages/rust-tools/cli/Cargo.toml` -> Define a single `[[bin]]` or default binary name.
- `packages/rust-tools/cli/src/bin/*.rs` -> Moved to `src/commands/*.rs` or `src/*.rs`.
- `packages/rust-tools/cli/src/main.rs` -> New entry point containing the CLI router.
- `packages/*/src/index.ts` -> Change execution path to `unified-binary sub-command ...`.

## Validation
- **Lint/Typecheck:** Must pass `pnpm verify:commit`, `cargo fmt --all`, and `cargo clippy`.
- **Manual QA:** Ensure the Nuxt app can still invoke the tools via the TS wrappers correctly, and verify that the unified binary executes all subcommands without issues.
