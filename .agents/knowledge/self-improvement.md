# Self-improvement — keep `.agents/` current

`.agents/` is the project's durable agent memory. **Reviewing it before finish is part of every task.** The policy applies to external MCP client Code, Gemini/Antigravity, and any other agent working in this repository even when its client has no automatic reminder hook.

## Before declaring a task finished

1. Review the task's diff/findings for anything another agent would need to know later.
2. Update an existing memory/knowledge file instead of creating a duplicate when possible.
3. Add any new memory to [`../memories/README.md`](../memories/README.md).
4. Update [`../plans/README.md`](../plans/README.md) when a plan starts, completes, becomes blocked, or changes status.
5. Remove or amend guidance that became false because of the task.
6. If nothing durable changed, explicitly acknowledge that conclusion rather than inventing a memory just to satisfy the process.

A task is not documentation-complete when its implementation and `.agents/` tell different stories.

## When to write

| Trigger | Goes to |
| --- | --- |
| A decision someone could reasonably reverse without knowing why | `memories/<topic>.md` |
| A trap, dead end, incident, or fix that looks right but is wrong | `memories/<topic>.md` |
| The user establishes a durable repo-specific working constraint | the relevant `knowledge/` file or a memory if it is decision context |
| A stable command, convention, or operating rule changes | matching file in `knowledge/` |
| Repository architecture changes enough that the project map is misleading | `knowledge/project.md` |
| Tool/skill/MCP discovery changes | `knowledge/resources.md` |
| Multi-step work needs a durable handoff | `plans/<effort>.md` and `plans/README.md` |

## What not to write

- A second copy of facts already obvious from code/config when a link to the source is enough.
- A chronological changelog or narration of the session.
- Temporary debugging state, credentials, tokens, private URLs, or copied sensitive output.
- Speculation presented as a durable fact.
- A "completed" status that has not met the plan's own acceptance definition.

Durable docs may summarize implementation facts when they are needed to orient future agents, but point to the authoritative code/config rather than pretending Markdown is the runtime source of truth.

## How to write

- One topic per file; use kebab-case for new files.
- Start with the decision/constraint, then capture the reasoning and the tempting wrong alternative.
- Prefer amending an existing file over adding a near-duplicate.
- Keep indexes complete in the same change.
- Delete or clearly supersede stale memories. Stale memory is worse than missing memory.
- Preserve historical plan evidence, but mark historical snapshots as such when their checklists no longer describe current status.

## Automatic reminder coverage

### external MCP client Code

`.external-mcp/settings.json` registers a `Stop` hook that runs [`../hooks/check-agents-sync.sh`](../hooks/check-agents-sync.sh). The hook is a **best-effort reminder only**.

Current script behavior:

- watches source/config paths such as `app/`, `server/`, `shared/`, `modules/`, `plugins/`, `nuxt.config.ts`, `package.json`, `eslint.config.mjs`, and `.mcp.json`;
- compares watched files against `.agents/.last-sync`;
- blocks at most once per session by recording session state under `.agents/.sync-state/`;
- tells the agent to update `.agents/` or explicitly acknowledge that nothing durable was learned.

To acknowledge that no durable agent-memory update is needed:

```sh
touch .agents/.last-sync
```

### Important limitations of the current hook

Do **not** describe the hook as proving that "source changed but `.agents/` did not." The implementation does not compare source modification times with `.agents/` modification times; it only checks whether watched source is newer than `.agents/.last-sync`.

Consequences:

- editing a file under `.agents/` does **not** by itself move `.agents/.last-sync`;
- a missing `.last-sync` marker is created when the hook runs, so a fresh clone/session can miss changes made before that first marker creation;
- the hook depends on its shell environment/tools and is not a cross-agent repository service;
- hook silence is not evidence that documentation is synchronized.

These are implementation limitations, not documentation semantics. Fixing them requires changing the hook/config and is outside a docs-only task.

### Gemini/Antigravity and other agents

There is no repository-level equivalent Stop hook for these clients today. `GEMINI.md` imports the shared guidance and `AGENTS.md` points to it, so the closeout protocol above is mandatory **by instruction**, not automatically enforced by this repository.

If a client later gains an equivalent hook, keep the behavior centralized around this same closeout policy rather than forking a second set of rules.

## Principle

The reminder exists to catch forgetting. The real self-improvement loop is: **read existing durable context → do the work → capture new durable context → keep indexes/statuses truthful**.
