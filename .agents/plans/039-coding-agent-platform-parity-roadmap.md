# Plan 039 — Coding Agent Platform Parity Roadmap

**Status:** IN PROGRESS — 039A–039B CLOSED / VERIFIED; 039C–039J planned and unstarted
**Created:** 2026-08-16  
**Predecessor:** Plan 038 — Coding Workspace MCP Tools (CLOSED / VERIFIED)  
**Plan family:** 039A through 039J  

## Goal

Evolve Masih Awam from a secure MCP coding-tool relay into a complete, vendor-neutral coding-agent platform with modern industry-standard coding-agent ergonomics while preserving the repository's existing security, architecture, verification, and self-improvement boundaries.

The target is **capability parity, not product cloning**. Reuse industry standards and existing repository primitives instead of reproducing vendor-specific configuration formats or building a new agent framework from scratch.

## Success criteria

Plan 039 is complete only when the platform has all of the following working together:

1. a verified maintainability/refactor foundation enforcing DRY, pragmatic SOLID, Layered Architecture, YAGNI, KISS, folder cohesion, file-size discipline, and mandatory documentation/agent-guide synchronization;
2. bounded native Git/change intelligence and safe multi-hunk patch ergonomics;
3. LSP-backed code navigation and diagnostics without a custom parser/indexer;
4. a unified, enforceable capability policy with protected paths, input-aware approvals, and explicit network/exec risk boundaries;
5. deterministic lifecycle hooks that cannot widen hard security policy;
6. isolated subagents with scoped prompts, tools, permissions, budgets, and reusable agent profiles;
7. safe background-agent/worktree isolation for genuinely independent work, with bounded concurrency;
8. structured task/progress state, context-budget visibility, and continuation/pagination for large tool outputs;
9. standards-based extension interoperability around Agent Skills, MCP, LSP, resources, and external MCP servers without a proprietary marketplace;
10. a coherent first-party UX, telemetry/audit trail, documentation, and end-to-end verification proving the entire agent loop.

## Current verified baseline

As of 2026-08-16 on branch `dev`:

- Plan 038 is closed and verified.
- The Rust relay exposes 12 MCP tools:
  - `terminal_exec`
  - `terminal_job_start`
  - `terminal_job_get`
  - `terminal_job_cancel`
  - `http_fetch`
  - `web_search`
  - `directory_list`
  - `file_search`
  - `text_search`
  - `file_read`
  - `file_edit`
  - `file_write`
- Workspace native operations share contained path resolution under `execution_root`.
- Terminal execution uses Bubblewrap and already masks common credential stores such as `.ssh`, `.aws`, `.config/gcloud`, `.docker`, `.kube`, `.npmrc`, `.netrc`, `.pypirc`, and Cargo credential files.
- Native workspace tools currently enforce execution-root containment but do not yet share the terminal's credential-store masking policy.
- The first-party Nuxt application already has AI SDK tool approval integration and conversation-scoped remembered `always` / `never` decisions.
- Existing MCP approval decisions are tool-level rather than a general argument-aware deny/ask/allow rule system.
- The repository already has Agent Skills-compatible reusable skills under `ai-self/` and `.agents/skills/`.
- The repository already has context compaction and persisted context summaries, but not a first-class task ledger/context inspector comparable to modern coding-agent UIs.
- The relay currently exposes MCP tools but does not expose MCP resources/resource templates.
- There is no native LSP bridge, Git read-tool family, hook runtime, subagent runtime, custom agent-profile runtime, or background worktree-agent orchestration.
- The repository intentionally has no GitHub Actions CI workflow and no conventional unit-test suite; deterministic contract/security scripts and the mandatory local commit gate remain authoritative.
- `.agents/` is deliberately vendor-neutral. Do not introduce client-specific configuration directories or formats as the canonical project contract.

## Industry baseline reviewed for this roadmap

Implementation must re-check current protocol/tooling standards and relevant first-party documentation at execution time because coding-agent platforms evolve quickly. The roadmap adopts recurring industry patterns without making vendor-specific guidance canonical:

