---
name: github-delivery
description: Use when substantial repository work is complete and task-owned changes should be validated, committed, and pushed unless the user explicitly opts out.
license: MIT
---

# GitHub Delivery

Before any staging, branch mutation, commit, or push, apply `workspace-scope` and verify the current repository is the task's locked project. Revalidate the lock immediately before Git writes.

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

## Team repository mode

Treat the repository as team/shared when the user says it is a work/team repository or when repository contribution metadata indicates a collaborative workflow (for example CONTRIBUTING guidance, CODEOWNERS, PR templates, protected/shared branch conventions, or organization-managed rules).

In team/shared repositories:

1. Respect repository guidance and server-side GitHub rules as the source of truth for branch naming, required checks, reviews, CODEOWNERS, and merge policy.
2. Before creating a task branch from a shared base, fetch the relevant remote state first when network access is available. Do not branch from a knowingly stale local base.
3. Never overwrite, reset, checkout over, or absorb unrelated teammate/user changes to make branching easier. If the worktree is mixed and a safe branch transition is not possible, keep the current state intact and report the blocker.
4. Do not commit directly to the default branch or another shared/protected integration branch. If work starts there and changes are not yet committed, create an appropriate task branch when it is safe to do so.
5. Auto-commit and auto-push are allowed only on the current task branch and only for task-owned changes after validation.
6. Do not force push or rewrite published history. If remote history diverged, fetch and reconcile safely; stop for user input before any history-rewriting strategy.
7. After pushing, inspect whether a PR already exists for the branch when tooling allows. Update the existing task branch rather than opening duplicate PRs.
8. Creating a new PR is an external collaboration action: prepare the title/body automatically, but require user approval before opening it unless the user has explicitly enabled auto-PR for that repository/workflow.
9. Never self-approve, bypass required reviews/checks, dismiss review feedback, merge the PR, delete shared branches, create a release, or deploy merely because CI passes. Those remain explicit-user or repository-automation actions.
10. When a PR exists, preserve reviewer context: new commits should be pushed to the same task branch unless repository policy requires a new branch/PR.

### Branch defaults when repository guidance is absent

- New feature: `feat/<short-topic>`
- Bug fix: `fix/<short-topic>`
- Documentation: `docs/<short-topic>`
- Maintenance/tooling: `chore/<short-topic>`

Use repository-specific naming when it exists instead of these defaults.

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
