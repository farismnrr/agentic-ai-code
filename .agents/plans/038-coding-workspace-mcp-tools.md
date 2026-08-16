# Plan 038 — Coding Workspace MCP Tools

**Status:** IN PROGRESS — PLAN-07 VERIFIED
**Created:** 2026-08-16
**Predecessor context:** Plan 037 — Long-Running MCP Execution, Streaming, and Task Lifecycle
**Goal:** Add a small, secure, high-value set of native workspace tools to the Masih Awam MCP relay so coding assistants can inspect, search, and edit repositories without routing routine filesystem work through `terminal_exec`.
**Success Criteria:** The relay exposes bounded native workspace tools for directory inspection, file discovery, text search, reading, editing, and writing; every operation respects the existing execution-root boundary; mutation tools prevent path/symlink escape; outputs are context-bounded; MCP schemas/annotations are accurate; and existing terminal/web tools continue working unchanged.

## Scope

### In scope

Workspace v1:

```text
Workspace
├── directory_list
├── file_search
├── text_search
├── file_read
├── file_edit
└── file_write

Execution — existing
├── terminal_exec
├── terminal_job_start
├── terminal_job_get
└── terminal_job_cancel

Web — existing
├── http_fetch
└── web_search
```

Follow-up capabilities after Workspace v1:

```text
Git
├── git_status
└── git_diff

Code Intelligence — future
├── code_symbols
├── code_definition
├── code_references
└── semantic_search
```

### Out of scope for Workspace v1

- `delete_file`, `move_file`, `copy_file`, `mkdir`, and `chmod` native tools.
- Native Git mutation such as commit, push, reset, rebase, or checkout.
- LSP server integration.
- Embeddings, vector databases, semantic indexing daemons, or custom RAG infrastructure.
- Replacing `terminal_exec`.
- Changing the current Docker trust model.
- Filesystem access outside the configured execution root.

## Current State

- MCP tool definitions live in `packages/rust-tools/interfaces/src/mcp.rs`.
- Execution dispatch lives in `packages/rust-tools/application/src/execution.rs`.
- Tool arguments are validated against JSON Schema before dispatch.
- MCP annotations already support `readOnlyHint`, `destructiveHint`, `idempotentHint`, and `openWorldHint`.
- Current coding tools require the `relay.coding` OAuth scope.
- `ServerConfig::resolved_execution_root()` establishes the relay filesystem boundary.
- `relay_core::terminal_policy::resolve_contained_cwd()` already provides canonicalized containment for terminal working directories.
- The relay supports a broad single-owner execution root such as `/home/<user>`, allowing calls to target sibling repositories through `cwd`.
- Existing black-box coverage exercises MCP discovery, argument validation, dispatch, authentication, and security behavior.

## Constraints & Decisions

### Reuse one filesystem security model

Do not implement separate path-validation logic in every tool. Generalize the existing execution-root containment rules into reusable workspace path primitives first.

Conceptually:

```text
execution_root
      │
      ▼
resolve cwd
      │
      ▼
resolve requested path
      │
      ▼
canonical/parent validation
      │
      ▼
execution_root containment
      │
      ▼
operation
```

Existing paths and new write targets require different resolution because a new file cannot itself be canonicalized before creation.

Path resolution is a validation-time foundation, not a claim of race-free mutation. A returned `PathBuf` can become stale if filesystem state changes after validation. PLAN-06 and PLAN-07 must therefore pair these resolvers with operation-time revalidation/no-follow behavior and atomic mutation semantics, and their acceptance must prove that an external target cannot be touched through a race or symlink swap.

### Native filesystem operations first

Implement these using Rust filesystem APIs rather than shell commands:

- `directory_list`
- `file_read`
- `file_edit`
- `file_write`

This keeps path guarantees, error handling, output bounds, and atomic mutation behavior under relay control.

### Search strategy

- Prefer native deterministic traversal for `file_search`.
- Allow `text_search` to use the already-reviewed `rg` executable through direct argv semantics if that remains the simplest robust implementation.
- Never interpolate user search input into a shell string.

### `cwd` semantics

All workspace tools that operate on paths should accept optional `cwd`, resolved beneath the configured execution root. The selected `cwd` is not required to be a Git repository root.

### Bounded results everywhere

