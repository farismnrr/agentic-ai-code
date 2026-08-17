# Plan 039C — LSP Code Intelligence and Diagnostics

**Status:** IN PROGRESS — PHASE-01/PHASE-02 CLOSED/VERIFIED; PHASE-03 through PHASE-07 IMPLEMENTED, FINAL PLAN VERIFICATION PENDING
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

### PHASE-03 — Rust proof — IMPLEMENTED (FINAL PLAN VERIFICATION PENDING; deferred by design to a later full-plan review)

- [x] Configure reviewed `rust-analyzer` invocation.
- [x] Prove symbols, definition, references, hover, diagnostics on a bounded fixture/current source.
- [x] Verify no protected-path/network escape.

Deterministic acceptance: `cargo run -p relay-application --example rust_lsp_acceptance` (see `packages/rust-tools/application/src/lsp/rust.rs`, refactored during PHASE-04 into a thin wrapper over the new shared `lsp/semantic.rs` layer with no behavioral change; the wrapper's public API and the acceptance fixture are unchanged).

### PHASE-04 — TypeScript/Vue proof — IMPLEMENTED

- [x] Integrate the reviewed language-server setup matching current Nuxt/Vue tooling: `typescript-language-server@5.3.0` for `.ts`/`.tsx`/`.js` and `@vue/language-server@3.3.8` (Volar) for `.vue`, both already installed under the fnm-managed Node toolchain (no install performed by this plan boundary).
- [x] Prove `.ts` navigation/diagnostics representative of the real app: symbols, definition, references, hover, diagnostics all proven with real `typescript-language-server` semantics against a deterministic fixture.
- [x] Avoid conflicting duplicate TypeScript/Vue servers: one `typescript` session and one `vue` session, keyed exactly like Rust by `(canonical workspace root, language)` in the existing `LspSessionManager`; no second daemon.
- [x] `.vue` proof, scoped to what the installed server build actually supports: real capability negotiation is proven (`renameProvider`/`definitionProvider`/`hoverProvider`/`documentSymbolProvider` all true), and a real `.vue` document query is proven to fail *safely and boundedly* (`language_server_timeout` or `language_server_crashed`, session faulted closed, no hang, no false-empty success) rather than claiming semantic navigation this server cannot actually perform standalone. Root cause (read directly from `@vue/language-server@3.3.8`'s `lib/server.js`): this build only implements hybrid mode — every document-level request forwards a `_vue:<Command>` request to a companion `typescript-language-server` process running the `@vue/typescript-plugin` tsserver plugin. That plugin package is not part of this machine's reviewed toolchain (`typescript-language-server`, `@vue/language-server`, `typescript` are installed; `@vue/typescript-plugin` is not). Installing it was out of scope for this run (no silent tooling installs per the plan's non-goals) — flagged for the final verifier / a follow-up plan boundary to explicitly review and approve.

Concrete defect found and fixed while integrating TypeScript (in the *generic* substrate, not a rust-analyzer-only code path — PHASE-03 never hit it because rust-analyzer happens to report the structured `textDocumentSync` object form): `ServerCapabilities::from_initialize` treated the LSP spec's numeric `textDocumentSync` form (`typescript-language-server` reports `"textDocumentSync": 2`) as `openClose = false` unconditionally, silently starving every TypeScript/Vue query of `didOpen`. Also added the `textDocument.publishDiagnostics` client capability declaration, without which `typescript-language-server` never pushes diagnostics. Both fixes are covered by the deterministic acceptance below and by the unchanged PHASE-01/02/03 acceptance (`scripts/verify-lsp-foundation.sh`, `rust_lsp_acceptance`), which still pass unmodified.

Deterministic acceptance: `bash scripts/verify-lsp-typescript.sh` (`packages/rust-tools/application/examples/typescript_lsp_acceptance.rs`).

### PHASE-05 — MCP tool surface — IMPLEMENTED

- [x] Expose `code_symbols`, `code_definition`, `code_references`, `code_implementations`, `code_hover`, `code_diagnostics` (plus `code_rename_preview`, see PHASE-06) in `packages/rust-tools/interfaces/src/mcp/catalog.rs`, dispatched from a new `relay_application::code` module reusing the existing `tools/call` pipeline, schema validation, and `coding_security_scheme()` — no new admission/security layer.
- [x] Accurate schemas/annotations: all seven tools are `read_only_hint: true`, `destructive_hint: false`; `code_rename_preview` additionally documents preview-only, non-applying semantics in its description.
- [x] Continuation: a bounded offset-based `max_results`/`continuation` token on `code_symbols`/`code_references`/`code_implementations`/`code_diagnostics`. Plan 039H (task/output management) does not exist yet in this repository, so no incompatible pagination scheme was invented — this is a minimal, self-contained mechanism a future 039H continuation framework can subsume.
- [x] Capability-not-supported is returned distinctly from server failure via the existing `LspError` category system (`unsupported_lsp_capability` vs. `language_server_crashed`/`language_server_timeout`/etc.), never collapsed into one generic error.

Deterministic acceptance: `bash scripts/verify-code-mcp.sh` (`packages/rust-tools/application/examples/code_mcp_acceptance.rs`) — proves the full dispatch path end-to-end against a real `rust-analyzer` session: catalog presence/annotations, definition/references/hover/diagnostics content, pagination, capability-gated implementations, invalid-request rejection, and workspace-relative (never absolute host) paths in every response.

### PHASE-06 — rename preview — IMPLEMENTED

- [x] Normalize LSP WorkspaceEdit into a safe preview model: both `changes` and `documentChanges` (text-edit only) forms, in `packages/rust-tools/application/src/lsp/rename.rs` (split out of the shared `lsp/semantic.rs` layer purely to stay under the maintainability line-budget).
- [x] Reject edits outside the verified workspace/protected policy: every target path is resolved and containment/protected-path-checked through the same `resolve_existing_path`/`reject_protected_target` pipeline `code_definition`/`code_references` already use.
- [x] Never auto-apply edits from LSP: `code_rename_preview` only normalizes and returns; `applied: false` is always present in the response; proven never to mutate disk across five scenarios (valid multi-file, valid single-file, unsupported resource operation, overlapping edits, outside-root).
- [x] Unsupported resource operations (`documentChanges` entries with a `kind`, i.e. file create/rename/delete) fail the whole preview closed rather than silently dropping them. Overlapping edits within one file are rejected as ambiguous rather than guessed at.
- [x] Proved preview can be translated to Plan-039B patch semantics without loss: a preview edit is mechanically translated into a unified-diff string and applied through the existing `apply_patch` tool (dry-run, then commit), producing byte-for-byte the same renamed content — while the preview call itself remains non-mutating.

Deterministic acceptance: `bash scripts/verify-rename-preview.sh` (`packages/rust-tools/application/examples/rename_preview_acceptance.rs`, using a small scripted fake LSP server to make adversarial `WorkspaceEdit` shapes deterministic — a real language server's exact adversarial output is not practical to force reliably).

### PHASE-07 — post-edit diagnostic integration contract — IMPLEMENTED

- [x] The narrow Plan 039C integration contract (not the future Plan 039E hooks framework) was already implemented as a property of the existing document-sync substrate: every semantic query (`RustLanguageServer`/`TypeScriptLanguageServer` -> `semantic::sync` -> `LspSession::sync_document`) re-reads the current contained document on disk, compares its content hash, bumps the internal document version, and sends the correct `didOpen`/`didChange` per the negotiated `textDocumentSync` kind before issuing the request — so a native mutation through `file_edit`/`file_write`/`apply_patch` is observed by the very next `code_*` query on the same session, with no restart.
- [x] Proved diagnostics reflect a fresh edit without restarting the whole relay/server process.

Deterministic acceptance: `bash scripts/verify-post-edit-diagnostics.sh` (`packages/rust-tools/application/examples/post_edit_diagnostics_acceptance.rs`) — creates a contained rust-analyzer fixture, queries `code_diagnostics` on the active session, mutates through the native `file_write` path, queries `code_diagnostics` again through the *same* cached session (`Arc::ptr_eq` proves no restart), and observes the new diagnostic; bounded polling never silently returns the stale pre-edit result as current.

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

- [x] Semantic navigation works for Rust and TypeScript. `.vue` is capability-negotiated and fails safely/boundedly rather than claiming navigation the installed server build cannot standalone-perform (see PHASE-04; needs a follow-up decision on installing `@vue/typescript-plugin`).
- [x] No custom parser/index/vector service was introduced.
- [x] LSP subprocesses are bounded and scoped per verified project (unchanged PHASE-01/02 substrate; TypeScript/Vue reuse the same `LspSessionManager`).
- [x] Diagnostics are useful immediately after edits (PHASE-07) and expose a `version` field when the server reports one.
- [x] Rename remains preview-only; `code_rename_preview` never applies edits (PHASE-06).
- [x] All MCP output is bounded and public-safe (workspace-relative paths, bounded text/result counts, classified `LspError` categories, no raw stderr/host paths).
- [x] Tool build, deterministic LSP acceptance (`verify-lsp-foundation.sh`, `verify-lsp-typescript.sh`, `verify-code-mcp.sh`, `verify-rename-preview.sh`, `verify-post-edit-diagnostics.sh`, `rust_lsp_acceptance`) and `pnpm verify:commit` all pass at this boundary. Live deployed MCP smoke checks were intentionally not run (no relay restart/deploy performed this run) — deferred to final independent verification.

## PHASE-04 through PHASE-07 implementation boundary (2026-08-17)

Implemented sequentially on top of the PHASE-01/02 verified foundation and the PHASE-03 (unverified-but-working) Rust proof, without redoing PHASE-01/02/03 except for the one concrete generic-substrate defect fixed during PHASE-04 (documented above under PHASE-04). No public behavior bypasses the existing workspace mutation policy: `code_rename_preview` is strictly read-only/preview, and no `code_*` tool can write to disk.

Commits: `feat(lsp): add TypeScript/Vue language-server proof (Plan 039C PHASE-04)`, `feat(mcp): expose public code intelligence tools (Plan 039C PHASE-05/06)`, `test(lsp): add rename-preview adversarial and patch-interop acceptance (Plan 039C PHASE-06)`, `test(lsp): add post-edit diagnostic freshness acceptance (Plan 039C PHASE-07)`.

Known open item for the final independent verifier: whether to install/approve `@vue/typescript-plugin` (enabling full `.vue` TS-backed semantics via the hybrid-mode bridge) is an explicit follow-up decision, not made silently in this run.

PHASE-03 through PHASE-07 are **implemented but not yet independently, exhaustively verified** against the plan's full security/architecture matrix — that review is a separate later boundary. This plan is **not closed**.
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

The acceptance also covers a deterministic notification broken-pipe mode: a server closes its stdin after initialization, the next notification write returns the stable `Crashed` error, the session becomes faulted, the child is terminated/reaped through the shared lifecycle path, and manager lookup replaces rather than reuses the unhealthy session before final shutdown leaves no active sessions.

PHASE-03 through PHASE-07 remain unstarted. This plan is **not closed** at this boundary.

Independent review/remediation evidence for this boundary:

- initial read-only review of `65c791397f5e83d30f64c3300f43363d460c0dd5..62a4d4f20b1f49f6faeecb01e2b048cde83d52f7` identified two material lifecycle gaps: timed-out requests did not invalidate/terminate their server session, and protocol-faulted sessions could remain alive until all retained handles disappeared;
- remediation makes timeout/protocol faults fail the session closed, terminate and reap the process group immediately, and makes manager replacement/idle cleanup defensively reap faulted sessions;
- the remediation also reuses the existing execution process-group kill primitive and adds deterministic assertion coverage for invalid JSON-RPC envelopes and post-error fault state;
- notification writes now fault and terminate unhealthy sessions through the same reviewed lifecycle machinery as request/protocol faults; the original stable `LspError` is returned and unhealthy sessions are not reused;
- final remediation commit `a4d2be44afbb1f4279dffe3deab81b5d322f08e2` passed the complete PHASE-01/02 validation matrix and received a fresh independent read-only review of `65c791397f5e83d30f64c3300f43363d460c0dd5..a4d2be44afbb1f4279dffe3deab81b5d322f08e2` with `VERDICT: NO MATERIAL FINDINGS`.
