# .agents

Authoritative agent guidance for this repository lives here. Root `AGENTS.md`, `EXTERNAL MCP CLIENT.md`, and `GEMINI.md` are entrypoints into this folder; keep repo-specific guidance centralized here instead of copying it into per-agent files.

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
| [`hooks/`](hooks/) | external MCP client Code reminder hooks | Only when investigating agent closeout automation |

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

**Every agent must perform the closeout review in [`knowledge/self-improvement.md`](knowledge/self-improvement.md) before declaring a task finished.** This is the mechanism that keeps plans, memories, and knowledge from drifting away from the repository.

Do not rely on an automatic hook as proof that `.agents/` is current:

- **external MCP client Code:** `.external-mcp/settings.json` wires a `Stop` reminder to `.agents/hooks/check-agents-sync.sh`. It is a best-effort backstop, not the source of truth.
- **Gemini/Antigravity:** `GEMINI.md` imports this index and the core knowledge files, but there is no equivalent repository Stop hook.
- **Other agents:** `AGENTS.md` points here; closeout is instruction-driven unless the client supplies its own automation.

The current hook's exact behavior and limitations are documented in [`knowledge/self-improvement.md`](knowledge/self-improvement.md). If the hook and this documentation disagree, treat the script as current behavior and fix the documentation or hook deliberately; never claim cross-agent automation that does not exist.

## Conventions for this folder

- `.agents/` is the source of truth for shared agent guidance.
- `.external-mcp/skills/*` are external MCP client-discovery symlinks; do not duplicate the underlying skill content there.
- `skills-lock.json` remains at repo root because the `skills` CLI expects it there.
- Plans and memories are Markdown, one topic per file.
- New memory files must be added to [`memories/README.md`](memories/README.md) in the same change.
- New plans or status changes must be reflected in [`plans/README.md`](plans/README.md).
- Use kebab-case for new plan/memory filenames. Preserve oddly named historical files unless they can be renamed without breaking references.
- Delete or amend durable guidance when it stops being true; stale memory is worse than missing memory.