- fine-grained deny / ask / allow permissions with runtime enforcement rather than prompt-only safety;
- sandbox and network policy as separate technical boundaries;
- deterministic lifecycle hooks around tool/session/agent events;
- isolated subagents with scoped context, tools, permissions, and budgets;
- LSP for language-aware diagnostics/navigation instead of custom parsing/indexing;
- Agent Skills for reusable procedural guidance;
- MCP for external tools and read-only resources;
- bounded tool output, task/progress state, cancellation, and agent-native telemetry.

These patterns are references, not authority over this repository. Current source, `AGENTS.md`, `ai-self/CONSTITUTION.md`, repository policy, and the relevant open standards remain authoritative.

## Non-negotiable design principles

### 0. Maintainability foundation first

Plan 039A is a blocking foundation for every later capability. Before adding new coding-agent features, the repository must be refactored and verified against:

- DRY;
- pragmatic SOLID;
- Layered Architecture;
- YAGNI;
- KISS;
- cohesive feature/capability foldering;
- explicit source-file maintainability budgets;
- mandatory documentation and agent-guide synchronization after architecture changes.

Later child plans must follow the maintainability policy established by 039A and must not re-introduce oversized dumping-ground files or flat implementation folders.

### 1. Vendor-neutral core

Implement reusable concepts in repository-owned contracts. Do not hard-code any client-specific configuration format into the runtime.

### 2. Enforcement is not prompting

Model prompts and skills may guide behavior but never constitute a security boundary. Filesystem, process, network, approval, and protected-path rules must be enforced outside the model.

### 3. Keep three permission layers distinct

- **Relay hard boundary:** cannot prompt an external MCP user interactively; it enforces hard allow/deny constraints, sandboxing, OAuth, protected paths, and operator policy.
- **First-party Nuxt approval policy:** can present user approval prompts and persist scoped decisions.
- **Third-party MCP client approval policy:** remains controlled by that client; the relay provides accurate MCP annotations/metadata while still enforcing its own hard boundary.

Do not pretend a first-party approval UI protects direct external MCP client/other-client relay calls.

### 4. Least privilege and narrowing inheritance

A child agent, hook, skill, or plugin-like extension may receive equal or fewer capabilities than its parent/session/operator policy. It must never widen a hard deny or escape the sandbox.

### 5. Sequential implementation of the plan family

Even where implementation could be parallelized, execute Plan 039 child plans in dependency order and validate each before advancing. Subagents may perform focused review/research for the current child plan, but do not run concurrent implementation phases against the same worktree.

### 6. No custom RAG/vector infrastructure

Do not add a vector DB, embeddings service, custom semantic index, or repository memory database merely to imitate another product. Re-evaluate semantic search only if measured lexical + LSP discovery gaps justify it.

### 7. Prefer standards over custom frameworks

- Git for change/history semantics;
- LSP for code intelligence;
- MCP for external tools/resources;
- Agent Skills for reusable procedures;
- existing AI SDK/LangGraph composition for the model loop;
- Bubblewrap for process isolation.

### 8. Bounded everything

Every agent-side capability must have explicit limits for relevant dimensions: output bytes, results, recursion, duration, context contribution, child-agent turns/tokens, concurrency, hook duration, and retained task state.

### 9. Security-sensitive defaults fail closed

Unknown/opaque shell wrappers, interpreters, protected paths, malformed policies, untrusted hooks, invalid LSP responses, stale patch targets, and ambiguous worktree ownership must not silently fall back to broader authority.

## Plan family

