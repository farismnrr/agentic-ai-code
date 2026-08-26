---
name: implementation-planning
description: Use whenever the user asks to buat plan, bikin plan, pecah kerjaan step by step, create an implementation/technical/phased/migration/refactor/upgrade plan, split large work into multiple plans, or produce a structured roadmap and todo checklist before execution.
license: MIT
---

# Implementation Planning

Use this skill whenever the user asks to "buat plan", "bikin plan", "plan this", "implementation plan", "break this down", or requests a phased technical design/migration/refactor/upgrade plan.

## Default behavior

- All implementation plans must be written in **English**, even when the user speaks another language, unless the user explicitly requests a different language.
- Planning is **plan-only** by default. Do not edit implementation files merely because a plan was requested.
- If the user asks to plan **and** execute, create/review the plan first, then execute it.
- Resolve ambiguity from the repository, current conversation, existing docs, and available tools before asking questions.
- If a non-blocking detail cannot be resolved, record a bounded assumption and proceed. Do not invent precise facts such as file paths, APIs, line numbers, or commands.
- Do not include time estimates unless the user explicitly asks for them.

## Source and authority

Apply the authority order from `ai-self/CONSTITUTION.md`. This local skill is the routing authority for planning. Installed planning skills may be used as references, especially `.agents/skills/create-implementation-plan/SKILL.md` for deterministic phase/task structure.

## Planning workflow

### 1. Establish the outcome

State the goal, success criteria, scope, explicit non-goals, and constraints already supplied by the user or repository. Do not broaden the requested scope.

### 2. Inspect before designing

For an existing repository, inspect enough real context to avoid a speculative plan:
- repository structure and current Git state;
- `README`, `CONTRIBUTING`, architecture/design docs, and existing plan conventions when relevant;
- exact modules/files/interfaces involved;
- tests, build/lint commands, CI, migrations, deployment/config boundaries when relevant;
- relevant `ai-self` skills and repository-specific instructions.

Prefer verified paths and symbols. If a path or interface cannot be verified, make discovery an explicit early task rather than guessing.

### 3. Choose the implementation shape

Before decomposing tasks:
- identify the smallest coherent architecture;
- preserve existing patterns unless the task explicitly requires redesign;
- note meaningful alternatives and why one is preferred;
- identify dependencies, sequencing constraints, parallelizable work, and team/review boundaries.

Apply KISS, DRY, YAGNI, least privilege, and repository conventions.

### 4. Keep one plan by default; split only when independence is real

Default to one numbered plan file for the initiative and decompose it into phases/tasks inside that file. Cross-subsystem scope, many phases, or a long checklist are not sufficient reasons to create child plans. Split into child plans only when parts are genuinely independently deliverable/reviewable, have materially separate owners/lifecycles, or the single file would become impractical to execute and maintain. When a split is truly justified, keep dependencies and handoffs explicit and avoid administrative fragmentation.

### 5. Decompose each plan into phases

Each phase must:
- have one measurable goal;
- produce a coherent, reviewable result;
- declare dependencies on earlier phases;
- end with explicit validation and exit criteria;
- be independently understandable by an engineer or agent.

Use stable IDs: `PHASE-01`, `TASK-001`, `TASK-002`, etc.

### 6. Make tasks atomic and executable

Each task should contain:
- **Outcome**: one concrete deliverable.
- **Files**: exact create/modify/test paths when verified.
- **Dependencies**: prior task IDs or `none`.
- **Steps**: ordered checkbox actions.
- **Validation**: exact relevant command/check and expected condition where known.
- **Commit boundary**: logical commit intent when repository changes are produced.

A task is too large when it contains multiple independently reviewable outcomes. A task is too small when it is merely mechanical setup that only exists to support the same deliverable.

Do not use placeholders such as `TBD`, `TODO`, "add appropriate validation", or "write tests" without specifying what behavior must be verified.

### 7. Plan validation and failure handling

Include relevant unit/integration/e2e validation, lint/type/build/config/schema checks, migration/compatibility checks, rollback/recovery, security boundaries, and CI/PR/reviewer gates for team repositories.