Server-enforced hard limits must exist for file bytes/lines, directory entries/depth, file-search results, text-search matches/previews, write payloads, edit payloads, and total result size. Caller-provided limits may lower these bounds but never raise the operator maximum.

### Structured results where useful

Prefer structured metadata when it materially helps clients, while retaining compact text content where compatibility requires it. Useful metadata includes path, line range, total line count, result count, and explicit truncation state.

## Phase Overview

| Plan | Goal | Depends On | Status | Exit Criteria |
|---|---|---|---|---|
| PLAN-01 | Shared workspace/path safety foundation | none | Complete | Read/write paths have one validation-time containment contract; mutation phases add operation-time atomic/no-follow safety |
| PLAN-02 | `directory_list` | PLAN-01 | Complete | Bounded directory inspection works through MCP |
| PLAN-03 | `file_search` | PLAN-01 | Complete | Bounded deterministic file discovery works through MCP |
| PLAN-04 | `text_search` | PLAN-01 | Complete | Bounded literal/regex source search works through MCP |
| PLAN-05 | `file_read` | PLAN-01 | Complete | Complete/ranged text reading works through MCP |
| PLAN-06 | `file_edit` | PLAN-01, PLAN-05 | Complete | Guarded exact edits are atomic and contained |
| PLAN-07 | `file_write` | PLAN-01 | Complete | Create/replace operations are explicit, atomic, and contained |
| PLAN-08 | MCP integration and security hardening | PLAN-02..07 | Planned | Full v1 surface passes black-box and regression coverage |
| PLAN-09 | Documentation and agent guidance | PLAN-08 | Planned | Docs match the actual schemas and tool-selection behavior |
| PLAN-10 | Read-only Git tools | PLAN-08 | Future | Structured `git_status` and `git_diff` are available |
| PLAN-11 | Code intelligence | PLAN-08 | Future | Evidence-driven symbol/navigation support exists |
| PLAN-12 | Semantic search | PLAN-08 | Deferred | Added only if lexical search proves insufficient |

# PLAN-01: Workspace Path & Safety Foundation

**Goal:** Build one reusable filesystem security layer before exposing native workspace tools.
**Dependencies:** none

## TASK-001: Define the shared path model

**Outcome:** One documented path contract shared by all workspace tools.

**Files:**
- Modify: `packages/rust-tools/core/src/lib.rs`
- Create or modify: an appropriately named workspace-path module under `packages/rust-tools/core/src/`
- Verify: deterministic core acceptance example/script (repository policy intentionally has no unit-test suite)

**Steps:**
- [x] Define optional `cwd` resolution beneath `execution_root`.
- [x] Define relative `path` semantics against resolved `cwd`.
- [x] Decide and document absolute-path behavior: absolute paths are accepted only when their canonical target/write parent remains beneath `execution_root`.
- [x] Define handling of `.` and `..`: they are allowed only when canonical resolution remains contained.
- [x] Define symlink behavior: read-style existing paths may follow contained symlinks; external targets are rejected; existing final write symlinks are rejected.
- [x] Define nonexistent-target behavior separately for reads and writes.
- [x] Reject directory/file type mismatches consistently.

**Validation:**
- `./scripts/verify-workspace-path-security.sh` → deterministic positive/negative resolver acceptance under the repository's no-unit-test policy.
- `cargo fmt --all -- --check`
- `RUSTFLAGS='-D warnings' cargo check -p relay-core --all-targets --all-features --locked`
- `cargo clippy -p relay-core --all-targets --all-features --locked -- -D warnings`

**Commit boundary:** `feat(relay): add shared workspace path resolution`

## TASK-002: Separate existing-path and write-target resolution

**Outcome:** Reusable APIs for canonical existing paths and potentially nonexistent write targets.

**Steps:**
- [x] Add a contained `cwd` resolver reused by terminal policy and available to workspace tools.
- [x] Add existing-path resolution that canonicalizes the target and verifies containment.
- [x] Add write-target resolution that canonicalizes the required existing parent before appending a missing final component.
- [x] Ensure an external symlinked parent cannot produce a contained-looking resolved write target.

**Validation:**
- Deterministic acceptance covers existing files/directories, missing files/cwd/parents, contained and external absolute paths, contained/external `..` behavior, contained/external symlinks, symlink loops, type mismatches, and write-parent resolution.

