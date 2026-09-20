# .agents

Authoritative guidance for **any coding agent** working in this repository lives here. Root [`AGENTS.md`](../AGENTS.md) is the only repository agent entrypoint; keep repo-specific guidance centralized here instead of creating client/vendor-specific instruction files.

## Read this first

This is a **Nuxt 4 application plus a Rust native-tool workspace**. Use Nuxt-native mechanisms for web application work, and preserve the explicit Rust/MCP security boundaries for native execution work.

**Workspace-root invariant:** `$HOME/Documents/Projects` is the canonical relay authorization/execution root. The `ai-code` repository checkout is source code only: agents may select it as `cwd` while editing/testing this repository, but must never use it as the root for user workspaces, Creative Projects, Blender production, or acceptance artifacts. Blender creative projects use `$HOME/Documents/Projects/Blender/<creative-project>/...`; any project-internal `blender/` layout lives beneath that creative-project directory.

Before changing anything, read the files relevant to the task:

1. [`knowledge/project.md`](knowledge/project.md) — current stack, layout, and verification commands.
2. [`knowledge/git.md`](knowledge/git.md) — branch/PR/commit rules; never commit directly to `main` or `dev`.
3. [`knowledge/nuxt-way.md`](knowledge/nuxt-way.md) — required approach for Nuxt/Vue work.
4. [`knowledge/conventions.md`](knowledge/conventions.md) — project conventions.
5. [`memories/README.md`](memories/README.md) — the **single canonical durable memory**; read it before repeating old mistakes.
6. The relevant numbered file under [`plans/`](plans/) when the task belongs to a current multi-step plan. [`plans/030-previous-plans-summary.md`](plans/030-previous-plans-summary.md) is historical only.

## What's where

| Path | Contents | When to read it |
| --- | --- | --- |
| [`knowledge/`](knowledge/) | Stable project rules and operating knowledge | Before changing the relevant subsystem |
| [`skills/`](skills/) | Framework/UI/tool skills and package skill links | Before work covered by a skill |
| [`memories/README.md`](memories/README.md) | All durable decisions, constraints, incidents, and traps | At task start and closeout |
| [`plans/`](plans/) | Plan 030 historical snapshot plus future incrementing plan files | Before continuing planned work |
| [`contracts/`](contracts/) | Frozen contract/evidence snapshots used by acceptance gates; only entries explicitly marked current remain live contract guidance | Before changing a published contract or interpreting historical acceptance evidence |

### knowledge/

| File | Covers |
| --- | --- |
| [`nuxt-way.md`](knowledge/nuxt-way.md) | Nuxt-native dependency/config/code placement rules |
| [`self-improvement.md`](knowledge/self-improvement.md) | Mandatory closeout, canonical-memory, and plan-maintenance rules |
| [`project.md`](knowledge/project.md) | Current stack, repository layout, commands, runtime surfaces |
| [`conventions.md`](knowledge/conventions.md) | Coding and UI conventions |
| [`git.md`](knowledge/git.md) | Branching, commits, PRs, and local commit gates |
| [`tooling.md`](knowledge/tooling.md) | Environment/runtime config, lint/typecheck, and local hook tooling |
| [`resources.md`](knowledge/resources.md) | Installed skills, MCP resources, and Agentation |

### Contract snapshot rule

Files under `contracts/` may be current frozen contracts or dated Phase-0/baseline/evidence snapshots. A historical date, `Phase 0`, `baseline`, or explicit `HISTORICAL / SUPERSEDED` marker means the file records that checkpoint and does not override current source/tests, current operator docs, current knowledge/memory, or a later explicit contract. Do not revive a removed tool/protocol simply because it appears in an older contract snapshot.

## Memory model

The repository deliberately keeps **one durable memory file**: [`memories/README.md`](memories/README.md). All memory notes that existed before 2026-08-12 were compacted into it.

- Do not add new sibling `memories/*.md` files.
- Amend the canonical file in place when a durable decision/trap changes.
- Delete or shorten stale memory instead of growing a second copy.
- Git history is the place to recover the pre-compaction long-form memory notes.

## Plan model

[`plans/030-previous-plans-summary.md`](plans/030-previous-plans-summary.md) is a **one-time historical compaction** of every plan that existed through Plan 029b. The user explicitly closed those plans for a planning-data refresh.

