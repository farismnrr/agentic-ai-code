# AGENTS.md

All repository-owned agent guidance lives in **[`.agents/`](.agents/)**. Start at [`.agents/README.md`](.agents/README.md), then read the knowledge, skill, plan, and memory files relevant to the task.

This is the **only repository agent entrypoint**. Do not add client/vendor-specific agent instruction files or settings; shared guidance must remain usable by any coding agent.

Before declaring work complete, follow the closeout rules in [`.agents/knowledge/self-improvement.md`](.agents/knowledge/self-improvement.md) and run:

```sh
bash scripts/check-agent-docs.sh
```

Keep this file a pointer. New durable guidance belongs in `.agents/`, not here.
