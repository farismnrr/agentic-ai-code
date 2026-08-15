# ai-self

Persistent self-improvement workspace for ChatGPT + Masih Awam MCP. It stores procedural knowledge and policy, not user-memory data or custom RAG.

## Authority

Read `CONSTITUTION.md` first when policy matters. In short: platform/user constraints outrank local policy; local policy outranks repository-local skills; reviewed first-party skills outrank community skills. Lower layers may specialize but never weaken higher-layer safety or approval boundaries.

## Layout

- `CONSTITUTION.md` — authority, behavioral rules, safety boundaries.
- `registry.yaml` — routing index for local/external skills and tool providers.
- `skills/` — reusable local procedural skills.
- `tools/` — narrowly scoped local helpers callable through MCP.
- `lessons/` — durable corrections and reusable lessons.
- `policies/` — explicit autonomy/approval boundaries.

## Runtime loop

1. For substantial technical work, load the constitution/registry when relevant.
2. Load only the skill(s) matching the current task.
3. Perform work through Masih Awam MCP or other explicitly available tools.
4. Respect tool boundaries as hard boundaries; never bypass missing sudo/elevation/system access.
5. Validate before relying on changes.
6. Reflect after substantial work and persist only reusable improvements.
7. Use `github-delivery` to commit/push task-owned completed work when policy allows.

## Primary local skills

- `workspace-scope` — resolve and lock the correct repository before mutation.
- `implementation-planning` — structured plan-only-by-default technical planning.
- `github-delivery` — safe task-owned commit/push completion.
- `skill-acquisition` — reviewed skill discovery, installation, update, and conflict control.

## Skill providers

- Prefer GitHub CLI native `gh skill` search/preview/install/update while available.
- Use Context7 as a secondary discovery/documentation provider.
- Externally installed skill directories are local dependencies and are ignored by Git; canonical sources are recorded in `registry.yaml`.

This directory intentionally does not duplicate ChatGPT native Memory.

## Project identity and workspace isolation

`BOOTSTRAP.md` contains the global ChatGPT Custom Instructions needed to enforce workspace isolation before any project-local skill is selected.

`project.yaml` identifies this repository using stable Git/project evidence. Before repository mutation, use `skills/workspace-scope/SKILL.md` to resolve and verify the requested project and establish an ephemeral task-local workspace lock.

The lock is deliberately not persisted as a global current-project value. A new conversation or project switch must resolve the target again. Prior chat memory may help locate a repository but never authorizes writes.
