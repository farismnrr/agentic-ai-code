# Plan 039H — Task, Context, and Output Management

**Status:** IMPLEMENTED — FINAL INDEPENDENT VERIFICATION PENDING  
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039G  

## Goal

Give the coding-agent loop explicit task/progress state and predictable context economics: structured todo tracking, context-budget visibility, bounded continuation/pagination for large tool outputs, and compact result references that reduce repeated searches and transcript bloat.

## Current state

- The application already tracks conversation context summaries/compaction and model token limits.
- Plan 038 tools already cap read/search/list output, but list/search tools generally require a fresh query when the caller wants more results.
- Plan 039B/039C introduce potentially large Git/LSP result sets.
- Plan 039F introduces child-agent outputs that should return summaries rather than full transcripts.
- There is no first-class structured task ledger comparable to modern coding-agent todo/progress views.

## Part A — structured task ledger

Add an internal session/conversation task model for agent work. It is not a substitute for `.agents/plans/` and must not persist temporary task state into `ai-self/`.

Suggested task state:

```text
id
title
status = pending | in_progress | blocked | completed | cancelled
depends_on[]
short_note?
updated_at
```

Rules:

- bounded task count/title/note length;
- one clear active task by default;
- tasks may reference a persisted numbered plan but do not rewrite plan truth automatically;
- completed state is a progress aid, not proof that repository acceptance passed;
- ephemeral task state belongs to the conversation/session persistence already owned by the app, not a new database/service unless schema evidence requires a minimal addition.

Candidate internal tool:

```text
task_update(...)
```

The model may use it to keep the UI synchronized, but task completion cannot bypass deterministic validation or Plan-039E stop hooks.

Implementation: `server/application/task-context-output.ts` provides the bounded owner-scoped ledger and `task_update` operation; conversation task APIs and `ChatTaskLedger.vue` expose the compact progress view. Completed is explicitly progress state, never validation or delivery proof.

## Part B — context inspector/budget

Expose a first-party context view showing bounded estimates such as:

- model context window;
- estimated/known used tokens;
- reserved output budget;
- compaction summary presence/cutoff;
- relative contribution of recent messages/tool outputs where available without storing new raw telemetry;
- active subagent/task count;
- whether another compaction is approaching.

Do not pretend token estimates are exact when provider accounting is unavailable.

Implementation: `ChatContextUsage.vue` consumes `/api/conversations/:id/context`, distinguishes provider-measured boundaries from estimates, and renders unknown values as unavailable; the endpoint exposes only bounded metadata.

The model may receive compact budget metadata to encourage efficient tool use, but user-facing context state remains separate from hidden reasoning.

## Part C — continuation/pagination contract

Create one reusable continuation mechanism for tools with potentially large deterministic result sets:

- `directory_list`
- `file_search`
- `text_search`
- Plan-039B `git_diff`, `git_log`, `git_show`
- Plan-039C symbols/references/diagnostics where useful.

Requirements:

- token binds to canonical tool, query parameters, cwd/repository identity, and pagination position;
- token is opaque to the model/user;

Implementation: the shared continuation core uses signed opaque claims with bounded page/total budgets, expiry, scope, canonical query, limit, and snapshot checks. LSP, workspace, text-search, and Git pagination use the same contract; Git object/ref and working-tree snapshots reject stale resumes. Transport/session ownership is added by the application layer where an owner identity is available.

## Implementation notes

- Result references are short-lived, owner-scoped, cardinality/byte/item bounded, and deterministically evicted. They are convenience context only and are not used as authorization or repository truth.
- Output classification is centralized as inline-small, paginated-medium, summarized-large, or retained-failure metadata; raw content is never telemetry.
- No new database, durable memory, vector store, RAG index, hidden reasoning store, or provider accounting source was introduced.
- bounded lifetime or stateless signed encoding; choose the simplest design consistent with security;
- stale underlying state is detected where correctness matters (Git/object refs, file identity/hash, LSP document version);
- caller cannot edit token fields to escape scope or increase limits;
- pagination is not an excuse for unlimited total retrieval—per-session/context budgets remain.

