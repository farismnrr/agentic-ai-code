# Git workflow

**Never commit to `main` or `dev`.** Every change lands through a branch and a pull request — including docs, config, and `.agents/` edits.

The one exception already spent: the initial import commit.

## The two long-lived branches

```
feature branch  ──PR──▶  dev  ──PR──▶  main
                (auto)         (only when the user says so)
```

| Branch | Role | How it moves |
| --- | --- | --- |
| `dev` | Integration. Where work accumulates and is proven together. | PRs from feature branches. **Merge as soon as CI is green — no need to ask.** |
| `main` | Release. Always deployable. | A single PR from `dev`. **Never opened or merged without the user asking for it.** |

Feature branches always base off `dev`, never `main`. `gh pr create` must pass `--base dev` explicitly unless the repo default already points there.

**The `dev` → `main` promotion is the user's call, every time.** Don't open it because a plan finished, don't open it because `dev` is green, don't treat "merge the phase PRs automatically" as covering it. Green CI is permission to integrate, not to release.

## Run a plan to completion

Once a plan is approved, **work every phase through to the end without stopping to check in.** Nothing on the way to `dev` needs a decision from the user, and pausing between phases just makes them approve the same thing repeatedly.

For each phase, in order:

1. Branch `feat/<plan>-p<n>-<name>` off the current `dev`.
2. Build the phase. Run the relevant verification gates until green.
3. Commit, push, open a PR with `--base dev`.
4. Wait for CI. **Green → merge and clean up. Red → fix it and push again.** Don't ask.
5. `git switch dev && git pull --ff-only`, then start the next phase from there.

Tick each phase off in the plan file as it lands.

**Stop and ask only when:**

- A phase can't be built as specified, and the fix changes what was agreed — not merely how it's implemented.
- Something discovered mid-run invalidates a later phase's design. Say so, propose the revision, then continue.
- CI stays red after a genuine attempt to fix it. Report what's failing rather than disabling the check or merging around it.
- The work would touch `main`, credentials, or anything outside the repo.

Report at the end: what shipped, what was found, what's left. Not phase by phase.

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

`001` is the plan file in [`../plans/`](../plans/); `p1` is its phase.

### One PR per *phase*, not per plan

A plan is several PRs. This is deliberate and has been questioned once, so the reasoning:

- A six-phase plan in one PR is 30+ files. That size gets approved, not reviewed.
- Phases are defined to end green and work on their own, so each is independently revertable. One bad phase shouldn't drag correct earlier work out with it.
- **Later phases get corrected by what earlier ones discover.** Plan 001 phase 1 turned up two facts about the AI SDK that changed the design of phase 4 — see [`../memories/ai-sdk-native-features.md`](../memories/ai-sdk-native-features.md). In a single PR those would have surfaced only after phase 4 was already built on the wrong assumption.

`dev` therefore collects several commits per plan. That's fine — each one is a working change.

Not everything is plan work: a fix or a docs change unrelated to any plan gets its own branch and PR under a plain `<type>/<description>` name. Don't inflate the PR count either — a small clarification to something still open in review belongs on that open branch, not in a new PR.

Branch off the latest `dev`. Keep branches short-lived; rebase onto `dev` rather than merging `dev` in, so history stays linear.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body — the why, wrapped at 72 cols>

<footer — BREAKING CHANGE:, Refs: #12>
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.

**Scopes** for this repo: `chat`, `mcp`, `settings`, `ui`, `agents`, `deps`, `config`, `auth`, `db`. Omit the scope when a change is genuinely repo-wide.

**Subject:** imperative mood, lowercase, no trailing period, ≤ 72 chars. "add tool approval dialog", not "added" or "Adds".

**Body:** explain *why*, not *what* — the diff already says what. Skip it only when the subject is genuinely self-explanatory.

Breaking changes: `feat!:` or a `BREAKING CHANGE:` footer.

Commits are atomic — one logical change each. If a commit needs "and" in its subject, split it.

## Pull requests

