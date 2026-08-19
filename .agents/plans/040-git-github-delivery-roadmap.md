# Plan 040 — Reliable Git + GitHub Delivery Workflow Roadmap

**Status:** IN PROGRESS
**Created:** 2026-08-19
**Predecessor:** Plan 039 — Coding Agent Platform Parity (CLOSED / VERIFIED / MERGED)
**Plan family:** 040A–040F

## Goal

Complete the coding-agent delivery loop so an agent can reliably move from a reviewed local change to branch/commit/push/PR/checks/merge/cleanup without depending on an unrestricted networked shell, exposing GitHub credentials to ordinary terminal execution, or holding fragile long-lived MCP HTTP calls open until remote work finishes.

Plan 040 intentionally establishes reliability before mutation:

- **MCP task/transport reliability** — sync fast-path, durable task slow-path, deadlines, polling/backoff, cancellation, retained result, reconnect/resume discipline;
- **local Git mutation** — repository state, branches, commits, merge/rebase/conflicts;
- **remote Git transport** — fetch/push/ref synchronization;
- **forge workflow** — pull requests, checks, reviews, merge and remote cleanup.

The model-facing forge contract should remain neutral where practical. GitHub is the first implementation target because this repository uses GitHub and `gh`, but core contracts must not hard-code GitHub-only concepts where a generic change-request abstraction is sufficient.

## Why reliability is first

Plan 037 already introduced one authoritative long-running terminal job manager, MCP Tasks support, fallback polling/cancellation tools, bounded output retention, and process-tree cleanup. Plan 040 must extend that foundation rather than recreate it.

Current repository inspection still shows reliability gaps above the job manager:

- first-party task handling is shaped around `terminal_exec` rather than a generic slow-operation lifecycle;
- task polling uses a fixed 50 ms loop;
- first-party MCP HTTP round trips do not have an explicit per-request AbortSignal/deadline;
- the client advertises JSON + SSE acceptance while its current result path consumes JSON directly;
- slow future operations such as remote Git, GitHub PR creation/merge, waiting for checks, and multi-agent work must not rely on one HTTP request surviving for their full duration;
- historical positive outbound `http_fetch` reliability was documented as environmentally unproven and should be revalidated while this layer is being hardened.

The target is **not full async everywhere**. Fast bounded calls stay synchronous; slow/external/unbounded calls become durable tasks when appropriate.

## Non-negotiable security/reliability rules

1. Do not solve remote delivery by broadly enabling network + credential access inside ordinary `terminal_exec`.
2. `~/.config/gh`, Git credentials, OAuth tokens and equivalent credential stores remain protected from general-purpose subprocesses.
3. Remote mutation must pass the existing capability/effect/approval policy.
4. Repository identity, origin and target refs must be validated before any remote mutation.
5. Force push, admin merge, branch-protection bypass, destructive ref rewrites and cross-repository operations fail closed unless a future explicit policy allows a narrow case.
6. Tool output, errors and telemetry remain bounded and secret-safe.
7. Merge/rebase conflict resolution must be explicit and reviewable; never silently choose a side just to make Git clean.
8. Local and remote state transitions must be truthful: do not claim pushed/merged/deleted until independently observed.
9. HTTP request lifetime and durable task lifetime are separate concepts.
10. A disconnect or request timeout must not be treated as implicit task cancellation.
11. External mutation retry/resume must be idempotent or deduplicated; retries must not create duplicate PRs/comments/merges/pushes.
12. No hot unbounded polling loops; cadence, concurrency, retained task count, result bytes and TTL are bounded.

## Execution guide — mandatory sequential workflow

**Do not implement all child plans at once.**

Execute exactly in this order:

1. implement **040A** only;
2. run its focused tests + mandatory repository gates + live timeout/disconnect/cancellation proof;
3. obtain independent review because task/transport behavior is security- and reliability-sensitive;
4. mark 040A CLOSED / VERIFIED and merge it according to repository policy;
5. re-read current `main` before beginning **040B**;
6. repeat the same process for 040B, then 040C, 040D, 040E, and 040F;
7. do not start Plan 041 until Plan 040 is fully CLOSED / VERIFIED / MERGED;
8. follow [Plans 040–042 execution guide](../roadmap-execution-guide-040-042.md): do **not** restart/redeploy the relay per child or phase. Runtime restart/deployment and ChatGPT connector/action resync are operator-owned. Continue implementation and deterministic verification against source/build artifacts until a genuine live-runtime checkpoint requires loading the new runtime, then stop and provide the user the exact reviewed restart/deploy and resync commands/steps.

