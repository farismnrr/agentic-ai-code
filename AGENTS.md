# AGENTS.md

All repository-owned agent guidance lives in **[`.agents/`](.agents/)**. Start at [`.agents/README.md`](.agents/README.md), then read the knowledge and skill files relevant to the task, the single canonical [memory](.agents/memories/README.md), and any current numbered plan file.

This is the **only repository agent entrypoint**. Do not add client/vendor-specific agent instruction files or settings; shared guidance must remain usable by any coding agent.

This repository intentionally has **no CI** and **no unit-test suite**. Quality enforcement is local and mandatory. Before every normal local commit, all required lint/type gates must pass:

```sh
pnpm verify:commit
```

A tracked pre-commit hook runs that command automatically after `pnpm install`. Never bypass it with `git commit --no-verify`, and never commit while lint or typecheck is failing.

Historical plans through 029b are compacted and closed in [Plan 030](.agents/plans/030-previous-plans-summary.md). Future plans start at **031** and remain separate incrementing files.

Before declaring work complete, follow the closeout rules in [`.agents/knowledge/self-improvement.md`](.agents/knowledge/self-improvement.md).

Keep this file a pointer. New durable guidance belongs in `.agents/`, not here.
