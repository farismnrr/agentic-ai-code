# Plan 039F — Subagents and Reusable Agent Profiles

**Status:** CLOSED / VERIFIED (2026-08-26)
**Created:** 2026-08-16  
**Parent:** [Plan 039 — Coding Agent Platform Parity Roadmap](039-coding-agent-platform-parity-roadmap.md)  
**Depends on:** Plan 039E  

**Closure evidence:** `pnpm verify:subagents` passed, with background and
task/context integrations covered by `pnpm verify:background-agents`,
`pnpm verify:task-context-output`, and `pnpm verify:039j`. The later 039J and
master-039 closure records supersede this stale independent-verification header.

## Goal

Add parent-managed subagents with isolated context, scoped tools, narrowed permissions, explicit budgets, and reusable vendor-neutral agent profiles so exploration/review/verification can happen without flooding or over-privileging the main conversation.

## Core model

A subagent is a child model session that receives only explicitly selected task context and returns a bounded result to its parent. It does not share hidden reasoning, mutable model state, or unrestricted parent tools.

Required invariants:

- separate context window/session identity;
- parent passes explicit task + bounded context package;
- child hard policy = intersection(parent policy, agent-profile policy, operator policy);
- child may never widen permissions;
- toolset is allowlisted per role;
- child has max turns/tokens/tool calls/wall time;
- parent receives a concise result + structured evidence references, not the entire child transcript by default;
- cancellation propagates parent → child;
- recursion/depth is bounded.

## Built-in profiles

Start with a small useful set inspired by industry roles but owned by this repository:

### `explore`

- read-only workspace/search/Git/LSP;
- no mutations;
- optimized for codebase discovery;
- returns findings with paths/symbols/evidence.

### `plan`

- read-only plus planning skills;
- no implementation;
- produces executable plan/checklist grounded in inspected source.

### `review`

- read-only Git diff/history/LSP/diagnostics;
- focuses on correctness, security, architecture, regressions;
- no code mutation by default.

### `verify`

- read + approved execution of repository validation commands;
- no source mutation unless explicitly enabled for a specific workflow;
- summarizes pass/fail and relevant bounded failure output.

### `general-purpose`

- broader toolset for complex delegated work;
- still inherits parent hard policy and budgets;
- mutation capability is allowed only when the parent/session mode permits it.

Do not create dozens of persona agents.

## Agent profile format

Create a vendor-neutral repository profile format under `.agents/agents/`, likely Markdown + YAML frontmatter to align with existing skills and keep profiles reviewable.

Candidate fields:

```yaml
name:
description:
model_policy: default|fast|strong
tools:
  allow: []
  deny: []
effects:
  allow: []
max_turns:
max_tool_calls:
max_output_tokens:
working_mode: read-only|workspace
skills: []
```

The profile body contains role instructions. Exact schema must be validated and documented.

Project profiles must not embed secrets or executable credentials.

## Delegation interface

Expose a parent-only internal agent capability rather than a public remote shell primitive. Candidate tool:

```text
delegate_task(
  agent,
  task,
  cwd?,
  context_refs?,
  isolation? = shared_read,
  budget?
)
```

The parent runtime resolves the named profile, intersects policy, starts a child session, and returns:

```text
status
summary
findings[]
evidence[]
validation[]
remaining_risks[]
```

Do not dump raw internal reasoning.

## Context packaging

Prefer references and selective retrieval over copying the parent conversation:

- task statement;
- verified workspace/repository identity;
- explicitly selected relevant messages/summary;
- relevant plan/skill/profile;
- current Git status/diff refs when needed;
- file/symbol references the child can re-read itself.

Child should use tools to inspect source instead of receiving huge pasted source blocks.

## Model routing

Support profile hints (`fast`, `default`, `strong`) mapped through existing user-configured providers/models. Never hard-code one vendor/model name into repository profiles.

The parent may choose a stronger model only within user configuration and budget policy. Profile hints are advisory within allowed models, not authority to access another user's/provider's model.

## Sequential-first rule

Plan 039F implements **one active child at a time per parent** first. Background/concurrent agents belong to Plan 039G after isolation is proven.

This keeps correctness and workspace ownership simple while the delegation contract stabilizes.

## Phases

### PHASE-01 — runtime contract

- [x] Define child session identity/lifecycle/result schema.
- [x] Define policy intersection/narrowing.
- [x] Define budgets, depth limit, cancellation, failure semantics.
- [x] Define context package and telemetry privacy.

### PHASE-02 — agent-profile loader

- [x] Add `.agents/agents/` convention and schema.
- [x] Validate names/descriptions/tool/effect rules.
- [x] Reject unknown capabilities and malformed profiles.
- [x] Add built-in `explore`, `plan`, `review`, `verify`, `general-purpose` profiles without duplicating skill bodies.

### PHASE-03 — child model execution

- [x] Reuse existing AI SDK provider/model composition.
- [x] Create isolated child context and narrowed toolset.
- [x] Enforce budgets and cancellation.
- [x] Keep child lifecycle out of conversation persistence unless explicitly required for UX/audit.

### PHASE-04 — delegation tool/orchestrator

- [x] Add parent-only delegation capability.
- [x] Resolve profile and cwd/repository scope safely.
- [x] Return bounded structured summary/evidence.
- [x] Prevent child from spawning another child initially.

### PHASE-05 — role acceptance

Prove:

- explore cannot write;
- plan cannot implement;
- review cannot mutate;
- verify can run approved checks but cannot silently fix code;
- general-purpose remains bounded by parent mode/policy;
- policy/profile conflicts always narrow or deny.

### PHASE-06 — hooks/skills integration

- [x] Load only explicitly relevant skills/profile-selected skills.
- [x] Fire bounded subagent lifecycle events.
- [x] Ensure skills cannot grant tools/effects not already allowed.

### PHASE-07 — first-party UX contract

- [x] Parent transcript shows delegation start/status/result compactly.
- [x] User can inspect the child summary/evidence and cancel a running child through the existing stop boundary.
- [x] Raw chain-of-thought is never exposed as a requirement.

## Non-goals

- peer-to-peer agent teams in this plan;
- unlimited recursive delegation;
- concurrent writer agents in one worktree;
- custom model provider bypass;
- hidden persistent memory database;
- copying every parent message into every child.

## Acceptance criteria

- [x] Parent can delegate focused work to isolated scoped children.
- [x] Built-in profiles cover explore/plan/review/verify/general-purpose without duplication.
- [x] Child tool/policy authority is always equal or narrower than parent/operator authority.
- [x] Context and output are bounded; results return as concise evidence-backed summaries.
- [x] One-child-at-a-time behavior is stable before Plan 039G introduces concurrency.
- [x] Cancellation, failure, telemetry, and approval behavior are implemented.
- [x] Repository verification and live agent acceptance pass.
