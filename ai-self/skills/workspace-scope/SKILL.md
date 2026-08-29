---
name: workspace-scope
description: Use before repository or filesystem mutation when Masih Awam MCP can access multiple projects or the user names, switches, or implies a target project. Resolve and lock the correct repository before writing, committing, pushing, or installing project-scoped skills.
license: MIT
---

# Workspace Scope Guard

Prevent cross-project contamination when the tool can access the user home or multiple repositories.

## Core rule

Never choose a write target from prior-chat memory, the last-used working directory, or a guessed project path. Resolve and verify the target project for the current task before the first mutation.

## Project resolution

Use this precedence when identifying the target project:

1. Explicit project path in the current user request.
2. Explicit project/repository name in the current user request, resolved read-only.
3. Unambiguous project identity established in the current conversation.
4. Read-only discovery of candidate repositories.

Native client memory and prior conversations may provide hints for discovery but never authorize a filesystem write target by themselves.

If more than one candidate remains plausible, do not write. Report the ambiguity or resolve it through read-only repository identity checks.

## Verify repository identity

Before mutation:

1. Resolve the canonical repository root with `git rev-parse --show-toplevel` when the project is a Git repository.
2. Canonicalize paths before boundary checks.
3. Read `ai-self/project.yaml` when present.
4. Verify the configured repository identity using stable evidence such as the canonical `origin` repository and required marker files.
5. Treat absolute paths as location hints, not durable identity, because repositories may move or be cloned elsewhere.

Normalize equivalent Git remote forms when comparing identity (for example SSH and HTTPS forms for the same `owner/repository`).

If the manifest identity conflicts with the actual repository, stop before writing and report the mismatch.

## Task-local workspace lock

After verification, establish one task-local `WORKSPACE_ROOT`. The lock is ephemeral and must not be carried blindly into another conversation or unrelated task.

While locked:

- run repository commands with `cwd` at `WORKSPACE_ROOT` or a verified descendant;
- write only inside `WORKSPACE_ROOT`;
- reject path traversal or symlink resolution that escapes the root;
- do not modify sibling repositories merely because they are reachable from `$HOME`;
- do not silently switch projects when a command fails or a file is missing.

Read-only discovery outside the lock is allowed only when needed to locate or identify the requested project and must not access secrets or unrelated personal data.

## Project switching

If the user explicitly switches to another project, discard the current lock and perform project resolution and verification again. Never reuse the previous project's root as a fallback.

For an explicit multi-repository task, lock and operate on each repository separately. Keep changes, validation, commits, and pushes isolated per repository. Never combine files from different repositories into one Git operation.

## Git write revalidation

Immediately before commit, push, branch creation that changes repository state, or project-scoped skill installation, re-check:

- current canonical Git top-level;
- verified project identity / expected origin when configured;
- current branch and repository status as relevant;
- task-owned paths are inside the locked root.

If any check differs from the established lock, stop. Do not auto-correct by changing directories to a remembered project.

## Failure behavior

Missing project, ambiguous identity, root mismatch, remote mismatch, or attempted path escape are hard blockers for mutation. Continue only with safe read-only diagnosis or after the target is unambiguously resolved.

The correct failure mode is `wrong/unknown workspace: no write performed`, never `best guess and continue`.