## TASK-003: Harden symlink escape behavior

**Outcome:** Path resolution rejects observed-state symlink/root escapes. Recursive traversal behavior is proven by PLAN-02/03, while operation-time edit/write race safety is proven by PLAN-06/07.

**Steps:**
- [x] Verify a contained symlink to an external directory is rejected by path resolution.
- [x] Verify a contained symlink to an external file is rejected by path resolution.
- [x] Verify write-target resolution rejects an external symlinked parent and canonicalizes a contained symlinked parent.
- [x] Verify path resolution rejects a recursive symlink loop.
- [x] PLAN-02/PLAN-03 traversal acceptance proves listing/search may report symlink entries without following them recursively; no traversal exists in PLAN-01.

**Validation:**
- Resolver escape attempts fail before filesystem access through the escaped target; the acceptance fixture verifies its external sentinel remains unchanged. Operation-time mutation race safety is intentionally deferred to PLAN-06/07 where mutations actually exist.

## TASK-004: Normalize path errors

**Outcome:** Stable safe errors without unnecessary host-path leakage.

**Steps:**
- [x] Standardize resolver errors for missing/inaccessible paths, wrong entry type, root escape, existing final symlink/type mismatch, and invalid write parent; create/overwrite conflicts remain mutation-phase errors.
- [x] Keep caller-controlled values and sensitive host details out of path diagnostics; deterministic acceptance includes a path canary check.

**Phase exit criteria:**
- [x] Shared resolution APIs exist and are covered by the deterministic positive/negative acceptance script allowed by current repository policy.
- [x] Validation-time symlink escapes are blocked for existing paths and write-target resolution.
- [x] New write targets can be validated without requiring the final file to exist; this does not replace PLAN-06/07 operation-time race/atomicity protections.

# PLAN-02: `directory_list`

**Goal:** Give coding assistants a cheap bounded view of repository structure.
**Dependencies:** PLAN-01

## TASK-201: Define MCP schema

**Outcome:** Stable v1 contract.

Proposed input:

```text
directory_list(
  path?,
  cwd?,
  depth?,
  max_entries?
)
```

**Steps:**
- [x] Default `path` to `.`.
- [x] Default `depth` to `2`, with server hard maximum `4`.
- [x] Define server hard maxima: `100` returned entries, `4096` scanned entries per directory, and `256 KiB` serialized result size.
- [x] Add correct read-only MCP annotations and `relay.coding` security scheme.

## TASK-202: Implement bounded native traversal

**Outcome:** Deterministic directory listing without spawning `find` or `tree`.

**Steps:**
- [x] Resolve the directory using PLAN-01 primitives, including native-path re-resolution for filesystem-discovered entries.
- [x] Use deterministic lexical ordering.
- [x] Return entry type and relative path.
- [x] Bound recursion depth, scanned entries, returned entries, and serialized result bytes.
- [x] Do not recursively follow symlink directories.
- [x] Mark caller/result-limit truncation explicitly; reject pathological directory scans at the hard scan cap.

**Validation:**
- `./scripts/verify-directory-list.sh` covers empty/normal directories, nested depth, deterministic ordering, caller/default result truncation, a `4097`-entry pathological scan-cap rejection, symlink entries/loops/external targets, traversal/absolute escapes, nonexistent paths, file-as-directory, schema rejection, and safe error output through real MCP `tools/list` + `tools/call`.

**Commit boundary:** `feat(relay): add directory_list workspace tool`

**Phase exit criteria:**
- [x] Tool appears in `tools/list`.
- [x] Bounded listing works through `tools/call`.
- [x] Root/symlink containment tests pass.

# PLAN-03: `file_search`

**Goal:** Provide fast bounded filename/path discovery without requiring shell commands.
**Dependencies:** PLAN-01

## TASK-301: Define glob semantics

**Outcome:** One unambiguous search syntax.

Proposed input:

```text
file_search(
  pattern,
  cwd?,
  max_results?
)
```

**Steps:**
- [x] Use documented glob semantics rather than mixing regex/fuzzy/glob behavior in one field.
- [x] Support patterns such as `**/*.rs`, `**/*auth*`, and exact filenames.
- [x] Define hidden/ignored file behavior explicitly.
- [x] Set a hard server result maximum.

## TASK-302: Implement contained traversal

