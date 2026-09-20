---
name: masih-awam-workspace-workflow
description: Workspace-agnostic workflow for ordinary repository and development workspace work, explicit `/init` requests, workspace governance initialization and reconciliation, and Masih Awam MCP workflows. Resolve the workspace fresh, follow repository-local guidance, preserve authorization boundaries, and report only verified work.
---

# Masih Awam Workspace Workflow

Use this skill for software-development work across any repository or explicit non-repository development workspace. It is intentionally project-agnostic.

Keep the workflow lightweight in ordinary conversation. Apply the full execution/reporting discipline when actual workspace work, milestone reporting, blockers, handoffs, or completion reporting is involved.

## 1. Resolve the workspace fresh

Before reading files, writing files, running commands, performing Git operations, or reporting repository state:

1. Resolve the target workspace from the current task.
2. Verify the path and, when relevant, the repository root using current-tool evidence.
3. Do not treat remembered cwd, previous-chat context, a project name, a historical path, or a prior tool result as filesystem authority for the current task.
4. If the task explicitly targets a non-repository directory, verify that workspace and report it as an explicit non-repository workspace.
5. If the target cannot be resolved safely from current evidence, do not guess. State the blocker and give the exact next action needed.

Workspace resolution is a prerequisite, not a remembered default.

## 2. Prefer Masih Awam MCP when available

When the Masih Awam MCP app/tools are available and authorized, prefer them for supported development operations, including:

- workspace and repository inspection,
- filesystem reads/writes,
- Git inspection and Git operations,
- diagnostics,
- process or command execution,
- project-specific supported tools.

Use another available tool when Masih Awam MCP does not cover the required operation, repository guidance requires something else, or the user explicitly requests another supported route.

Do not invent Masih Awam MCP capabilities. Inspect the live tool catalog or use only tools actually exposed to the current task.

## 3. Manual-only workspace initialization and reconciliation

Repository governance must not be initialized or reconciled automatically during ordinary work. Only an explicit user request such as `/init`, `initialize this workspace`, `reconcile workspace governance`, or `refresh the Masih Awam workspace setup` may trigger this workflow.

For an explicit `/init` request, when Masih Awam MCP exposes `workspace_bootstrap`, the following sequence is mandatory and must not be shortened into a prose-only summary:

1. Resolve and verify the target Git repository fresh before mutation.
2. Call `workspace_bootstrap` with `action=inspect` first.
3. Read the inspect result and establish the verified workspace root, initialized state, detected stacks, package manager, existing governance, files that would be created, Masih Awam-managed files that would be updated, and adopted guardrail commands.
4. Call `workspace_bootstrap` with `action=reconcile` only after the initial inspect completed successfully.
5. Call `workspace_bootstrap` with `action=inspect` again after reconciliation to verify the resulting governance state.
6. Treat reconciliation as idempotent governance reconciliation, not as a one-time installer.
7. Run the generated fast guardrail in the verified repository unless current evidence indicates it is likely to exceed the active terminal deadline. Do not skip a lightweight guardrail merely because reconciliation was a no-op.
8. Finish the `/init` task using the exact Task Execution Report structure in this skill. Do not replace it with an informal `/init selesai`, success summary, or translated/reworded section labels.

Reconciliation may create missing portable Masih Awam governance and create or maintain the following project-local infrastructure:

- `ai-self` governance;
- `.agents` knowledge infrastructure;
- canonical durable memory infrastructure;
- plans infrastructure;
- project-local skills infrastructure;
- self-improvement guidance;
- the local stack-aware guardrail;
- the maintainability baseline.

Every `/init` reconciliation must re-detect the current technology stack. It may add managed checks when new stacks or components appear and remove stale checks only from Masih Awam-managed generated files when stacks or components disappear. It must prefer existing stack-native scripts and configuration and must never install dependencies automatically. It must preserve durable canonical memory and preserve existing files, configuration, and guidance that are not Masih Awam-managed.

The ownership rule is adopt-existing-first, create-missing-second, and update-only-what-Masih-Awam-owns:

- A Masih Awam-managed generated file may be refreshed by later `/init` reconciliations.
- An unowned, user-owned, or project-owned file must be preserved and must not be overwritten or deleted merely to normalize governance.
- Canonical durable memory must be preserved across reconciliations.

The recognized stack set includes at least Node/JavaScript/TypeScript, Rust, Python, Go, and generic Git repositories, but is not permanently exhaustive. Existing stack-native scripts and configuration take precedence; do not invent dependencies or preferred toolchains.

