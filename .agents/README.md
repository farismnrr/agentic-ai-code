# .agents

Everything an AI agent needs for this repo lives here. `EXTERNAL MCP CLIENT.md` and `AGENTS.md` at the repo root are pointers to this folder and nothing more — don't grow them, grow this.

## Read this first

**This is a Nuxt 4 + Nuxt UI 4 project. Do things the idiomatic Nuxt way — dependencies, config, code, file placement — and prefer the framework's own mechanism over a generic JS/Vue solution, even when the generic one is shorter or more familiar.** The details are in [`knowledge/nuxt-way.md`](knowledge/nuxt-way.md); read it before writing code or adding a dependency.

## What's where

| Folder | Contents | When to read it |
| --- | --- | --- |
| [`knowledge/`](knowledge/) | How this project works and the rules for changing it | Before any code change |
| [`skills/`](skills/) | Installed agent skills (`nuxt`, `nuxt-ui`) | Before framework or UI work |
| [`plans/`](plans/) | Implementation plans for in-flight work | When picking up a multi-step task |
| [`memories/`](memories/) | Durable decisions and context that outlive a session | At session start; append when you learn something lasting |
| [`hooks/`](hooks/) | Scripts wired into external MCP client Code via `.external-mcp/settings.json` | Rarely — when changing the reminder behavior |

**Before you finish a task, update this folder.** Read [`knowledge/self-improvement.md`](knowledge/self-improvement.md) for what belongs where. A `Stop` hook nudges you once per session if source changed and `.agents/` didn't.

### knowledge/

| File | Covers |
| --- | --- |
| [`nuxt-way.md`](knowledge/nuxt-way.md) | **The working agreement.** Dependency install rules, Nuxt-native mechanism table |
| [`self-improvement.md`](knowledge/self-improvement.md) | **Keeping this folder current.** What to record, where, and what not to record |
| [`project.md`](knowledge/project.md) | Stack, versions, directory layout, commands |
| [`conventions.md`](knowledge/conventions.md) | Coding rules — colors, icons, auto-imports, gotchas |
| [`tooling.md`](knowledge/tooling.md) | ESLint setup, environment variables, runtime config |
| [`resources.md`](knowledge/resources.md) | Skills and the Nuxt UI MCP server — what each is for |

## Conventions for this folder

- **Skills are the source of truth here.** `.external-mcp/skills/*` are symlinks into `.agents/skills/*`, so external MCP client Code auto-discovers them while the real files stay in one place. Other agents can point at `.agents/skills/` directly.
- `skills-lock.json` stays at the repo root — the `skills` CLI expects it there. Update with `npx skills update`.
- Plans and memories are Markdown, one file per topic, named in kebab-case.
- When you learn something durable — a decision, a constraint, a trap — write it to `memories/` rather than leaving it in the conversation.
