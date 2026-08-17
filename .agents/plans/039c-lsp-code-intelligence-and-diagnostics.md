# Plan 039C — LSP Code Intelligence and Diagnostics

**Status:** PLANNED  
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039B  

## Goal

Add real language-aware code intelligence using Language Server Protocol rather than custom parsing or semantic indexing, giving agents fast definitions, references, symbols, implementations, hover/type information, and diagnostics that complement lexical search.

## Why LSP

Modern coding agents use LSP because lexical search cannot reliably answer semantic questions such as which overload is referenced, where an interface is implemented, or what type error an edit introduced. LSP is already the industry protocol used by editors and current coding-agent plugin systems.

This repository must **adapt existing language servers**, not build a compiler front end, AST database, or custom repository index.

## Initial capability surface

- `code_symbols`
- `code_definition`
- `code_references`
- `code_implementations`
- `code_hover`
- `code_diagnostics`
- `code_rename_preview` (preview only; application remains an explicit mutation through safe patch/edit tooling)

## Non-goals

- custom semantic/vector search;
- custom AST parser/indexer;
- silently installing language servers;
- allowing arbitrary project-supplied LSP commands;
- LSP-originated edits bypassing workspace mutation policy;
- unbounded background indexing;
- claiming universal language support;
- auto-applying code actions in v1.

## Current repository target

The repo itself contains TypeScript/Vue and Rust, so those ecosystems are the first required proving grounds. Exact server binaries/configuration must be re-verified at implementation time rather than frozen here.

Likely existing ecosystem choices include:

- Rust: `rust-analyzer`;
- TypeScript/Vue: a reviewed TypeScript/Vue language-server setup compatible with the repository's current Nuxt/Vue toolchain.

Do not add a new dependency if an already-installed, reviewed language server can be used through operator-configured safe PATH/toolchain paths.

## Architecture

Create a narrow **LSP session manager** outside MCP transport concerns.

Responsibilities:

- detect/configure approved language servers;
- start each server with direct argv, bounded environment, contained cwd, and explicit process lifetime;
- perform JSON-RPC framing;
- initialize with the verified workspace root;
- maintain a bounded number of sessions;
- synchronize documents safely;
- normalize server responses into stable internal types;
- enforce timeout/result limits;
- restart/evict unhealthy sessions;
- redact raw server errors from public responses;
- expose only capability-supported operations.

LSP servers are executable project tooling and must be treated as untrusted subprocesses. They receive no broader filesystem/network/credential access than needed.

## Workspace/root semantics

The relay can serve multiple sibling repositories under the user's home execution root. Therefore LSP state must be keyed by canonical repository/workspace identity, not by one global relay directory.

Requirements:

- discover the relevant project root from tool `cwd`/path using safe read-only markers and Git/workspace identity;
- prevent one repository's LSP process/state from being reused against an unrelated repository;
- cap simultaneous project/language sessions;
- cleanly shut down idle sessions;
- never traverse protected credential paths while discovering roots.

## Proposed tool contracts

### `code_symbols`

Inputs:

```text
cwd?
path?
query?
max_results?
continuation?
```

Supports document symbols when `path` is supplied and workspace-symbol search when `query` is supplied, only when the selected server advertises the capability.

### `code_definition`

Inputs:

```text
path
line
column
cwd?
```

Return one or more contained locations with bounded preview metadata. Reject locations outside permitted roots rather than reading them automatically.

### `code_references`

Inputs:

```text
path
line
column
cwd?
include_declaration?
max_results?
continuation?
```

### `code_implementations`

Same location model as references, capability-gated.

### `code_hover`

Return bounded plain/markdown type/docs text. Sanitize/limit server-provided markdown and never render executable HTML.

### `code_diagnostics`

Inputs:

```text
cwd?
path?
severity?
max_results?
continuation?
```

Return normalized:

- path;
- range;
- severity;
- diagnostic code when stable;
- source;
- bounded message.