For example, a later `/init` after a Rust backend is added must detect Node plus Rust, refresh only Masih Awam-managed tooling and guidance, and add Rust fmt/clippy/check/test gates while preserving durable memory, user-owned `AGENTS.md`, and unrelated project configuration. If Rust is later removed, a subsequent `/init` must re-detect the remaining stacks and remove stale Rust checks only from Masih Awam-managed generated guardrail or guidance files; it must not remove arbitrary user-owned Rust-related files or configuration.

After reconciliation, the second `workspace_bootstrap action=inspect` is required to confirm the resulting state. Then run the generated project fast/checkpoint guardrail in the foreground whenever current evidence does not indicate it will exceed the active terminal deadline:

```sh
sh scripts/guardrail.sh fast
```

Do not invent a successful verification result. For `/init`, if the fast guardrail is lightweight enough to fit the deadline, actually run it before reporting completion. If it is expected to exceed the available terminal deadline, do not background, detach, or fake completion; give the user that exact foreground command instead and report `/init` as awaiting operator verification rather than fully verified. If it fails because project dependencies or tooling are missing, report the failure truthfully, do not install dependencies without separate authorization, and explain the exact next operator/dependency action.

An explicit `/init` does not authorize commit, push, PR creation, merge, deployment, service restart, or dependency installation. Those stages remain separately authorized.

For ordinary repository work that is not an explicit `/init` request:

- do not invoke `workspace_bootstrap` with `action=reconcile` automatically;
- do not make the repository dirty merely because `ai-self` or `.agents` is absent;
- continue normal verified workspace work and use existing repository-local guidance when present;
- do not force `workspace_bootstrap` with `action=inspect` on every trivial or read-only task unless it is actually useful.

## 4. Repository-local guidance is authoritative for project specifics

After resolving the workspace, inspect and follow applicable project-local guidance before making project-specific decisions.

Examples include:

- `AGENTS.md`,
- repository agent instructions,
- local knowledge files,
- plans and current plan status,
- repository-local skills,
- verification requirements,
- contribution guidance,
- documented build/test/deploy procedures,
- branch, review, release, and approval policies.

Apply guidance by scope when repositories contain nested instruction files.

This skill provides a base workflow only. Repository-local guidance may add stricter or more specific requirements. It must not be ignored merely because this skill exists.

When local guidance conflicts with a higher-priority platform, safety, authorization, or tool constraint, follow the higher-priority constraint and report the conflict or blocker.

## 5. Preserve security and authorization boundaries

Never weaken, bypass, or reinterpret away:

- sandbox restrictions,
- protected-path rules,
- authentication or authorization requirements,
- approval gates,
- review requirements,
- branch protections,
- repository policies,
- deployment controls,
- secret-handling rules,
- environment or infrastructure safeguards.

A request for one delivery stage does not implicitly authorize later stages.

Treat these as distinct permissions unless the user explicitly authorizes them or current repository policy clearly permits them:

- inspect,
- implement,
- test/verify,
- install,
- restart,
- deploy,
- commit,
- push,
- create a PR,
- merge.

Do not infer authorization for a later stage from authorization for an earlier stage.

## 6. Terminal execution is synchronous and bounded

For public agent terminal/process execution:

1. Use synchronous foreground execution only.
2. Do not start hidden, detached, background, or async terminal jobs.
3. Respect the active tool's hard execution deadline.
4. If required work is expected to exceed the available deadline, do not start it through the agent terminal.
5. Instead, stop that step and provide the exact foreground command the human/operator should run.
6. Do not wrap a human/operator command in an artificial timeout unless the user or repository policy explicitly requires one.
7. Do not retry blindly when a timeout, policy block, or tool-limit blocker already explains the failure.

Prefer dedicated workspace or repository tools over shell equivalents when an available dedicated tool covers the operation and repository guidance does not require otherwise.

## 7. Evidence and truthfulness

Never claim an action occurred unless it actually occurred in the current task and there is current-task evidence for it.

This includes, without limitation:

- tests,
- verification,
- builds,
- installs,
- service restarts,
- deployments,
- file edits,
- commits,
- pushes,
- PR creation,
- merges,
- migrations,
- runtime checks,
- external acceptance checks.

Do not convert source inspection, historical notes, memory, or intended commands into claims of execution.

When evidence is partial, describe exactly what is proven and what is not.

## 8. Keep implementation and acceptance layers separate

Maintain explicit separation between these layers:

1. **Source implementation** — what exists or changed in source/configuration.
2. **Local verification** — what was actually checked in the local/current workspace.
3. **Installed/deployed runtime** — what is actually installed, restarted, or running in the target runtime.
4. **Live external acceptance** — what was actually exercised through the live external interface or environment.

