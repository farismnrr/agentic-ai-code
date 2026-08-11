# 027 Exit Code Contract

**Date:** 2026-08-11
**Related Plan:** [027-cli-rust-refactor.md](../plans/027-cli-rust-refactor.md)

## Documented Exit Code Semantics

This document outlines the exit code semantics for all 3 migrated Rust CLIs (`terminal-tool`, `curl-tool`, `searxng-search-tool`).

- `0` → **Success**: The command was executed and completed as expected. In some cases, expected contractual errors (like a blocked SSRF request or an empty command string) will also exit with 0 to match the JS oracle's behavior, emitting `Error: ...` to stdout.
- `1` → **Application/Runtime Error**: An underlying execution failure occurred, such as an actual connection error, command timeout, missing binary, or OS-level failure.
- `2` → **Invalid CLI Usage**: The user passed bad arguments, missing required options, or unknown flags to the CLI. This is handled by the `clap` argument parsing library by default.
