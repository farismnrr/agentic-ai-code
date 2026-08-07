# GEMINI.md

All agent guidance for this repo lives in **`.agents/`**. Antigravity doesn't proactively read `AGENTS.md`'s pointer the way external MCP client Code does — a plain link is inert text to it — so the load-bearing content is pulled in directly below via `@import`, which resolves at context-load time instead of waiting for you to decide to go read it.

@.agents/README.md

@.agents/knowledge/nuxt-way.md

@.agents/knowledge/git.md

@.agents/knowledge/conventions.md

Everything above is authoritative for this session, not background reading. In particular: **never commit directly to `main` or `dev`** (see the git rules above) and **use the Nuxt-native mechanism, not a generic JS/Vue one**, unless nothing built in covers the case.

`.agents/plans/`, `.agents/memories/`, and the rest of `.agents/knowledge/` aren't imported here (they're long and change often — importing them would go stale fast). Use your Read tool to open them when a task actually calls for it: plans before multi-step work, memories before repeating a mistake someone already made, `project.md`/`tooling.md` for commands and layout.

Keep this file a pointer plus imports — don't paste new guidance here. New guidance belongs in `.agents/`, same as `EXTERNAL MCP CLIENT.md`.