**Outcome:** Deterministic file discovery with no symlink recursion.

**Steps:**
- [x] Resolve `cwd` through PLAN-01.
- [x] Traverse natively unless implementation evidence strongly favors an already-approved binary.
- [x] Return paths relative to selected `cwd` where practical.
- [x] Sort results deterministically.
- [x] Stop at configured result limits and mark truncation; selection follows deterministic byte-sorted depth-first traversal, while returned matches are lexically sorted.
- [x] Prevent symlink loops and root escape.

**Validation:**
- Exact match, nested glob, no match, max-results truncation, hidden/generated directory behavior, and symlink loops.

**Commit boundary:** `feat(relay): add file_search workspace tool`

**Phase exit criteria:**
- [x] File discovery is bounded, deterministic, and contained.
- [x] Ignore behavior is documented and tested.

# PLAN-04: `text_search`

**Goal:** Expose ripgrep-quality source search through a bounded stable MCP contract.
**Dependencies:** PLAN-01

## TASK-401: Define search contract

**Outcome:** Stable literal/regex interface.

Proposed input:

```text
text_search(
  query,
  cwd?,
  glob?,
  regex?,
  case_sensitive?,
  max_results?
)
```

**Steps:**
- [x] Choose documented defaults for literal/regex and case sensitivity.
- [x] Define hard result and preview-byte caps.
- [x] Add correct read-only/closed-world MCP annotations.

## TASK-402: Implement direct-argv search execution

**Outcome:** Search uses mature matching semantics without shell interpolation.

**Steps:**
- [x] Resolve `cwd` through PLAN-01.
- [x] If using `rg`, resolve it through the existing approved executable policy.
- [x] Pass all user values as direct argv arguments.
- [x] Prefer machine-readable ripgrep output where practical.
- [x] Parse path, line, optional column, and bounded preview.
- [x] Cap pathological long matching lines.
- [x] Return explicit truncation state.

**Validation:**
- Literal search, regex search, invalid regex, case sensitivity, glob restriction, Unicode, binary files, no results, maximum-result truncation, huge matching line, and symlink behavior.

**Commit boundary:** `feat(relay): add bounded text_search tool`

**Phase exit criteria:**
- [x] Literal and regex searches work through MCP.
- [x] No user search value passes through a shell string.
- [x] Results are bounded and deterministic enough for agent consumption.

# PLAN-05: `file_read`

**Goal:** Provide line-oriented bounded text-file reading.
**Dependencies:** PLAN-01

## TASK-501: Define read schema

**Outcome:** Stable ranged-read contract.

Proposed input:

```text
file_read(
  path,
  cwd?,
  offset_line?,
  limit_lines?
)
```

**Steps:**
- [x] Use 1-based line numbers in model-facing metadata/output.
- [x] Define hard byte and line caps.
- [x] Define behavior for requests beyond EOF.
- [x] Reject directories.
- [x] Prefer explicit non-text/invalid-UTF-8 failure over silently corrupting source text.

## TASK-502: Implement streaming/bounded read

**Outcome:** Reading a range does not require emitting or retaining an unbounded file payload.

**Steps:**
- [x] Resolve target through PLAN-01.
- [x] Verify it is a regular file.
- [x] Read only enough data to satisfy the requested bounded range and metadata strategy.
- [x] Return range metadata and explicit truncation.
- [x] Handle empty files and long single lines safely.

**Validation:**
- Empty/small files, beginning/middle ranges, beyond EOF, huge files, huge lines, Unicode, invalid UTF-8/binary files, and symlink escape attempts.

**Commit boundary:** `feat(relay): add bounded file_read tool`

**Phase exit criteria:**
- [x] Complete and ranged reads work through MCP.
- [x] Hard server bounds are enforced.
- [x] Binary/non-text behavior is explicit.

# PLAN-06: `file_edit`

**Goal:** Perform guarded surgical source edits without shell text manipulation.
**Dependencies:** PLAN-01, PLAN-05

## TASK-601: Define exact replacement contract

**Outcome:** Ambiguous edits fail instead of guessing.

Proposed input:

```text
file_edit(
  path,
  old_text,
  new_text,
  replace_all?
)
```

Default: `replace_all=false`.

Required behavior when `replace_all=false`:

```text
0 matches -> fail
1 match   -> edit
>1 match  -> fail
```