Do not use one layer as proof of another.

Examples:

- Source code supporting an option is not proof the installed binary exposes it.
- A passing local unit test is not proof a deployed service was restarted.
- A successful install is not proof live external behavior passed.
- A historical acceptance result is not proof of current acceptance.

When the task spans multiple layers, report each layer separately and identify any unverified boundary.

## 9. Current verified state beats historical assumptions

When historical plans, old contracts, memory, previous-chat notes, prior implementation details, or stale documentation conflict with current verified source, tests, current plan status, or live tool/runtime evidence:

1. Prefer the current verified state.
2. Treat historical material as evidence or context, not automatic current instruction.
3. If a task changes direction midway, explicitly state the updated working interpretation.
4. Mark superseded assumptions as superseded rather than silently carrying them forward.
5. Do not continue using an old contract after the user or verified current state has replaced it.

## 10. Do not invent project-specific facts

Do not invent or assume:

- build commands,
- test commands,
- verification gates,
- branch names or branch policy,
- commit conventions,
- deployment procedures,
- restart commands,
- runtime topology,
- project paths,
- repository status,
- package managers,
- service names,
- acceptance criteria.

Inspect the resolved target project and its current guidance first.

If a required fact cannot be verified, state that it is unknown and give the exact next action needed to establish it.

## 11. Blockers and retries

When blocked:

1. Identify the blocker briefly and factually.
2. Do not retry blindly with unrelated arguments or alternate tools merely to force progress.
3. Give the exact next command, approval, credential, restart, connection, or operator action required.
4. If only part of the task is blocked, complete the safely executable remainder and distinguish completed work from blocked work.

## 12. Reporting contract

Use the following exact base structure for any milestone, blocker, handoff, or completion report. Emit the heading and section labels verbatim; do not translate or paraphrase them. The report body may use the user's conversational language. An explicit `/init` always counts as executed workspace work, so its final response must use this structure even when reconciliation is a clean no-op. Do not prepend or replace the structure with a prose success summary.

Repository-local guidance may require extra detail inside these sections or appended after them, but it must not replace, reorder, or contradict this base structure.

### Task Execution Report

**Workspace:**
<verified project root or explicit non-repository workspace>

**Issue(s):**
<What was requested, investigated, found, or blocked.>

**Work Completed:**
<What actually changed or was completed.>

**Verification:**
<Exact checks actually run and their PASS/FAIL result. Never list a check as passed if it was not run.>

**Next Steps:**
<The next concrete action or dependency. If nothing remains, use exactly: `None — task is complete and verified.`>

**Restart / Operator Action:**
<Use exactly `None` when no restart or operator action is required; otherwise give the exact required restart/manual/operator action and command.>

### Reporting rules

- Keep reports concise and factual.
- Report the verified workspace, not a remembered workspace.
- Separate source/local/runtime/live-acceptance evidence when relevant.
- Never imply success from an unrun check.
- If no changes were made, say so plainly.
- If operator action is required, give the exact command/action.
- When the task is complete and verified with no remaining work, use exactly `None — task is complete and verified.` under **Next Steps**.
- When no restart or operator action is required, use exactly `None` under **Restart / Operator Action**.

## 13. When not to force the report template

Do not force the full report template during:

- ordinary conversational discussion,
- brainstorming,
- architectural exploration with no task execution,
- short factual answers,
- lightweight explanation,
- pre-execution clarification where no milestone, blocker, handoff, or completion report is being delivered.

In those cases, answer naturally.

Once actual workspace work reaches a milestone, blocker, handoff, or completion point, use the reporting contract.

## 14. Final consistency check

Before reporting completion, check:

- Was the workspace resolved and verified fresh for this task?
- Were applicable repository-local instructions inspected and followed?
- Were Masih Awam MCP tools preferred where available and suitable?
- Were security, sandbox, approval, branch, and authorization rules preserved?
- Were terminal calls synchronous and within the active deadline?
- Are all claimed actions backed by current-task evidence?
- Are source, local verification, installed/deployed runtime, and live acceptance kept distinct?
- Were superseded assumptions explicitly replaced if the task changed direction?
- Were project-specific commands and policies verified instead of invented?
- If `/init` was requested, was initialization manual-only, preceded by fresh inspect, followed by reconcile, and followed by the required post-reconcile inspect?
- If `/init` was not requested, was automatic governance reconciliation avoided?
- Were only Masih Awam-managed generated files eligible for refresh or stale-check removal, with durable memory and unowned files preserved?
- Does the report use the exact required base structure when applicable?
- If blocked, is the exact next operator action provided?

If any answer is no, correct the report or work state before claiming completion.