Never plan to bypass hooks, required checks, reviews, CODEOWNERS, or protected-branch policies.

### 8. Review the plan before handoff

Check that:
- every requirement maps to at least one task;
- task ordering is valid;
- no unverified implementation detail is presented as fact;
- no duplicate or contradictory tasks exist;
- all risky actions have a guard/rollback;
- validation covers the requested outcome;
- no implementation work was accidentally performed during plan-only mode.

## Required output structure

Write every plan in English.

Use this structure unless the repository already has a stricter plan template:

```markdown
# <Title> Implementation Plan

**Status:** Planned
**Goal:** <one sentence>
**Success Criteria:** <observable completion conditions>

## Scope
### In scope
- ...
### Out of scope
- ...

## Current State
- verified repository/context facts

## Constraints & Decisions
- constraints
- chosen architecture and rationale
- alternatives rejected when material

## Phase Overview
| Phase | Goal | Depends On | Exit Criteria |
|---|---|---|---|

## PHASE-01: <name>
**Goal:** ...
**Dependencies:** none

### TASK-001: <name>
**Outcome:** ...
**Files:**
- Modify: `path`
- Test: `path`

**Steps:**
- [ ] ...
- [ ] ...

**Validation:**
- `command` → expected condition

**Commit boundary:** `type(scope): intent`

**Phase exit criteria:**
- [ ] ...

## Risks & Rollback
- Risk → mitigation / rollback

## Final Acceptance Criteria
- [ ] ...

## Execution Handoff
- execution order
- parallelizable tasks
- approvals/external actions still required
```

## Multi-plan structure — exception only

Use a master roadmap plus child plans only when the work has genuinely independent deliverables, owners, release/review lifecycles, or a single plan file would be impractical. Large size by itself is not enough; prefer one plan with phase IDs and task IDs.

When a multi-plan split is justified, the master roadmap must contain:

- a table of child plans with goal, dependency, status, and exit criteria;
- a master checklist;
- execution order and parallelizable plans;
- cross-plan risks/decisions;
- final initiative acceptance criteria.

Each child plan must contain its parent roadmap, plan ID, dependencies, its own task checklist, validation, rollback, acceptance criteria, and handoff to the next dependent plan. Do not start a dependent child plan until its prerequisite exit criteria are satisfied.

## Plan persistence

Before persisting a plan file, apply `workspace-scope` and verify the target repository. Conversational planning may inspect multiple candidates read-only, but plan files must never be written to a guessed or previously used project.

- If the repository already has a plan directory/convention, follow it.
- If the user explicitly asks for a plan file, persist it using that convention.
- If no convention exists and persistence is requested, prefer `docs/plans/YYYY-MM-DD-<topic>.md`.
- If the user only asks for a conversational plan, do not create repository files unless the surrounding workflow clearly expects a plan artifact.

## Team repositories

For shared repositories:
- branch/review/CI policy belongs in the plan;
- do not plan direct commits to default/shared branches;
- identify CODEOWNERS/reviewer/PR gates when discoverable;
- separate local implementation steps from external collaboration actions requiring approval.

## Handoff to execution

When execution begins:
1. re-check Git/worktree state and assumptions that may have gone stale;
2. execute phases in dependency order;
3. validate each phase before advancing;
4. use `github-delivery` for task-owned commit/push behavior;
5. update persisted plan status/checklists only when the repository uses them.

## Maintainability refactor planning

For structural refactors, treat line/file counts as discovery signals rather than decomposition goals. First map public callers, security/policy owners, and independent reasons-to-change; preserve stable facades where framework or client contracts depend on them; split only cohesive responsibilities; then run a DRY/SOLID/layering/YAGNI/KISS deletion pass. If a framework-owned public directory legitimately crosses a count budget, prefer one narrow exact-path exception with a concrete reason over wrapper spam. Make documentation/agent-guide synchronization an explicit blocking acceptance step whenever module or folder ownership changes.