| Plan | Capability | Depends on | Status | Exit criteria |
| --- | --- | --- | --- | --- |
| **039A** | Maintainability + layered refactor foundation | 038 | CLOSED / VERIFIED | DRY/SOLID/layering/YAGNI/KISS, folder/file budgets, enforcement, docs and agent guides are verified |
| **039B** | Git read intelligence + patch ergonomics | 039A | CLOSED / VERIFIED | Structured bounded Git inspection and safe patch workflow work through MCP |
| **039C** | LSP code intelligence + diagnostics | 039B | Planned | Definitions/references/symbols/hover/diagnostics work through bounded language-server adapters |
| **039D** | Capability policy, approvals, protected paths, network/exec controls | 039C | Planned | Hard relay policy and first-party approval policy are explicit, input-aware, testable, and non-bypassable |
| **039E** | Deterministic hooks/lifecycle | 039D | Planned | Trusted bounded hooks run at defined lifecycle events and can only preserve/narrow authority |
| **039F** | Subagents + reusable agent profiles | 039E | Planned | Parent can delegate to isolated scoped agents and receive bounded evidence-backed summaries |
| **039G** | Background agents + Git worktree isolation | 039F | Planned | Independent background tasks can run with bounded concurrency and isolated writes without cross-worktree corruption |
| **039H** | Task/context/output management | 039G | Planned | Structured task ledger, context visibility, continuation tokens/pagination, and output budgets are integrated |
| **039I** | Standards-based extension interoperability + MCP resources | 039H | Planned | Skills/agents/hooks/LSP/MCP compose cleanly; read-only resources are exposed where useful without a proprietary marketplace |
| **039J** | Agent UX, observability, security regression, docs, closure | 039I | Planned | Full first-party and remote-MCP agent workflows are proven with truthful bounded evidence |

## Master Todo

- [x] PLAN-039A — maintainability + layered refactor foundation
- [x] PLAN-039B — Git read intelligence + patch ergonomics
- [ ] PLAN-039C — LSP code intelligence + diagnostics
- [ ] PLAN-039D — capability policy, approvals, protected paths, network/exec controls
- [ ] PLAN-039E — deterministic agent hooks/lifecycle
- [ ] PLAN-039F — subagents + reusable agent profiles
- [ ] PLAN-039G — background agents + Git worktree isolation
- [ ] PLAN-039H — task/context/output management
- [ ] PLAN-039I — standards-based extension interoperability + MCP resources
- [ ] PLAN-039J — integrated UX, observability, regression validation, documentation, closure

## 039A verified handoff baseline

Plan 039A closed on 2026-08-17 from branch `refactor/039-maintainability-foundation` with implementation baseline commit `1872ca6ff7bf8572e9bf91ce7ff37ca59733749b`.

Verified handoff facts for 039B:

- `pnpm verify:commit`, `pnpm build`, `pnpm build:tools`, Rust workspace tests, relevant workspace/path/MCP contract/zero-bypass/error-confidentiality checks, maintainability self-test, and final production-preview route smoke are green;
- the largest maintained-source hard budget is enforced at 500 lines and no unexplained hard violation remains;
- review-threshold files are explicit; `app/composables` retains one documented 16-file cohesive Nuxt auto-import exception;
- Rust execution is split by request translation, sandbox construction, and process/job lifecycle; native workspace tools own their own dispatch/result adaptation; transport is split by access control, MCP HTTP routing, and tool adaptation; MCP catalog ownership is separated from protocol facade types; config CLI composition is separated from core config validation;
- sandbox/path/OAuth/security policies remain centralized rather than duplicated across the new modules;
- terminal direct argv already supports values such as `--help` and `--locked`; deterministic workspace integration now guards that behavior while public MCP catalog compatibility remains frozen;
- 039B through 039J remain unimplemented at this boundary.

## 039B verified handoff baseline

Plan 039B closed on 2026-08-17 from branch `feat/039b-git-read-patch` with final implementation/reviewer baseline commit `1b7ed8913070f1c4042d0e91fe8bcfc0418ffc4e`.

Verified handoff facts for 039C:

- the relay now exposes 18 MCP tools: the verified Plan-038 twelve plus bounded native `git_status`, `git_diff`, `git_log`, `git_show`, `git_blame`, and constrained `apply_patch`;
- fixed direct-argv Git execution neutralizes hostile executable helpers/configuration, uses `GIT_OPTIONAL_LOCKS=0`, preserves execution-root/nested-repository semantics, and returns bounded truthful status/diff/history/blame output;
- `apply_patch` performs bounded all-target preflight, no-follow/path/protected-entry validation, stale-target detection, atomic replacement, post-rename commit-state tracking, and best-effort rollback with truthful incomplete-rollback reporting;
- final source validation and an independent read-only security/architecture review are green; the review of the exact committed implementation range returned `NO MATERIAL FINDINGS`;
- the reviewed `ai-tools 0.0.10` release artifact SHA-256 is `03d3ca5cad6add61eee91b02676f4b70dcc96ac1ac0a3632852d5f8e2295aa10`, matching the manually installed relay binary; the restarted user service reported `active`;
- refreshed external MCP client MCP discovery exposed all 18 tools and live authenticated acceptance exercised the complete catalog, including a disposable Git/patch workflow, schema rejection before dispatch, containment rejection, background-job cancellation, and exact canary cleanup; the primary worktree was clean afterward;
- continuation/pagination remains intentionally deferred to Plan 039H; Plan 039C is still unstarted at this handoff boundary.

## Explicit non-goals

- cloning any vendor product UI, proprietary prompts, model routing, or client-specific configuration files;
- building a new general-purpose agent framework beside AI SDK/LangGraph;
- building an extension marketplace/package registry;
- native wrappers for every Unix filesystem operation;
- automatic production/deployment authority;
- secret-store access by default;
- unbounded autonomous loops;
- unlimited child-agent recursion or concurrency;
- peer-to-peer autonomous agent teams before isolated parent-managed subagents and worktrees are proven;
- vector DB / custom RAG / custom AST indexer without evidence of need;
- bypassing repository hooks, reviews, approval policy, or existing security boundaries.

## Cross-plan security invariants

0. Every child plan must preserve the 039A maintainability/layering policy and update documentation/agent guidance whenever architecture or durable workflow changes.
1. Execution root remains a hard filesystem ceiling, not an approval hint.
2. Protected credential paths must be enforced consistently across terminal and native tools.
3. No capability may use shell interpolation for untrusted tool input when direct argv/native APIs can express the operation.
4. Read-only Git tooling must disable or neutralize executable Git features such as external diff/textconv/fsmonitor/config-driven command execution where applicable.
5. LSP processes are untrusted project tooling and must run with bounded workspace/process/network authority.
6. Hook configuration is executable policy; repository hooks are never auto-trusted merely because a repository contains them.
7. Subagents inherit hard policy and may only narrow it.
8. Background writers require isolated worktrees or equivalent proven isolation; no concurrent write agents share one checkout.
9. External MCP integrations receive only explicitly enabled tools; write tools must remain visible as write/destructive to clients.
10. Telemetry records actions and outcomes, never source contents, secrets, prompts, full commands with sensitive values, or raw provider/process errors.

## Verification policy for every child plan

Each child plan must:

- re-read current source and relevant current official standards before implementation;
- reverify workspace identity and Git state before mutation;
- use existing layered-architecture boundaries;
- run narrow deterministic validation after each meaningful change;
- inspect the diff before advancing;
- obtain an independent subagent/security review for security-sensitive boundaries when possible;
- run `pnpm verify:commit` before every normal local commit;
- run stronger relevant security/black-box/build checks before child-plan closure;
- never mark acceptance as passed without real evidence;
- update human/operator documentation and agent-facing guidance whenever the child plan changes durable architecture, workflow, policy, or public capability behavior;
- update this master checklist only after the child plan is genuinely closed.

## Execution order

Execute strictly and sequentially. Do not run implementation of sibling child plans in parallel:

```text
039A Maintainability / layered refactor
  ↓
039B Git read + patch
  ↓
039C LSP intelligence
  ↓
039D Capability policy / approvals
  ↓
039E Hooks / lifecycle
  ↓
039F Subagents / profiles
  ↓
039G Background agents / worktrees
  ↓
039H Task / context / output management
  ↓
039I Extension interoperability / MCP resources
  ↓
039J UX / observability / final closure
```

A child plan may use focused subagents for inspection or independent review of the **current** phase, but the next child plan must not begin until its predecessor is closed with evidence.

## Final initiative acceptance

Plan 039 closes only when Plans 039A through 039J close and a fresh end-to-end review finds no unresolved P0/P1 issue in the new agent-capability surfaces. The final state must remain simpler than a parallel custom agent platform: standards and existing primitives should do most of the work.