A child plan may use read-only/review subagents, but sibling child-plan implementations must not run concurrently against the same worktree. If implementation of one child exposes a prerequisite gap owned by an earlier child, stop and remediate the earlier boundary rather than papering over it in a later child.

## Child plans

| Plan | Capability | Depends on | Status | Exit criterion |
| --- | --- | --- | --- | --- |
| 040A | MCP task + transport reliability | 039 / 037 foundation | CLOSED / VERIFIED / MERGED (2026-08-19) | Fast calls remain sync; slow calls can use bounded durable tasks with explicit HTTP deadlines, backoff, cancellation, retained results, reconnect/retrieval semantics and live relay proof |
| 040B | Local Git mutation + structured conflict workflow | 040A | PLANNED | Branch/commit/merge/rebase/abort/continue work safely through bounded native contracts with explicit conflict state |
| 040C | Remote Git transport | 040B | PLANNED | Fetch/push/ref sync work through a narrow credential-isolated path with remote/ref validation, idempotency and policy enforcement |
| 040D | Forge abstraction + GitHub/`gh` adapter | 040C | PLANNED | Generic change-request/read-check primitives map safely to GitHub without exposing `gh` credentials to normal terminal execution |
| 040E | Pull-request lifecycle + merge | 040D | PLANNED | Agent can create/update/read PRs, inspect checks/reviews, wait reliably, merge with policy, and clean remote branches truthfully |
| 040F | End-to-end delivery orchestration + UX/observability | 040E | PLANNED | First-party and remote-MCP workflows prove branch → commit → push → PR → checks → merge → cleanup with correct approvals, retries/task semantics and audit evidence |

## Master todo

- [x] 040A — MCP task + transport reliability foundation
- [ ] 040B — local Git mutation + structured conflicts
- [ ] 040C — remote Git transport
- [ ] 040D — forge abstraction + GitHub adapter
- [ ] 040E — PR/check/review/merge lifecycle
- [ ] 040F — integrated delivery loop and closure

## Target end-to-end workflow

```text
inspect
  ↓
create/switch branch
  ↓
edit + validate
  ↓
commit
  ↓
push (task-eligible if remote latency requires)
  ↓
create/update PR (idempotent external mutation)
  ↓
inspect/wait for checks (durable task/polling, no fragile long request)
  ↓
remediate if needed
  ↓
merge (explicit approval + idempotent remote mutation)
  ↓
delete remote/local feature branch
  ↓
sync integration branch
  ↓
verify local/remote parity
```

Every transition must be independently observable and bounded.

## Explicit non-goals

- arbitrary shell access to GitHub credentials;
- making every MCP tool asynchronous;
- inventing a proprietary WebSocket/background protocol when current MCP task/transport mechanisms suffice;
- auto-bypassing branch protection or required reviews;
- self-approving a protected merge merely because CI passes;
- GitHub-specific model contracts when a generic forge concept is sufficient;
- supporting every forge in Plan 040; GitHub is the first adapter, architecture remains extensible;
- building a custom Git implementation instead of invoking Git with hardened direct argv / native parsing;
- automatically resolving semantic conflicts without inspecting both sides and validating the result;
- indefinite persistence of task results or raw command/network output.

## Cross-plan handoff

Plan 040A's generic task/transport reliability becomes infrastructure reused by:

- Plan 040 remote Git/forge/check waiting;
- Plan 041 potentially slow LSP/toolchain operations where task semantics are justified;
- Plan 042 multi-agent orchestration, which must never invent a second background lifecycle when 040A/037 can supply one.

## Closure criteria

Plan 040 closes only when:

- 040A–040F are individually CLOSED / VERIFIED / MERGED;
- ordinary terminal credential isolation remains intact;
- a fresh independent security/reliability review reports zero unresolved P0/P1 findings in the new task, transport and delivery surfaces;
- deterministic and live acceptance prove timeout/disconnect/cancellation/result-retrieval plus the complete Git + GitHub delivery loop;
- documentation and operator guidance explain sync vs task behavior and when native Git/forge tools are used versus `terminal_exec`;
- ChatGPT live proof does not depend on an indefinitely held HTTP request;
- no closure claim depends on unverified remote state.