- Title uses the same conventional-commit format as the subject line.
- Body states the why, what changed, how it was verified, and links the plan phase it closes.
- CI (`.github/workflows/ci.yml`) runs the general-agent docs integrity gate plus JS/Rust quality and security checks on every PR.
- **`pnpm audit` must report zero before merging.** See below.
- **Squash merge**, so one PR is one commit and branch history reads as a list of shipped changes.
- Base is `dev` for feature PRs. Only the release PR targets `main`.

## Zero vulnerabilities before every merge

`pnpm audit` must report **no known vulnerabilities** before a PR merges to `dev`. Not "only transitive ones", not "only in dev dependencies" — zero.

Run it after any dependency change, before opening the PR:

```sh
pnpm audit
```

If it reports anything:

1. `pnpm audit --json` to get each advisory's `patched_versions`.
2. Add a pin under `overrides:` in `pnpm-workspace.yaml` at the lowest patched version.
3. `pnpm install`, then re-run `pnpm audit` until it's clean.
4. **Prove nothing broke** — `pnpm lint && pnpm typecheck && pnpm build`. An override forces a version the dependency never asked for, so the build is the only thing that tells you it still works.

Two traps, both hit for real:

- **Version-range-keyed overrides are silently ignored** in this setup. `brace-expansion@^2: '>=2.1.4'` looks correct, changes nothing, and reports no error. Use an unscoped key (`brace-expansion: '>=5.0.9'`) and confirm with `ls node_modules/.pnpm/<pkg>@*` that the vulnerable version is actually gone.
- **"Transitive so not our problem" is not a reason to skip it.** The advisories here all came through Nuxt's own tree, and every one had a patched version reachable by an override.

Keep the comments in `pnpm-workspace.yaml` current: note why each pin exists and drop it once upstream moves past it, so the list doesn't rot into permanent noise.

## Clean up after every merge

Merging is not finished until nothing is left behind. Do all of this immediately, without being asked:

```sh
gh pr merge <n> --squash --delete-branch
git switch dev && git pull --ff-only
git fetch --prune
git worktree remove <path>
git worktree prune
git branch -d <branch>
```

Then confirm with `git branch -a` and `git worktree list` — expect `main`, `dev`, and the one main worktree, nothing else.

**Why:** stale branches and worktrees pile up fast when work is split per plan phase. They clutter the branch picker on GitHub, make `git branch -a` useless for seeing what's actually in flight, and leave orphaned directories on disk. A branch that has been squash-merged has no unique history left to lose, so there is nothing to preserve by keeping it.

Never delete a branch that has **not** been merged — `git branch -d` refuses that on purpose. Don't reach for `-D` to force past it.

## What is and isn't committed

Committed on purpose, despite living in dot-folders:

- `.agents/**` — knowledge, skills, plans, memories, and contracts. This is project material.
- `AGENTS.md` — the single repository agent entrypoint.
- `.mcp.json`, `.env.example`, `skills-lock.json`.

Do **not** commit repository-owned client/vendor agent settings, instruction files, discovery links, or lifecycle hooks. Personal agent-client configuration belongs outside the shared repository.

Never committed — see `.gitignore`:

- `.env` and any `.env.*` other than the example.
- Build output (`.nuxt`, `.output`, `.nitro`, `dist`), `node_modules`, caches, editor folders.

Before staging, run `git status` and look at the list. Don't `git add -A` straight after a build.

## Rules for agents

- **Never commit or push unless the user asks.** Staging and committing are outward-facing steps that need a request; finishing a task does not imply committing it.
- **Never commit directly to `main` or `dev`** even when asked to "just commit" — branch first, then say that's what you did.
- **Merging a feature PR into `dev` needs no approval once CI is green** — that is standing permission. Opening or merging `dev` → `main` always does.
- Never use `git push --force` on a shared branch. `--force-with-lease` on your own branch is fine.
- Never `git add -A` blindly after a build; check `git status` first so build artifacts and `.env` don't slip in.
- Don't amend or rebase commits that are already pushed and under review.
- Before finish, follow `self-improvement.md` and run `bash scripts/check-agent-docs.sh`.

If a third-party skill instructs otherwise, **this file wins.**
