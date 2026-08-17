# Plan 039G — Background Agents and Git Worktree Isolation

**Status:** PLANNED  
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039F  

## Goal

Extend proven sequential subagents into safe background work with bounded concurrency and Git-worktree isolation, allowing genuinely independent coding/review tasks to proceed without multiple writers corrupting the same checkout.

## Principle

Concurrency is an optimization, not the default. Use it only when tasks are independent. Shared-file or tightly dependent work stays sequential.

## Required capabilities

- background child lifecycle: start/status/cancel/result;
- bounded concurrency per parent/user/relay;
- explicit `shared_read` vs `worktree` isolation;
- automatic dedicated Git worktree for background writer agents;
- parent-owned integration/review step;
- cleanup that never deletes an unmerged/dirty worktree silently;
- no force push/history rewrite/merge autonomy.

## Worktree security model

A writing background child must never share the parent's mutable checkout.

Required flow:

```text
verify canonical repo
 -> verify clean/suitable base according to task policy
 -> allocate task-owned branch/worktree
 -> lock ownership metadata
 -> child operates only inside worktree path
 -> validate child result
 -> parent reviews diff/commits
 -> integration remains explicit and policy-governed
 -> cleanup only when safe and owned
```

Worktree paths must stay within a configured safe worktree root under the execution boundary. Symlink/path escape protections still apply.

## Background task model

Candidate internal operations:

- `agent_task_start`
- `agent_task_get`
- `agent_task_cancel`

These are agent-runtime operations, not necessarily public relay MCP tools. Decide the correct application/internal surface during implementation.

Each task records bounded metadata:

```text
task_id
parent_session_id
agent_profile
repository_identity
isolation
branch/worktree identity
state
started_at
bounded progress summary
result reference
```

Do not persist raw child transcripts merely for polling.

## Concurrency policy

Start conservatively:

- configurable global/user/session caps;
- default no more than a small number of active background agents;
- only read-only agents may share a checkout concurrently;
- writing agents require distinct worktrees;
- same target branch/file ownership conflicts should be detected before starting when possible;
- CPU/memory/process limits must align with relay admission controls.

## Parent integration

Background agents do not self-merge.

The parent must:

1. retrieve bounded result + Git diff/status;
2. inspect validation evidence;
3. decide whether to accept, request follow-up, cherry-pick/merge manually through normal Git workflow, or discard;
4. follow existing `github-delivery` policy for commits/pushes.

## Agent teams

Industry tools also expose peer-to-peer agent teams. This plan should implement **parent-coordinated background agents first**. After that is proven, evaluate a minimal shared-task-list/team mode.

If team mode is implemented in this plan family:

- communication is structured messages through the parent/runtime, not arbitrary sockets;
- no hidden shared mutable memory;
- every writer remains worktree-isolated;
- bounded teammate count and message volume;
- no teammate may broaden another teammate's permissions;
- use only where workers genuinely need cross-communication.

Do not make peer-to-peer teams a prerequisite for ordinary coding-agent usefulness if the added coordination cost outweighs benefit.

## Phases

### PHASE-01 — background task lifecycle

- [ ] Define state machine, polling/result/cancel semantics, retention, budgets.
- [ ] Reuse terminal-job lifecycle lessons where applicable without coupling agent tasks to process jobs.

### PHASE-02 — read-only background agents

- [ ] Run explore/review children asynchronously with shared-read isolation.
- [ ] Prove parent continues independently and can poll/cancel.
- [ ] Bound output/context retention.

### PHASE-03 — worktree allocator

- [ ] Safe canonical repo/worktree discovery.
- [ ] Task-owned branch naming and collision handling.
- [ ] Contained worktree root.
- [ ] Ownership lock/metadata.
- [ ] Refuse unsafe dirty/conflicting conditions rather than guessing.

### PHASE-04 — writer background agents

- [ ] Restrict writer cwd/tools to allocated worktree.
- [ ] Run normal hooks/policy/LSP inside child worktree.
- [ ] Return status/diff/validation evidence to parent.
- [ ] Never auto-integrate.

### PHASE-05 — cleanup/recovery

- [ ] Safe cancel process tree and child session.
- [ ] Preserve dirty/unmerged worktrees for inspection.
- [ ] Remove only clean, task-owned disposable worktrees with explicit safe criteria.
- [ ] Recover stale task metadata after process restart where feasible.

### PHASE-06 — bounded multi-agent coordination evaluation

- [ ] Prove at least one genuinely independent multi-agent scenario.
- [ ] Measure overhead vs sequential subagents.
- [ ] Implement minimal team messaging/shared task list only if evidence justifies it; otherwise document parent-coordinated background agents as the intentional simpler standard.

## Acceptance criteria

- [ ] Read-only background agents are cancellable and bounded.
- [ ] Concurrent writers never share one mutable checkout.
- [ ] Worktrees/branches are task-owned, contained, collision-safe, and never destructively cleaned while dirty/unmerged.
- [ ] Background agents cannot merge/push/rewrite history autonomously beyond existing policy.
- [ ] Parent receives bounded diff/validation evidence before integration.
- [ ] Concurrency limits prevent resource explosion.
- [ ] Multi-agent/team complexity is added only if validated benefit exceeds coordination cost.