Diagnostics should be cheap enough to use in an edit → diagnose → edit loop, but the tool must not pretend diagnostics are complete until the LSP reports a stable result/version.

### `code_rename_preview`

Inputs:

```text
path
line
column
new_name
cwd?
```

Returns a bounded normalized workspace edit **without applying it**. The agent can then use `apply_patch` / safe file operations after normal policy/approval.

## Document synchronization

Prefer LSP's expected incremental/full document synchronization based on server capability. Do not continuously mirror the entire repository into memory.

After native file mutations, Plan 039E hooks may notify active LSP sessions. Until then, the LSP adapter must be able to re-read/version the target document before semantic queries so stale results are bounded and detectable.

## Phases

### PHASE-01 — LSP process/security contract

- [ ] Audit current repo toolchains and installed language servers.
- [ ] Define approved server configuration contract.
- [ ] Define sandbox/filesystem/network/environment policy.
- [ ] Define per-project session identity, limits, idle shutdown, crash handling.
- [ ] Define public-safe error model and telemetry fields.

### PHASE-02 — protocol/session manager

- [ ] Implement JSON-RPC transport and initialize/shutdown lifecycle.
- [ ] Support server capability negotiation.
- [ ] Bound messages and timeouts.
- [ ] Prevent session cross-talk between repositories.
- [ ] Add deterministic fake/minimal LSP fixture process if needed for protocol acceptance; do not create a conventional unit-test suite.

### PHASE-03 — Rust proof

- [ ] Configure reviewed `rust-analyzer` invocation.
- [ ] Prove symbols, definition, references, hover, diagnostics on a bounded fixture/current source.
- [ ] Verify no protected-path/network escape.

### PHASE-04 — TypeScript/Vue proof

- [ ] Integrate the reviewed language-server setup matching current Nuxt/Vue tooling.
- [ ] Prove `.ts` and `.vue` navigation/diagnostics representative of the real app.
- [ ] Avoid conflicting duplicate TypeScript/Vue servers.

### PHASE-05 — MCP tool surface

- [ ] Expose `code_symbols`, `code_definition`, `code_references`, `code_implementations`, `code_hover`, `code_diagnostics`.
- [ ] Add accurate schemas/annotations and server-side caps.
- [ ] Add continuation support shared with Plan 039H.
- [ ] Return capability-not-supported distinctly from server failure.

### PHASE-06 — rename preview

- [ ] Normalize LSP WorkspaceEdit into a safe preview model.
- [ ] Reject edits outside the verified workspace/protected policy.
- [ ] Never auto-apply edits from LSP.
- [ ] Prove preview can be translated to Plan-039B patch semantics without loss.

### PHASE-07 — post-edit diagnostic integration contract

- [ ] Define how Plan 039E hooks notify/refresh active LSP sessions after `file_edit`, `file_write`, and `apply_patch`.
- [ ] Prove diagnostics reflect a fresh edit without restarting the whole relay.

## Security cases

Explicitly test:

- malicious/oversized LSP responses;
- server crash/hang;
- server attempting to emit file locations outside execution root;
- symlinked project content;
- sibling-project session confusion;
- protected credential paths;
- project config that attempts to replace the approved server executable;
- output containing raw absolute host paths or secrets;
- stale document versions;
- unsupported server capabilities;
- invalid UTF-8 / malformed JSON-RPC.

## Acceptance criteria

- [ ] Semantic navigation works for Rust and the current TS/Vue stack.
- [ ] No custom parser/index/vector service was introduced.
- [ ] LSP subprocesses are bounded and scoped per verified project.
- [ ] Diagnostics are useful immediately after edits and are version/staleness aware.
- [ ] Rename remains preview-only until normal mutation policy applies it.
- [ ] All MCP output is bounded and public-safe.
- [ ] `pnpm verify:commit`, tool build, deterministic LSP acceptance, and live MCP smoke checks pass.
