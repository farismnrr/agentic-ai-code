# Plan 041A — LSP Capability Completion

**Status:** CLOSED / VERIFIED / MERGED (2026-08-19)
**Parent:** [Plan 041](041-code-intelligence-and-platform-polish-roadmap.md)
**Depends on:** Plan 040 CLOSED / VERIFIED

## Goal

Revisit the remaining real LSP/code-intelligence gaps on current `main` and improve only capabilities that can be supported cleanly by the installed/reviewed language-server stack.

## Current baseline to re-verify

Known historical limitations include:

- Rust workspace-symbol search currently reports unsupported in the relay;
- Vue definition/references/hover/diagnostics have previously returned bounded empty results with the installed TypeScript/Vue integration while Vue document symbols worked.

These are starting hypotheses, not immutable truth. Re-check current language-server versions/capabilities before implementation.

## Scope

- inspect server-advertised capabilities for Rust, TypeScript and Vue;
- enable Rust workspace symbols if rust-analyzer supports the needed bounded contract safely;
- re-test Vue language intelligence with current official Vue/TypeScript plugin integration;
- upgrade/configure reviewed language-server dependencies only when required and justified;
- preserve protected-path containment, workspace scoping, process cleanup, network isolation and result bounds;
- add truthful capability metadata/unsupported results where upstream still cannot satisfy a feature.

## Security / correctness requirements

- no custom parser/indexer as a workaround;
- no unrestricted LSP command passthrough;
- server-returned paths must remain contained and protected-path filtered;
- stale document/version handling remains explicit;
- malformed/oversized protocol frames remain bounded;
- language server remains an untrusted subprocess under the existing sandbox.

## Acceptance

At minimum verify real fixtures for:

- Rust symbols/workspace symbols, definition, references, hover, diagnostics;
- TypeScript definition/references/hover/diagnostics;
- Vue document symbols and any newly proven definition/references/hover/diagnostics capability;
- unsupported capability responses when a server still cannot provide a result;
- protected-path/sibling-workspace denial;
- crash/hang/cancellation/process reaping.

## Verified implementation outcome

- Rust `code_symbols(query=...)` now uses rust-analyzer's advertised `workspaceSymbolProvider` through the existing bounded semantic normalization path; no custom index/parser was added.
- The code-MCP acceptance now proves a real Rust workspace-symbol query, bounded pagination behavior, and no absolute-host-path disclosure.
- TypeScript/Vue acceptance was re-run against the reviewed host stack. Vue document symbols remain healthy; Vue definition/references/hover/diagnostics remain bounded-empty and are documented truthfully rather than claimed as semantic parity.
- The host currently provides `rust-analyzer 1.95.0`, `typescript-language-server 5.3.0`, and `vue-language-server 3.3.8`. No language-server upgrade is required for the proven Rust improvement.
- `pnpm verify:041a` passed three consecutive runs, `pnpm verify:commit` passed, `cargo test --workspace --locked` passed, and `git diff --check` passed.

## Exit criteria

- practical LSP capabilities improve where the upstream stack genuinely supports them;
- remaining limitations are explicitly documented and tested, never falsely claimed;
- relevant LSP/code MCP gates and full commit gate pass;
- final review finds zero unresolved P0/P1;
- 041A is merged before 041B begins.