When `replace_all=true`, replace all matches and report the replacement count.

## TASK-602: Implement atomic replacement

**Outcome:** Successful edits replace the target atomically where supported, and failed edits leave the original valid.

**Steps:**
- [x] Resolve and validate target through PLAN-01.
- [x] Enforce an edit-size hard limit.
- [x] Read target as valid text.
- [x] Count matches before writing.
- [x] Build replacement content.
- [x] Write a temporary file in the same directory.
- [x] Preserve relevant file permissions.
- [x] Flush/write according to repository reliability conventions.
- [x] Rename atomically over the original where supported.
- [x] Clean up temporary artifacts on error.

## TASK-603: Evaluate stale-edit protection without blocking v1

**Outcome:** Decide whether an optional content hash is worthwhile now or should be deferred.

**Steps:**
- [x] Evaluate returning a content hash from `file_read` and accepting `expected_hash` in `file_edit`.
- [x] Add it only if it stays simple and materially improves concurrent-change safety.
- [x] Otherwise record it as a follow-up, not a v1 requirement.

**Content-hash decision:** deferred from Workspace v1. Exact-match guards plus operation-time entry identity checks cover the current local edit contract without adding another client coordination field; a content hash remains a follow-up if real concurrent-edit usage shows a gap.

**Validation:**
- One match, zero matches, multiple matches, replace-all, same old/new text, empty replacement, Unicode, oversize target, permissions preservation, symlink escape, and failure-before-rename integrity.

**Commit boundary:** `feat(relay): add guarded file_edit tool`

**Phase exit criteria:**
- [x] Ambiguous edits cannot mutate files.
- [x] Successful edits are atomic under the selected implementation contract.
- [x] Failed writes preserve the original file.

# PLAN-07: `file_write`

**Goal:** Safely create or intentionally replace text files.
**Dependencies:** PLAN-01

## TASK-701: Define explicit creation/overwrite semantics

**Outcome:** Existing files are never silently replaced by default.

Proposed input:

```text
file_write(
  path,
  content,
  cwd?,
  create_parents?,
  overwrite?
)
```

Preferred defaults:

```text
create_parents = false
overwrite = false
```

**Steps:**
- [x] Require `overwrite=true` to replace an existing regular file.
- [x] Reject directories and unsupported target types.
- [x] Enforce payload hard limits.
- [x] Define permission behavior for create versus replacement.

## TASK-702: Implement contained parent creation

**Outcome:** Optional parent creation cannot cross the execution root.

**Steps:**
- [x] Resolve the nearest existing ancestor through PLAN-01.
- [x] Reject symlinked/external parent escapes.
- [x] Create missing parent segments only when `create_parents=true`.
- [x] Keep all created directories beneath the execution root.

## TASK-703: Share atomic writer with `file_edit`

**Outcome:** `file_write` and `file_edit` use one reviewed mutation primitive rather than duplicated write logic.

**Steps:**
- [x] Extract/reuse the same temp-file and rename mechanism used by `file_edit`.
- [x] Ensure a failed replacement leaves the old file intact.
- [x] Make `idempotentHint` a deliberate decision based on final overwrite semantics rather than assuming it is safe.

**Validation:**
- Create new file, overwrite existing file, default overwrite rejection, parent creation disabled/enabled, symlinked parent escape, oversize payload, Unicode, permissions, and failed replacement integrity.

**PLAN-07 implementation note:** Linux mutations walk parent directories from a stable execution-root directory descriptor using no-follow opens. New files use mode `0644`; overwrite preserves the existing regular file mode. Default creation commits with `renameat2(RENAME_NOREPLACE)` so a concurrent creator cannot be silently clobbered. Oversized MCP request bodies may be rejected by the transport with HTTP 413 before schema/dispatch, which is accepted as fail-closed pre-dispatch rejection.

**Commit boundary:** `feat(relay): add atomic file_write tool`

**Phase exit criteria:**
- [x] Creation and replacement semantics are explicit.
- [x] All writes remain contained.
- [x] Atomic mutation code is shared with `file_edit`.

# PLAN-08: MCP Integration & Security Hardening

**Goal:** Verify the new tools through the real MCP boundary and protect existing relay behavior.
**Dependencies:** PLAN-02 through PLAN-07

