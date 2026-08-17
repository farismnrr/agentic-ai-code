# Plan 039C — LSP Code Intelligence and Diagnostics

**Status:** IN PROGRESS — PHASE-01 / PHASE-02 IMPLEMENTED, VERIFIED, AND DELIVERED
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

- [x] Audit current repo toolchains and installed language servers.
- [x] Define approved server configuration contract.
- [x] Define sandbox/filesystem/network/environment policy.
- [x] Define per-project session identity, limits, idle shutdown, crash handling.
- [x] Define public-safe error model, bounded private stderr retention, and low-cardinality telemetry fields.

### PHASE-02 — protocol/session manager

- [x] Implement JSON-RPC transport and initialize/shutdown lifecycle.
- [x] Support server capability negotiation.
- [x] Bound messages and timeouts.
- [x] Prevent session cross-talk between repositories.
- [x] Add deterministic fake/minimal LSP fixture process for protocol/security acceptance; no conventional unit-test suite added.

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
## PHASE-01 / PHASE-02 verified foundation (2026-08-17)

The foundation is implemented without exposing any public `code_*` MCP tools and without integrating a real language server yet. Current machine audit found `rust-analyzer`, `typescript-language-server`, `vue-language-server`, and `tsserver` already installed; nothing was installed by this plan boundary.

Security/process contract:

- operator approval is explicit through repeated `--lsp-server language=executable` / `RELAY_LSP_SERVER` entries; values contain no shell command or repository-controlled argv; executable resolution uses the existing relay safe PATH and exact executable basename;
- LSP subprocesses reuse the authoritative Bubblewrap builder but run under a stricter profile: only the verified Git workspace is mounted read-only, toolchain directories are separate read-only mounts, networking is unshared, Docker/Tailscale sockets are never exposed, common credential stores/token files are hidden, HOME is temporary, and the environment is cleared before setting the bounded runtime variables;
- workspace identity is the canonical contained Git top-level reached through the existing hardened Git process path. Session keys are `(canonical workspace root, language)`, so sibling repositories never share one process/session merely because they live under one execution root;
- hard bounds are: 16 configured server mappings, 4 projects, 8 language-server sessions, 64 pending requests per session, 15s startup, 10s request deadline, 3s shutdown deadline, 8KiB framing headers, 4MiB JSON-RPC messages, 64KiB retained stderr, 16 synchronized documents/session, 1MiB/document, 10-minute idle eviction, and at most 2 restart/crash attempts per workspace/language in a 60-second window;
- public-facing `LspError` values are static normalized categories/messages and never contain raw server stderr or host paths. Raw stderr is retained only as a bounded private process tail and is not emitted through public errors. Any later telemetry may record only bounded low-cardinality fields such as event/outcome, language, error kind, duration, and counts; it must not record stderr, source content, credentials, or absolute workspace paths.

Protocol/session implementation lives under `packages/rust-tools/application/src/lsp*` and remains outside MCP transport routing. It implements strict `Content-Length` framing, duplicate/malformed header rejection, UTF-8/JSON and JSON-RPC 2.0 envelope validation, bounded exact reads, initialize/initialized, shutdown/exit, monotonic request IDs, pending-request correlation, server-notification tolerance, server-request non-confusion, capability capture, timeout/crash wakeup, timer-owned idle eviction, restart limits, concurrent same-key session serialization, and minimum bounded document open/change synchronization.

Deterministic acceptance is `bash scripts/verify-lsp-foundation.sh` using `packages/rust-tools/application/examples/lsp_foundation_acceptance.rs`. It proves normal initialize/request/shutdown and capability capture plus malformed framing, malformed JSON, oversized responses, hanging/crashed servers, invalid response IDs/envelopes, concurrent same-key session reuse, sibling-workspace process isolation, repository-local executable replacement resistance, credential-path masking, network isolation, bounded environment, document refresh/versioning, bounded stderr, and public-safe errors.

PHASE-03 through PHASE-07 remain unstarted. This plan is **not closed** at this boundary.

Independent review/remediation evidence for this boundary:

- initial read-only review of `65c791397f5e83d30f64c3300f43363d460c0dd5..62a4d4f20b1f49f6faeecb01e2b048cde83d52f7` identified two material lifecycle gaps: timed-out requests did not invalidate/terminate their server session, and protocol-faulted sessions could remain alive until all retained handles disappeared;
- remediation makes timeout/protocol faults fail the session closed, terminate and reap the process group immediately, and makes manager replacement/idle cleanup defensively reap faulted sessions;
- the remediation also reuses the existing execution process-group kill primitive and adds deterministic assertion coverage for invalid JSON-RPC envelopes and post-error fault state;
- because this remediation materially changes lifecycle behavior, the final committed state requires a fresh independent read-only review before push.