### MCP contract lineage

The Plan-029 v1, Plan-039B v2, and Plan-039C v3 MCP catalogs remain frozen historical evidence. Plan 039H intentionally adds signed continuation fields to the public workspace/search/Git/LSP schemas, so the current runtime is frozen as `.agents/contracts/039h-tool-catalog-v4.json` with hash `ee7b369df33d95e5c799e5f2d8a5efc7774f4c3c1221bf511f3680c981852c0d` and verified by `scripts/phase-039h-contract.sh`. Plan 039H remains `IMPLEMENTED — FINAL INDEPENDENT VERIFICATION PENDING`.

For simple stable operations, an explicit offset may be superior to a stateful cursor. Use cursors only where they materially improve integrity/ergonomics.

## Part D — compact tool-result references

Avoid repeating large content in later steps. Where useful, allow the runtime to assign short-lived result references for bounded outputs such as a diff chunk or subagent report.

Constraints:

- references are session-scoped and expire;
- no custom long-term retrieval database;
- references may point only to data already legitimately available in the session;
- hard memory caps and eviction;
- never use result refs to store secrets/raw sensitive tool output beyond normal policy.

## Part E — output budgeting

Define shared output classes and limits:

- inline small result;
- paginated medium result;
- summarized large result with continuation;
- terminal/background stream with retained head/tail/relevant failure slices.

Agents should get success summaries for noisy successful commands and enough bounded failure output for diagnosis, matching existing Plan-037 long-running execution principles.

## Phases

### PHASE-01 — task-state contract

- [ ] Define schema/lifecycle/bounds.
- [ ] Decide minimal persistence using existing conversation architecture.
- [ ] Add internal task-update interface.
- [ ] Prevent task state from becoming a false acceptance/proof source.

### PHASE-02 — task UX

- [ ] Render compact todo/progress panel in agent mode.
- [ ] Show blocked reasons and dependencies.
- [ ] Integrate subagent/background task status without duplicating runtime state.

### PHASE-03 — context inspector

- [ ] Reuse existing context-compaction/token metadata.
- [ ] Add user-facing context/budget view.
- [ ] Add bounded model-visible budget hints where useful.
- [ ] Verify no hidden chain-of-thought/raw private provider data is exposed.

### PHASE-04 — continuation core

- [ ] Define reusable continuation token/offset contract.
- [ ] Bind continuation to repository/cwd/query/tool identity.
- [ ] Add tamper/staleness/expiry checks where needed.

### PHASE-05 — retrofit existing workspace search/list tools

- [ ] `directory_list` continuation where useful.
- [ ] `file_search` continuation.
- [ ] `text_search` continuation.
- [ ] Preserve existing hard per-call caps/backward compatibility.

### PHASE-06 — Git/LSP continuation

- [ ] Integrate 039B diff/log/show pagination.
- [ ] Integrate 039C reference/symbol/diagnostic pagination.
- [ ] Prove underlying-state changes are handled truthfully.

### PHASE-07 — result refs/output budgets

- [ ] Add bounded session-scoped result references only where repeated reuse saves context materially.
- [ ] Apply shared output policy to child-agent and terminal results.
- [ ] Add memory/eviction limits and telemetry counters without content logging.

## Non-goals

- custom vector/context database;
- durable hidden repository memory;
- replacing numbered implementation plans with ephemeral todos;
- exact cross-provider token accounting when providers do not expose it;
- unlimited retrieval by repeatedly requesting continuation pages.

## Acceptance criteria

- [ ] Agent tasks are explicit, bounded, and visible without becoming false proof of completion.
- [ ] User can inspect context pressure/compaction state meaningfully.
- [ ] Large search/Git/LSP results support safe continuation rather than forced re-query or context flooding.
- [ ] Continuation cannot escape repository/query limits or be tampered into broader access.
- [ ] Result references are short-lived/bounded and do not create a custom retrieval store.
- [ ] Existing compaction behavior has no regression.
- [ ] Mandatory repository and end-to-end agent validation passes.
