# .agents

Authoritative guidance for **any coding agent** working in this repository lives here. Root [`AGENTS.md`](../AGENTS.md) is the only repository agent entrypoint; keep repo-specific guidance centralized here instead of creating client/vendor-specific instruction files.

## Read this first

This is a **Nuxt 4 application plus a Rust native-tool workspace**. Use Nuxt-native mechanisms for web application work, and preserve the explicit Rust/MCP security boundaries for native execution work.

Before changing anything, read the files relevant to the task:

1. [`knowledge/project.md`](knowledge/project.md) — current stack, layout, and verification commands.
2. [`knowledge/git.md`](knowledge/git.md) — branch/PR rules; never commit directly to `main` or `dev`.
3. [`knowledge/nuxt-way.md`](knowledge/nuxt-way.md) — required approach for Nuxt/Vue work.
4. [`knowledge/conventions.md`](knowledge/conventions.md) — project conventions.
5. [`plans/README.md`](plans/README.md) — check whether the task belongs to an existing plan.
6. [`memories/README.md`](memories/README.md) — check for durable decisions or known traps before repeating old mistakes.

## What's where

| Folder | Contents | When to read it |
| --- | --- | --- |
| [`knowledge/`](knowledge/) | Stable project rules and operating knowledge | Before changing the relevant subsystem |
| [`skills/`](skills/) | Framework/UI/tool skills and package skill links | Before work covered by a skill |
| [`plans/`](plans/) | Multi-step implementation plans and their status | Before continuing planned work |
| [`memories/`](memories/) | Durable decisions, constraints, incidents, and traps | At task start and task closeout |
| [`contracts/`](contracts/) | Frozen client-visible contracts used by acceptance gates | Before changing a published contract |

### knowledge/

| File | Covers |
| --- | --- |
| [`nuxt-way.md`](knowledge/nuxt-way.md) | Nuxt-native dependency/config/code placement rules |
| [`self-improvement.md`](knowledge/self-improvement.md) | Mandatory agent closeout and durable-memory rules |
| [`project.md`](knowledge/project.md) | Current stack, repository layout, commands, runtime surfaces |
| [`conventions.md`](knowledge/conventions.md) | Coding and UI conventions |
| [`git.md`](knowledge/git.md) | Branching, commits, PRs, and release boundaries |
| [`tooling.md`](knowledge/tooling.md) | Environment/runtime config and lint tooling |
| [`resources.md`](knowledge/resources.md) | Installed skills, MCP resources, and Agentation |

## Agent closeout is mandatory

**Every agent must perform the closeout review in [`knowledge/self-improvement.md`](knowledge/self-improvement.md) before declaring a task finished.** The closeout keeps plans, memories, and knowledge aligned with the repository.

The repository deliberately has **no client/vendor-specific lifecycle hook**. Instead:

- shared instructions live in `AGENTS.md` + `.agents/`;
- structural integrity is enforced by [`../scripts/check-agent-docs.sh`](../scripts/check-agent-docs.sh) in CI;
- semantic closeout remains the responsibility of the agent doing the work, because a static check cannot decide whether a new implementation detail is durable knowledge.

Run the integrity gate before finish:

```sh
bash scripts/check-agent-docs.sh
```

## Conventions for this folder

- `.agents/` is the source of truth for shared agent guidance.
- Do **not** add repository-owned client/vendor agent directories, settings, discovery links, or alternate instruction entrypoints. `AGENTS.md` + `.agents/` must stay portable across coding agents.
- `skills-lock.json` remains at repo root because the `skills` CLI expects it there.
- Plans and memories are Markdown, one topic per file.
- New memory files must be added to [`memories/README.md`](memories/README.md) in the same change.
- New plans or status changes must be reflected in [`plans/README.md`](plans/README.md).
- Use kebab-case for new plan/memory filenames. Preserve oddly named historical files unless they can be renamed without breaking references.
- Delete or amend durable guidance when it stops being true; stale memory is worse than missing memory.
