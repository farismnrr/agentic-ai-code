# Self-improvement — keep `.agents/` current

`.agents/` is the project's durable agent memory. **Reviewing it before finish is part of every task, regardless of which coding agent/client performs the work.**

## Before declaring a task finished

1. Review the task's diff/findings for anything another agent would need to know later.
2. Update an existing memory/knowledge file instead of creating a duplicate when possible.
3. Add any new memory to [`../memories/README.md`](../memories/README.md).
4. Update [`../plans/README.md`](../plans/README.md) when a plan starts, completes, becomes blocked, or changes status.
5. Remove or amend guidance that became false because of the task.
6. If nothing durable changed, explicitly acknowledge that conclusion rather than inventing a memory just to satisfy the process.
7. Run `bash scripts/check-agent-docs.sh` before finish.

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

## General enforcement

The repository intentionally avoids agent-client-specific hooks/settings. There is one shared entrypoint (`AGENTS.md`) and one shared durable guidance tree (`.agents/`).

[`../../scripts/check-agent-docs.sh`](../../scripts/check-agent-docs.sh) is the vendor-neutral structural backstop. CI runs it on every pull request. It currently verifies that:

- repository-owned vendor-specific agent entrypoints/settings are absent;
- shared guidance does not reintroduce vendor-specific agent lifecycle/discovery instructions;
- every Markdown memory is linked from `memories/README.md`;
- every Markdown plan/sub-plan is linked from `plans/README.md`.

The script deliberately does **not** try to infer whether source code "deserves" a memory update. That decision is semantic and belongs to the closeout review above. CI proves structural integrity; the working agent is responsible for capturing durable knowledge when the task creates it.

If a future agent client offers its own local automation, keep it personal/untracked. Do not fork repository guidance or add a second repository-owned lifecycle path.

## Principle

The self-improvement loop is: **read existing durable context → do the work → capture new durable context → keep indexes/statuses truthful → run the shared integrity gate**.
