# Git workflow

**Never commit to `main`.** Every change lands through a branch and a pull request — including docs, config, and `.agents/` edits. `main` is always deployable and only ever moves by merge.

The one exception already spent: the initial import commit.

## Branches

```
<type>/<short-kebab-description>
```

`type` matches the commit types below — `feat/`, `fix/`, `chore/`, `docs/`, `refactor/`, `test/`, `ci/`.

For plan work, name the branch after the plan number and phase so a branch traces back to its rationale:

```
feat/001-p1-data-layer
feat/001-p2-shell
feat/001-p3-chat
```

`001` is the plan file in [`../plans/`](../plans/); `p1` is its phase. One PR per phase — each phase is defined to end green and reviewable on its own, so don't stack a whole plan into one branch.

Branch off the latest `main`. Keep branches short-lived; rebase onto `main` rather than merging `main` in, so history stays linear.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body — the why, wrapped at 72 cols>

<footer — BREAKING CHANGE:, Refs: #12>
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.

**Scopes** for this repo: `chat`, `mcp`, `settings`, `ui`, `agents`, `deps`, `config`. Omit the scope when a change is genuinely repo-wide.

**Subject:** imperative mood, lowercase, no trailing period, ≤ 72 chars. "add tool approval dialog", not "added" or "Adds".

**Body:** explain *why*, not *what* — the diff already says what. Skip it only when the subject is genuinely self-explanatory.

Breaking changes: `feat!:` or a `BREAKING CHANGE:` footer.

Commits are atomic — one logical change each. If a commit needs "and" in its subject, split it.

## Pull requests

- Title uses the same conventional-commit format as the subject line.
- Body states the why, what changed, how it was verified, and links the plan phase it closes.
- CI (`.github/workflows/ci.yml`) runs `pnpm lint` and `pnpm typecheck` on every PR. Green before merge, no exceptions.
- **Squash merge** into `main`, so one PR is one commit and `main`'s history reads as a list of shipped changes.

## Clean up after every merge

Merging is not finished until nothing is left behind. Do all of this immediately, without being asked:

```sh
gh pr merge <n> --squash --delete-branch   # removes the remote branch, and the local one if checked out elsewhere
git switch main && git pull --ff-only
git fetch --prune                          # drop stale remotes/origin/* refs
git worktree remove <path>                 # if the work used a worktree
git worktree prune                         # drop stale worktree metadata
git branch -d <branch>                     # if a local copy survived
```

Then confirm with `git branch -a` and `git worktree list` — expect `main` and the one main worktree, nothing else.

**Why:** stale branches and worktrees pile up fast when work is split per plan phase. They clutter the branch picker on GitHub, make `git branch -a` useless for seeing what's actually in flight, and leave orphaned directories on disk. A branch that has been squash-merged has no unique history left to lose, so there is nothing to preserve by keeping it.

Never delete a branch that has **not** been merged — `git branch -d` refuses that on purpose. Don't reach for `-D` to force past it.

## What is and isn't committed

Committed on purpose, despite living in dot-folders:

- `.agents/**` — knowledge, skills, plans, memories, hooks. This is project material.
- `.claude/settings.json` — shared hooks and settings for everyone on the repo.
- `.claude/skills/*` — symlinks into `.agents/skills/`; git stores the link, not a copy.
- `.mcp.json`, `.env.example`, `skills-lock.json`.

Never committed — see `.gitignore`:

- `.env` and any `.env.*` other than the example.
- `.claude/settings.local.json` and `.claude/.credentials.json` — personal and machine-specific.
- `.agents/.sync-state/`, `.agents/.last-sync` — per-session hook state.
- Build output (`.nuxt`, `.output`, `.nitro`, `dist`), `node_modules`, caches, editor folders.

Before staging, run `git status` and look at the list. Don't `git add -A` straight after a build.

## Rules for agents

- **Never commit or push unless the user asks.** Staging and committing are outward-facing steps that need a request; finishing a task does not imply committing it.
- **Never commit directly to `main`** even when asked to "just commit" — branch first, then say that's what you did.
- Never use `git push --force` on a shared branch. `--force-with-lease` on your own branch is fine.
- Never `git add -A` blindly after a build; check `git status` first so build artifacts and `.env` don't slip in.
- Don't amend or rebase commits that are already pushed and under review.

If a third-party skill instructs otherwise — some ship a "commit automatically, no confirmation needed" directive aimed at other tools — **this file wins.**