## TASK-801: Complete tool catalog integration

**Files:**
- Modify: `packages/rust-tools/interfaces/src/mcp.rs`

**Steps:**
- [ ] Add all six tool definitions.
- [ ] Use JSON Schema 2020-12 where current catalog conventions do so.
- [ ] Set `additionalProperties: false`.
- [ ] Bound argument lengths/counts in schemas where appropriate.
- [ ] Set accurate MCP annotations.
- [ ] Keep the `relay.coding` OAuth scheme.

## TASK-802: Refactor dispatch for native operations

**Files:**
- Modify: `packages/rust-tools/application/src/execution.rs`
- Modify/create supporting application modules only when cohesion improves.

**Outcome:** Native filesystem operations do not have to masquerade as terminal jobs.

**Steps:**
- [ ] Separate native workspace dispatch from subprocess-backed operations.
- [ ] Keep `terminal_exec`, terminal jobs, `http_fetch`, and `web_search` behavior intact.
- [ ] Reuse the existing `ToolCallResult` model unless a concrete result-shape gap requires a minimal extension.

## TASK-803: Add negative schema tests

**Steps:**
- [ ] Missing required fields.
- [ ] Unknown fields.
- [ ] Wrong types.
- [ ] Oversized values.
- [ ] Invalid enums/ranges.
- [ ] Verify malformed input is rejected before filesystem/search execution.

## TASK-804: Extend authentication coverage

**Steps:**
- [ ] Verify tools cannot execute without required authentication in remote mode.
- [ ] Verify missing `relay.coding` scope never reaches dispatch.
- [ ] Do not add additional OAuth scopes unless implementation uncovers a concrete policy need.

## TASK-805: Extend MCP black-box tests

**Files:**
- Modify: `scripts/phase4-black-box.sh` or a more appropriate existing black-box test location if discovered during execution.

**Steps:**
- [ ] Verify all six tools appear in `tools/list`.
- [ ] Invoke each tool through `tools/call`.
- [ ] Verify result bounds/truncation.
- [ ] Verify schema rejection happens before dispatch.
- [ ] Verify root and symlink escape failures through actual MCP requests.

## TASK-806: Add filesystem security regression matrix

Test at minimum:

```text
../escape
../../etc/passwd
absolute external path
contained symlink -> external file
contained symlink -> external directory
write beneath external symlinked parent
recursive symlink loop
new path beneath external symlinked ancestor
```

## TASK-807: Verify existing tool regressions

**Steps:**
- [ ] `terminal_exec` still works with direct argv semantics.
- [ ] Terminal job start/get/cancel still work.
- [ ] `http_fetch` still works.
- [ ] `web_search` still works.
- [ ] Docker remains opt-in and unchanged.

**Validation:**
- `cargo fmt --check` → clean.
- `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- `cargo test --workspace` → all tests pass.
- Relevant black-box relay verification → all MCP/security cases pass.
- Run `pnpm lint` / `pnpm typecheck` if implementation touches application/TypeScript surfaces that require them.

**Commit boundary:** `test(relay): harden workspace MCP tool integration`

**Phase exit criteria:**
- [ ] New tools work through real MCP calls.
- [ ] Invalid inputs are rejected pre-dispatch.
- [ ] Authentication/security boundaries remain intact.
- [ ] Existing tools have no regression.

# PLAN-09: Documentation & Agent Guidance

**Goal:** Document the native workspace surface and when an agent should prefer it over the terminal.
**Dependencies:** PLAN-08

## TASK-901: Update tool documentation

**Files to review:**
- `README.md`
- `docs/getting-started.md`
- `docs/architecture.md`
- `docs/external-mcp.md`
- `packages/rust-tools/README.md`

Only modify documents where the information naturally belongs.

**Steps:**
- [ ] Document all six workspace tools and actual input semantics.
- [ ] Document execution-root, `cwd`, and path containment behavior.
- [ ] Update relay/server descriptions that currently mention only terminal, HTTP, and web search.
- [ ] Keep examples synchronized with the real MCP schemas.

## TASK-902: Document agent tool selection

Recommended guidance:

```text
Need file contents?        -> file_read
Need filename discovery?   -> file_search
Need text/code occurrence? -> text_search
Need repository structure? -> directory_list
Need surgical replacement? -> file_edit
Need create/full replace?  -> file_write
Need build/test/git/script? -> terminal_exec
```

Clarify that terminal remains the correct escape hatch for builds, tests, package managers, Git mutations, interpreters, project scripts, and unsupported operations.

**Validation:**
- Documentation options match tool schemas.
- README/tool inventory matches `tools/list`.

**Commit boundary:** `docs(relay): document native coding workspace tools`

**Phase exit criteria:**
- [ ] Current tool behavior is accurately documented.
- [ ] Agent guidance prefers structured native operations without discouraging legitimate terminal usage.

# PLAN-10: Read-only Git Tools

**Status:** Future follow-up after Workspace v1 stabilizes.
**Dependencies:** PLAN-08

Candidate tools:

```text
git_status
git_diff
```

## Proposed `git_status` result

- branch
- upstream
- ahead/behind
- staged paths
- unstaged paths
- untracked paths
- conflicts

## Proposed `git_diff` input

```text
git_diff(
  cwd?,
  path?,
  staged?,
  context_lines?
)
```

Requirements:

- [ ] Read-only annotations.
- [ ] Hard output bounds.
- [ ] Repository containment.
- [ ] No native `git_commit`, `git_push`, `git_reset`, `git_rebase`, or similar mutation in this plan.

# PLAN-11: Code Intelligence

**Status:** Future / evidence-driven.
**Dependencies:** PLAN-08

Candidate tools:

```text
code_symbols
code_definition
code_references
```

Preferred direction:

```text
workspace tool
     ↓
