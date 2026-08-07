# Self-improvement — keep `.agents/` current

`.agents/` is the project's memory. Anything you learn that stays in the conversation is lost at the end of the session; anything you write here survives. **Updating `.agents/` is part of finishing a task, not an optional extra.**

## When to write

Write something down when any of these happen:

| Trigger | Goes to |
| --- | --- |
| You made a decision someone could reasonably reverse without knowing why | `memories/<topic>.md` |
| You hit a trap, dead end, or a "fix" that looks right but is wrong | `memories/<topic>.md` |
| The user corrected you, or stated a preference for how work should be done | `memories/<topic>.md` |
| A new command, convention, or rule for building this project | the matching file in `knowledge/` |
| A new dependency, module, or tool changed how the project is set up | `knowledge/project.md` + `knowledge/tooling.md` |
| Multi-step work that won't finish this session | `plans/<effort>.md` |

## What NOT to write

- Anything already derivable from the code, config, or `package.json`. If reading the repo answers it, don't duplicate it — duplicates drift and then mislead.
- Narration of what you did. Record the **why** and the **constraint**, not the changelog.
- One-off details that only mattered inside a single conversation.

## How to write

- One file per fact, kebab-case, first line a one-sentence summary.
- State the reasoning, and name the wrong-looking-right alternative if there is one — that's the part that actually prevents a repeat.
- Add the file to the index in `memories/README.md`.
- **Prefer updating an existing file over adding a near-duplicate.** Check the index first.
- **Delete memories that stop being true.** A stale memory is worse than no memory.

## The reminder hook

`.claude/settings.json` registers a `Stop` hook running [`../hooks/check-agents-sync.sh`](../hooks/check-agents-sync.sh). If watched source paths (`app/`, `server/`, `nuxt.config.ts`, `package.json`, …) changed but nothing under `.agents/` did, it interrupts once with a prompt to persist what you learned.

It fires **at most once per session** — it cannot loop, and it will not nag.

To acknowledge that nothing is worth saving:

```sh
touch .agents/.last-sync
```

Editing any file under `.agents/` clears it naturally. Session state lives in `.agents/.sync-state/` (gitignored) — safe to delete anytime.

The hook is a backstop for forgetting, not the reason to do this. Write things down as you learn them.