Future planning remains normal and incremental:

- next unused numeric plan after the closed Plan 069 line is **070**;
- filename: `NNN-kebab-case.md`;
- never reuse a number;
- keep each new plan as its own file, including after completion;
- **do not automatically compact Plan 031+ into Plan 030**;
- there is no `plans/README.md` index; the numbered plan files are the source of truth for their own status.

An old unchecked item inside the Plan 030 history is not active work. Re-audit current source/external state and create a fresh numbered plan when needed.

**Supersession rule for Plan 031+:** numbered plan files preserve chronological implementation history, so older sections may truthfully describe a runtime or contract that was later replaced. Never treat an older dated checkpoint, unchecked historical item, historical tool count, or superseded design paragraph as current merely because it remains in the file. Resolve ambiguity in this order: current source/config and tests -> the plan's top-level current `Status`/explicit supersession note -> the latest dated closure/update for the relevant concern -> canonical memory/current knowledge. Historical checkpoints remain evidence only. When a plan changes direction mid-stream, update its top status and add an explicit supersession note rather than silently rewriting the old evidence.

**Active-plan rule:** many older Plan 031+ files predate the top-level `Status:` convention. Absence of a `Status:` line is **not** evidence that a plan is active, open, or safe to resume; unchecked boxes in such files are historical planning data until current source/external state is re-audited. Before resuming one of those plans, add an explicit current status/supersession note to that plan. Only a plan that explicitly identifies itself as current/in-progress (or is explicitly named by the user for re-audit) may drive new implementation work.

## Local quality policy

The repository intentionally has **no CI workflow**. Tests are normal repository tests rather than plan-specific verification scripts: web tests live under top-level `test/`, while Rust follows Cargo's package-local `tests/` convention.

After `pnpm install`, [`../scripts/install-git-hooks.sh`](../scripts/install-git-hooks.sh) configures Git to use [`.githooks/pre-commit`](../.githooks/pre-commit) and the main-only full [`.githooks/pre-push`](../.githooks/pre-push). Every checkpoint commit must pass:

```sh
pnpm guardrail:fast
```

The fast `pnpm guardrail:fast` checks repository policy, agent-doc integrity, architecture, and test layout, then auto-selects lint/typecheck for touched stacks. Closure uses `pnpm guardrail:full`; release preparation uses `pnpm guardrail:release`. Explicit service gates are available as `pnpm guardrail:nuxt` and `pnpm guardrail:rust`; use `pnpm guardrail:all` only for a deliberate cross-stack contract change. Do not make a Nuxt-only change pay for Rust validation, or a Rust-only change pay for Nuxt validation.

`scripts/` is reserved for guardrails and hook installation. Future plans must add feature-named tests under `test/` or Cargo `tests/`, not `verify-NNN`, `phase-NNN`, or other plan-numbered validation scripts. Historical plan references to removed scripts remain historical evidence only.

Do not use `git commit --no-verify`, disable `core.hooksPath`, or commit through another path merely to avoid a failing applicable gate.

See the current durable policy in [`memories/README.md`](memories/README.md#repository-policy-and-verification).

## Agent closeout is mandatory

**Every agent must perform the closeout review in [`knowledge/self-improvement.md`](knowledge/self-improvement.md) before declaring a task finished.** Keep code, knowledge, the canonical memory, and any current plan file aligned.

The repository deliberately has **no client/vendor-specific lifecycle hook**. Shared instructions live in `AGENTS.md` + `.agents/`; the tracked Git pre-commit hook is repository quality enforcement, not an agent-client lifecycle integration.

## Conventions for this folder

- `.agents/` is the source of truth for shared agent guidance.
- Do **not** add repository-owned client/vendor agent directories, settings, discovery links, or alternate instruction entrypoints.
- `skills-lock.json` remains at repo root because the `skills` CLI expects it there.
- Durable memory stays in exactly one file: `memories/README.md`.
- Plan 030 is the historical compaction snapshot; Plans 031+ remain separate incrementing files. Do not reuse an existing number; after Plan 069, the next unused numeric plan is 070 unless a previously reserved number is proven unused by the repository.
- Delete or amend durable guidance when it stops being true; stale memory is worse than missing memory.