language-server adapter
     ↓
existing project LSP
```

Requirements before implementation:

- [ ] Demonstrate that `file_search` + `text_search` + `file_read` leaves a meaningful navigation gap.
- [ ] Evaluate reuse of existing project language servers.
- [ ] Avoid building a custom parser ecosystem merely to expose symbol tool names.
- [ ] Add language support incrementally with explicit capability reporting.

# PLAN-12: Semantic Search

**Status:** Deferred.
**Dependencies:** PLAN-08

Do not implement until concrete usage demonstrates that lexical discovery is repeatedly insufficient.

Evidence should include one or more of:

- repeated failures to locate conceptually related code with lexical tools;
- repositories large enough that lexical navigation becomes materially inefficient;
- measurable reduction in tool calls/context usage;
- a simple implementation path that does not require disproportionate persistent infrastructure.

Prefer a lightweight/stateless approach before adding vector databases, persistent embedding daemons, custom RAG services, or agent frameworks.

# Cross-Cutting Requirements

## Result limits

Define operator-controlled hard maxima for categories such as:

```text
MAX_FILE_READ_BYTES
MAX_FILE_READ_LINES
MAX_DIRECTORY_ENTRIES
MAX_DIRECTORY_DEPTH
MAX_FILE_SEARCH_RESULTS
MAX_TEXT_SEARCH_RESULTS
MAX_MATCH_PREVIEW_BYTES
MAX_FILE_WRITE_BYTES
MAX_FILE_EDIT_BYTES
```

Exact numeric values should be selected from implementation/testing evidence rather than guessed in this plan.

## Ignore behavior

Choose and document consistent behavior for `.gitignore`, `.git/info/exclude`, hidden files, `.git`, `node_modules`, `target`, `.nuxt`, and `.output`.

Preferred direction:

- `text_search` may leverage ripgrep's normal ignore behavior.
- `file_search` should implement only the ignore behavior that remains simple and predictable.
- Avoid making the two tools expose radically different repository visibility without explicit documentation.

## Telemetry

Useful metadata may include:

- tool name;
- duration;
- success/failure;
- result count;
- truncated yes/no.

Do not log:

- file contents;
- write/edit content;
- search query contents unless the existing observability policy explicitly permits and sanitizes them;
- unnecessarily identifying host filesystem details;
- secret-bearing values.

Review existing telemetry sanitization before adding attributes.

## Error consistency

Prefer safe actionable errors such as:

```text
requested path escapes configured execution root
```

Avoid exposing full host paths or attacker-controlled diagnostic strings when they provide no recovery value.

## No shell interpolation

Native workspace tools must never convert user data into shell command strings. If `text_search` uses ripgrep, pass user values as direct argv entries.

# Risks & Rollback

- **Path-containment regression** → implement PLAN-01 first, test traversal/symlink cases aggressively, and do not expose mutation tools until the shared layer passes.
- **Large-result/context exhaustion** → server-side hard caps and explicit truncation on every read/search/list operation.
- **Atomic-write edge cases** → use one shared writer, temporary files in the target directory, and tests proving failure preserves the original.
- **Search behavior mismatch** → document ignore/glob semantics and cover them with black-box tests.
- **Dispatch complexity** → split native operations from job/process operations instead of forcing every tool through `ToolInvocation`.
- **Existing relay regressions** → full workspace tests plus existing MCP black-box coverage before considering Workspace v1 complete.
- **Overbuilding** → keep Git mutation, LSP, and semantic/vector infrastructure out of Workspace v1.

Rollback should be commit/phase-based. Each tool should be independently reviewable so a problematic tool can be reverted without removing the shared safety foundation or unrelated tools.

# Master Todo

- [x] PLAN-01: Shared workspace path and containment layer
- [x] PLAN-02: Add `directory_list`
- [x] PLAN-03: Add `file_search`
- [x] PLAN-04: Add `text_search`
- [x] PLAN-05: Add `file_read`
- [x] PLAN-06: Add guarded `file_edit`
- [x] PLAN-07: Add atomic `file_write`
- [ ] PLAN-08: Complete MCP/security/black-box integration
- [ ] PLAN-09: Update documentation and agent tool-selection guidance
- [ ] PLAN-10: Evaluate/add `git_status` and `git_diff`
- [ ] PLAN-11: Evaluate LSP-backed code intelligence
- [ ] PLAN-12: Re-evaluate semantic search only from demonstrated need

# Final Acceptance Criteria

Workspace v1 is complete when:

- [ ] all six workspace tools appear in `tools/list`;
- [ ] all schemas reject malformed input before execution;
- [ ] every path operation stays beneath `execution_root`;
- [ ] traversal and symlink escapes have regression tests;
- [ ] read/search/list outputs are bounded;
- [ ] mutation payloads are bounded;
- [ ] `file_edit` rejects ambiguous matches;
- [ ] `file_write` has explicit overwrite semantics;
- [ ] mutation is atomic where intended;
- [ ] no native workspace operation executes user input through a shell;
- [ ] MCP annotations accurately describe effects;
- [ ] OAuth requirements remain intact;
- [ ] existing terminal/job/http/search behavior has no regression;
- [ ] Cargo format, Clippy, and tests pass;
- [ ] MCP black-box tests pass;
- [ ] documentation matches actual schemas;
- [ ] no semantic-index/vector infrastructure has been added prematurely.

# Execution Handoff

Execute in dependency order:

```text
PLAN-01 Path/security foundation
       │
       ├─────────────┬────────────┬────────────┐
       ▼             ▼            ▼            ▼
