---
name: github-delivery
description: Automatically finish substantial successful repository work by safely staging only task-owned changes, creating a conventional commit, and pushing the current branch. Use after implementation, fixes, refactors, docs, tooling, or skill changes unless the user explicitly says not to commit or push.
---

# GitHub Delivery

Use this as the default completion workflow for substantial repository changes.

## Goal

Do not stop at "files changed" when the work is complete. When validation succeeds, proactively create a clean commit and push the current branch so the completed work is durable and available remotely.

## Required workflow

1. Inspect repository state before staging:
   - `git status --porcelain=v1 --branch`
   - `git diff -- <task-owned paths>`
   - inspect recent commit style with `git log -5 --oneline` when useful.
2. Identify exactly which files belong to the current task.
   - Never absorb unrelated or pre-existing user changes merely to get a clean working tree.
   - When the working tree is mixed, use explicit pathspecs with `git add -- <paths...>`.
3. Validate the task before commit using the narrowest relevant checks.
   - Do not invent expensive test suites when the change only needs syntax/config/content validation.
   - If required validation fails, fix it first or report the blocker; do not push known-broken work.
4. Stage only task-owned files.
5. Review staged content:
   - `git diff --cached --check`
   - `git diff --cached --stat`
   - `git diff --cached`
6. Create one or more logical commits using the repository's existing convention. If none is evident, use Conventional Commits.
7. Push proactively when all of the following are true:
   - the user did not say not to push;
   - a remote exists;
   - HEAD is on a normal branch, not detached;
   - validation passed;
   - the staged/committed content contains no secrets or unrelated changes.
8. Push only the current branch:
   - if upstream exists: `git push`;
   - otherwise: `git push --set-upstream origin HEAD` when `origin` exists.
9. Verify the push result and report the commit hash and branch.

## Safety rules

- Never `git add .`, `git add -A`, or equivalent when unrelated changes are present.
- Never commit `.env`, credentials, private keys, tokens, or secret material.
- Never use `--force`, `--force-with-lease`, destructive reset, or history rewriting without explicit user approval.
- Never use `--no-verify` unless the user explicitly requests it.
- Do not amend an existing commit unless explicitly requested; make a new commit instead.
- Never push directly to a protected/default branch if repository policy forbids it.
- Do not merge or create releases merely because a push succeeded.
- If authentication or remote policy rejects the push, keep the local commit and report the exact blocker.

## Related installed skills

When useful, also inspect:

- `.agents/skills/git-commit/SKILL.md` for commit-message and staging guidance.
- `.agents/skills/make-repo-contribution/SKILL.md` for repository contribution-policy checks.
