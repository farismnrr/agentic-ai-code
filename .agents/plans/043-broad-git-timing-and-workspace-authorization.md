---
goal: Broad Git and Worktree Support, Per-Tool Execution Timing, and Explicit Workspace Allowlisting for MCP Coding Server
version: '1.0'
date_created: '2026-08-19'
last_updated: '2026-08-19'
owner: relay-core
status: Completed
tags:
  - feature
  - architecture
  - security
  - git
  - worktrees
  - mcp
  - timing
  - workspace-allowlist
---

# Introduction

![Status: Completed](https://img.shields.io/badge/status-Completed-brightgreen)

This implementation plan addresses three essential capability gaps in the MCP coding server (`ai-tools` Rust workspace):

1. **Broad Git Command Support & Full Git Worktree Support**: Provide first-class structured MCP tools for standard Git workflows (worktrees, stashes, tags, remotes, restore, reset, cherry-pick, revert, branch rename, clean) so autonomous coding agents do not need to drop down to unstructured `terminal_exec git ...`.
2. **Per-Tool Execution Timing**: Inject monotonic execution duration and server-total latency into response `_meta.timing` across all tool calls, allowing transparent latency diagnosis without polluting text output.
3. **Explicit Workspace/Directory Allowlisting**: Implement explicit, auditable workspace authorization (`workspace_add`, `workspace_list`, `workspace_get`, `workspace_remove`) to enable safe multi-project and multi-worktree development without granting unrestricted filesystem access.

In addition, this plan investigates and resolves current runtime symptoms (`terminal_exec` failures, `terminal_job_start` exceptions, toolchain resolution, TypeScript `code_diagnostics` unsupported status, and `text_search` execution failures).

---

## 1. Requirements & Constraints

- **REQ-001**: Expose structured Git worktree tools (`git_worktree_list`, `git_worktree_add`, `git_worktree_remove`, `git_worktree_prune`, `git_worktree_get`) validating destination paths, branches/refs, parent directories, and workspace containment.
- **REQ-002**: Expose safe structured tools for stashes (`git_stash_list`, `git_stash_push`, `git_stash_pop`, `git_stash_apply`, `git_stash_drop`), tags (`git_tag_list`, `git_tag_create`, `git_tag_delete`), remotes (`git_remote_add`, `git_remote_remove`, `git_remote_set_url`), branch lifecycle (`git_branch_rename`), commit amendment, working tree recovery (`git_restore`, `git_clean`), and history management (`git_cherry_pick`, `git_revert`, `git_reset`).
- **REQ-003**: Inject structured timing metadata `_meta.timing: { dispatch_ms: u64, server_total_ms: u64 }` into every tool-call result using monotonic clocks (`Instant`), covering success, errors, synchronous execution, and background task creation.
- **REQ-004**: Implement in-memory explicit workspace allowlisting (`workspace_add`, `workspace_list`, `workspace_get`, `workspace_remove`) with bounded capacity and strict path canonicalization.
- **REQ-005**: Update containment resolution across file tools, search tools, git tools, terminal subprocesses, and LSP sessions to recognize all currently authorized workspace roots.
- **REQ-006**: Update Bubblewrap sandbox profiles to mount all authorized workspace roots (or the specific active target workspace) to allow sandboxed subprocesses to run in linked worktrees and authorized external projects.
- **SEC-001**: Enforce strict path validation on workspace addition: reject `/`, `/etc`, `/tmp`, `/dev`, `/proc`, `/sys`, `/var`, `/usr`, `/root`, depth < 3 paths, credential stores (`.ssh`, `.gnupg`), and symlink escapes.
- **SEC-002**: Prevent destructive or history-rewriting Git operations from bypassing capability policy or approval gates; keep user hooks, global Git configs, and interactive prompts disabled (`GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, `core.hooksPath=/dev/null`).
- **SEC-003**: Maintain strict open-world annotations on network-facing Git operations (`git_push`, `git_fetch`, `git_remote_*`) and ensure credentials remain isolated from ordinary execution.
- **SEC-004**: Worktree destinations outside currently authorized workspace roots must be rejected; filesystem authority is not expanded merely because Git recognizes a path.
- **CON-001**: Preserve existing Layered Architecture across `relay_core`, `relay_application`, `relay_infrastructure`, `relay_interfaces`, and `cli`.
- **CON-002**: All maintained Rust source files must strictly obey the maintainability budget (<500 physical lines per file, <15 files per folder) and pass `pnpm verify:commit`.
- **CON-003**: Do not introduce external database dependencies, background daemons, or unreviewed crates.
- **GUD-001**: Use direct `Command` argv construction with zero shell interpolation for all native Git execution.
- **GUD-002**: Parse machine-readable porcelain formats (`--porcelain=v1/v2`, `-z`, `--null`) with hard byte and entry caps.
- **PAT-001**: Follow the established `RepoContext` and `ToolCallResult` pattern with signed continuation pagination for large outputs.

---

## 2. Implementation Steps

### Implementation Phase 1: Workspace Allowlisting & Multi-Root Containment Foundation

- GOAL-001: Implement explicit workspace authorization registry and integrate multi-root containment across core path resolution and sandbox profiles.

| Task | Description | Completed | Date |
|------|-------------|-----------|------|
| TASK-001 | Create `WorkspaceAllowlist` in `relay_core::workspace_path` managing primary root and dynamic authorized paths with bounded capacity (max 32), canonicalization, and system directory validation. | Yes | 2026-08-19 |
| TASK-002 | Update `resolve_contained_cwd`, `resolve_existing_path`, `resolve_write_target`, and `SecureDirectory` in `relay_application::workspace` to validate containment against any authorized workspace root. | Yes | 2026-08-19 |
| TASK-003 | Update `spawn_with_profile` in `relay_application::execution::sandbox` to bind-mount all authorized workspace directories into the Bubblewrap sandbox namespace. | Yes | 2026-08-19 |
| TASK-004 | Add `workspace_add`, `workspace_list`, `workspace_get`, and `workspace_remove` tools to `relay_interfaces::mcp::catalog` and dispatch handlers in `relay_application::workspace::dispatch`. | Yes | 2026-08-19 |
| TASK-005 | Wire `WorkspaceAllowlist` through `ServerConfig`/relay state, with `--execution-root` as the hard ceiling and `--dir` as the primary authorized workspace; verify deny → add → allow → remove → deny lifecycle. | Yes | 2026-08-19 |

### Implementation Phase 2: Per-Tool Monotonic Execution Timing

- GOAL-002: Implement accurate monotonic dispatch and server-total execution timing across all MCP tool responses.

| Task | Description | Completed | Date |
|------|-------------|-----------|------|
| TASK-006 | Define `ToolTimingMeta` struct `{ dispatch_ms: u64, server_total_ms: u64 }` in `relay_interfaces::mcp` with clean serialization into `_meta.timing`. | Yes | 2026-08-19 |
| TASK-007 | Instrument `handle_tools_call` in `relay_infrastructure::transport::tools` to measure request arrival start time, dispatch start time, elapsed dispatch duration, and total response construction time. | Yes | 2026-08-19 |
| TASK-008 | Ensure timing metadata is attached to successful tool results, error results, hook approvals, task creation responses, and synchronous dispatch fallbacks. | Yes | 2026-08-19 |
| TASK-009 | Clarify background job timing: distinguish instantaneous job creation dispatch latency from completed process execution runtime in job JSON representation. | Yes | 2026-08-19 |
| TASK-010 | Add deterministic and black-box MCP acceptance verifying timing presence, non-negative values, `server_total_ms >= dispatch_ms`, fast-read timing, and metadata non-pollution. | Yes | 2026-08-19 |

### Implementation Phase 3: Broad Structured Git & Worktree Execution Layer

- GOAL-003: Implement centralized Git worktree and extended Git operations with strict validation, scrubbing, and structured output.

| Task | Description | Completed | Date |
|------|-------------|-----------|------|
| TASK-011 | Create `relay_application::git::worktree` implementing `git_worktree_list`, `git_worktree_add`, `git_worktree_remove`, `git_worktree_prune`, and `git_worktree_get` with porcelain parsing and strict destination workspace authorization. | Yes | 2026-08-19 |
| TASK-012 | Create `relay_application::git::stash` implementing `git_stash_list`, `git_stash_push`, `git_stash_pop`, `git_stash_apply`, and `git_stash_drop` with bounded message inputs and index validation. | Yes | 2026-08-19 |
| TASK-013 | Create `relay_application::git::tag` implementing `git_tag_list`, `git_tag_create`, and `git_tag_delete` with ref/name safety validation. | Yes | 2026-08-19 |
| TASK-014 | Implement `git_branch_rename`, `git_restore`, `git_clean`, `git_commit_amend`, `git_cherry_pick`, `git_revert`, and `git_reset` across the existing Git mutation/advanced layers with protected-path and history-rewrite safety bounds. | Yes | 2026-08-19 |
| TASK-015 | Implement `git_remote_add`, `git_remote_remove`, and `git_remote_set_url` through the structured Git layer, allowing only credential-free HTTPS/SSH remote URLs and preserving existing isolated fetch/push transport. | Yes | 2026-08-19 |
| TASK-016 | Register all new Git and worktree tools in `relay_interfaces::mcp::catalog` with accurate annotations (read-only, destructive, idempotent, open-world), security schemes, and input schemas. | Yes | 2026-08-19 |
| TASK-017 | Update `resolve_repo` and `resolve_git_workspace` in `relay_application::git::context` to discover and validate Git repositories across any authorized workspace root. | Yes | 2026-08-19 |

### Implementation Phase 4: Root Cause Remediation & Diagnostic Hardening

- GOAL-004: Diagnose and remediate observed runtime issues with `terminal_exec`, `terminal_job_start`, toolchain PATH resolution, TypeScript diagnostics, and search tools.

| Task | Description | Completed | Date |
|------|-------------|-----------|------|
| TASK-018 | Audit `terminal_exec` error mapping: return actionable bounded policy/argument failures (for example safe-PATH or workspace denial) while keeping internal/provider/process diagnostics redacted behind the generic internal tool error boundary. | Yes | 2026-08-19 |
| TASK-019 | Verify/harden terminal job start/get/cancel lifecycle, queued/running completion behavior, and bounded serialization; black-box candidate acceptance must complete without the previously observed client `ExceptionGroup` failure. | Yes | 2026-08-19 |
| TASK-020 | Verify toolchain discovery (`RELAY_TOOLCHAIN_PATH`) for Node.js/Corepack: `node`, `npm`, and pinned `pnpm` execute inside Bubblewrap; explicitly report that Vitest is not installed in this repository rather than fabricating acceptance. | Yes | 2026-08-19 |
| TASK-021 | Document TypeScript LSP server registration (`--lsp-server typescript=typescript-language-server,vue=vue-language-server`) and verify `code_diagnostics` operates cleanly when configured. | Yes | 2026-08-19 |
| TASK-022 | Harden `text_search` execution path: ensure ripgrep handles empty results and multi-workspace paths without throwing internal execution errors. | Yes | 2026-08-19 |

### Implementation Phase 5: Verification, Acceptance & Closeout

- GOAL-005: Execute comprehensive test suites, deterministic acceptance verification, and closeout review.

| Task | Description | Completed | Date |
|------|-------------|-----------|------|
| TASK-023 | Build deterministic Plan 043 acceptances: the base lifecycle suite plus a separate security/history suite covering negative workspace/worktree boundaries, protected paths, amend/history operations, and remote URL rejection. | Yes | 2026-08-19 |
| TASK-024 | Run the repository's existing Rust test targets (`cargo test --workspace`) plus deterministic acceptance examples; do not add a new tracked unit-test suite because repository policy forbids one. | Yes | 2026-08-19 |
| TASK-025 | Run repository verification gate (`pnpm verify:commit`) ensuring architecture, maintainability line budgets, agent doc integrity, lint, and type checks all pass. | Yes | 2026-08-19 |
| TASK-026 | Perform black-box MCP smoke tests against a temporary candidate relay: 77-tool discovery, real response timing, workspace add/remove revocation, terminal-job execution duration, and candidate-sandbox `pnpm verify:commit`. Active operator relay restart/rediscovery remains a separate deployment step. | Yes | 2026-08-19 |
| TASK-027 | Synchronize `.agents/memories/README.md`, update documentation, and perform standard closeout review. | Yes | 2026-08-19 |

### Closure evidence (2026-08-19)

- Current v8 `tools/list` contract: **77 tools**, preserving all 50 v7 tools and adding 27 Plan 043 tools; `git_push` and `git_commit_amend` are both present exactly once. Snapshot hash: `588154b6009b02863471c4c6ef043fb0a8e34831fef130da0fdf10aebb1934b7`.
- `cargo test --workspace`: PASS. `plan043_acceptance`: PASS. `plan043_security_acceptance`: PASS. `typescript_lsp_acceptance`: `TYPESCRIPT_LSP_ACCEPTANCE_PASS`.
- Black-box candidate MCP: `file_read` returned `_meta.timing { dispatch_ms: 0, server_total_ms: 10 }`; secondary workspace was denied before `workspace_add`, allowed after add, and denied again after `workspace_remove`; terminal job completed with `executionDurationMs: 74` in the sampled run.
- Candidate sandbox executed `node v24.15.0`, `npm 11.16.0`, Corepack/pnpm, and finally `pnpm verify:commit` end-to-end with Cargo + FNM toolchain mounts; the repository gate finished `OK`.
- TypeScript source support is verified when the language server is explicitly configured. The currently deployed operator relay still requires a separately approved rebuild/restart and MCP action rediscovery before these source changes are considered live; no production/service restart is part of this implementation closure.

---

## 3. Alternatives

- **ALT-001**: *Raw Shell Passthrough for Git (`terminal_exec git ...` only)*: Rejected because raw shell commands bypass capability analysis, approval policies, credential isolation, and structured result contracts, making autonomous agent workflows fragile and risky.
- **ALT-002**: *Persistent Arbitrary Filesystem Access*: Rejected because allowing unbounded filesystem mutation voids the Bubblewrap sandbox and execution-root security model. In-memory, explicitly authorized workspace roots provide exact containment.
- **ALT-003**: *Header-Only Timing Metadata*: Rejected because MCP clients consume JSON-RPC tool result bodies directly over streamable HTTP POST where custom response headers may be stripped by client proxies or SDKs.

---

## 4. Dependencies

- **DEP-001**: Bubblewrap (`bwrap`) on Linux host for sandboxed subprocess execution.
- **DEP-002**: Git CLI (`git` >= 2.30) available in system safe PATH.
- **DEP-003**: Ripgrep (`rg`) for text search.
- **DEP-004**: Node.js & TypeScript toolchains for LSP diagnostics.

---

## 5. Files

- **FILE-001**: `packages/rust-tools/core/src/workspace_path.rs` — multi-root workspace allowlist and path containment logic.
- **FILE-002**: `packages/rust-tools/core/src/config.rs` & `cli.rs` — server configuration extensions for initial workspace allowlists.
- **FILE-003**: `packages/rust-tools/interfaces/src/mcp.rs` — timing metadata structures and `_meta.timing` serialization.
- **FILE-004**: `packages/rust-tools/interfaces/src/mcp/catalog.rs` — tool declarations for worktrees, stashes, tags, remotes, advanced Git, and workspaces.
- **FILE-005**: `packages/rust-tools/application/src/execution/sandbox.rs` — multi-workspace Bubblewrap mount handling.
- **FILE-006**: `packages/rust-tools/application/src/workspace/dispatch.rs` & `secure.rs` — workspace tool handlers and multi-root directory operations.
- **FILE-007**: `packages/rust-tools/application/src/git/worktree.rs` — new module for structured Git worktree operations.
- **FILE-008**: `packages/rust-tools/application/src/git/stash.rs` — new module for structured Git stash operations.
- **FILE-009**: `packages/rust-tools/application/src/git/tag.rs` — new module for structured Git tag operations.
- **FILE-010**: `packages/rust-tools/application/src/git/advanced.rs` plus `git/mutation.rs` — restore/reset/revert/cherry-pick/clean/branch rename and bounded commit amendment.
- **FILE-011**: `packages/rust-tools/application/src/git/context.rs` & `process.rs` — multi-workspace Git repository discovery.
- **FILE-012**: `packages/rust-tools/infrastructure/src/transport/tools.rs` — timing measurement, error classification, and tool dispatch.
- **FILE-013**: `packages/rust-tools/application/examples/plan043_acceptance.rs` and `plan043_security_acceptance.rs` — positive lifecycle plus negative/security/history acceptance.
- **FILE-014**: `packages/rust-tools/application/src/execution/toolchain.rs` — reviewed Node/Rust toolchain-root recognition for safe shim/symlink execution.
- **FILE-015**: `docs/getting-started.md` and `docs/architecture.md` — workspace-boundary and TypeScript/Vue LSP registration guidance.

---

## 6. Testing

- **TEST-001**: `test_workspace_allowlist_security` — verify add/list/remove, rejection of system roots (`/`, `/tmp`, `/etc`), rejection of symlink escapes, and boundary isolation.
- **TEST-002**: `test_timing_metadata_presence` — verify `_meta.timing` contains valid `dispatch_ms` and `server_total_ms` on success and error responses.
- **TEST-003**: `test_git_worktree_lifecycle` — verify worktree list, add in authorized workspace, operate inside worktree, remove, and prune.
- **TEST-004**: `test_git_stash_and_restore_operations` — verify stash push, list, pop, apply, drop, and file restore.
- **TEST-005**: `test_git_tag_and_remote_operations` — verify tag creation, listing, deletion, and safe remote URL management.
- **TEST-006**: `test_git_history_operations` — verify cherry-pick, revert, branch rename, soft reset, and safe clean.
- **TEST-007**: `test_toolchain_and_lsp_diagnostics` — verify TypeScript LSP diagnostics and safe PATH executable resolution.
- **TEST-008**: `test_plan043_acceptance_suite` — comprehensive end-to-end acceptance example exercising all Plan 043 capabilities.

---

## 7. Risks & Assumptions

- **RISK-001**: Multiple workspace mounts in Bubblewrap could encounter command-line length limits if hundreds of directories were added. *Mitigation:* Bound allowlist capacity to 32 entries.
- **RISK-002**: Destructive Git commands (e.g. `git_clean`, `git_reset`) could cause data loss if misused by agents. *Mitigation:* Mark tools with `destructive_hint: true`, require explicit arguments, support dry-run where applicable, and deny uncontained paths.
- **ASSUMPTION-001**: The runtime environment is Linux with Bubblewrap installed and non-root execution permissions.
- **ASSUMPTION-002**: TypeScript language server (`typescript-language-server`) is available in configured safe PATH when TypeScript code intelligence is desired.

---

## 8. Related Specifications / Further Reading

- [Plan 039B — Git Read and Patch Ergonomics](file:///home/farismnrr/Projects/MasihAwam/ai-code/.agents/plans/039b-git-read-and-patch-ergonomics.md)
- [Plan 040 — Reliable Git + GitHub Delivery Workflow Roadmap](file:///home/farismnrr/Projects/MasihAwam/ai-code/.agents/plans/040-git-github-delivery-roadmap.md)
- [Canonical Memory](file:///home/farismnrr/Projects/MasihAwam/ai-code/.agents/memories/README.md)
- [Model Context Protocol Specification (2026-07-28)](https://modelcontextprotocol.io/)