PLAN-02          PLAN-03       PLAN-04      PLAN-05
 directory       file          text         file
 list            search        search       read
                                              │
                                              ▼
                                           PLAN-06
                                           file_edit

PLAN-01 ───────────────────────────────────► PLAN-07
                                            file_write

PLAN-02..07
       │
       ▼
PLAN-08 MCP integration/security
       │
       ▼
PLAN-09 docs
       │
       ├──► PLAN-10 Git read-only
       ├──► PLAN-11 Code intelligence
       └──► PLAN-12 Semantic search (only if justified)
```

For this repository, prefer sequential execution and validation of Workspace v1 even where phases are technically parallelizable:

1. PLAN-01 foundation and security tests.
2. PLAN-02 `directory_list`, expose through MCP, black-box verify.
3. PLAN-03 `file_search`, expose through MCP, black-box verify.
4. PLAN-04 `text_search`, expose through MCP, black-box verify.
5. PLAN-05 `file_read`, expose through MCP, black-box verify.
6. PLAN-06 `file_edit`, validate mutation safety and atomicity.
7. PLAN-07 `file_write`, validate explicit overwrite and atomicity.
8. PLAN-08 full regression/security validation.
9. PLAN-09 documentation.
10. Only after Workspace v1 is stable, evaluate PLAN-10 through PLAN-12.

Before each execution phase, re-check Git/worktree state and any assumptions that may have changed. Validate each phase before starting a dependent phase. Use the repository's existing Git delivery policy for task-owned commits/pushes once implementation work begins.
