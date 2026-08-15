# Skills

Local skills are reusable procedures that ChatGPT can load through Masih Awam MCP and expose through `.agents/skills/` when useful.

## Authoring rules

Each skill must have a `SKILL.md` with Agent Skills-compatible YAML frontmatter:

```yaml
---
name: lowercase-hyphen-name
description: Use when <specific trigger conditions>.
license: MIT
---
```

Keep `description` focused on **when to activate**, not a summary of every implementation step. Put procedural detail in the body or `references/`.

A skill should define:
- trigger/scope;
- procedure and expected outcome;
- required tools/capabilities;
- safety and approval boundaries;
- validation/success criteria;
- handoffs to other skills when needed.

## Quality rules

- Prefer updating an existing skill over creating a near-duplicate.
- One capability should have one clear primary skill.
- Ground technical procedures in verified workspace/tool behavior.
- Never let a skill override `../CONSTITUTION.md` or `../policies/default.yaml`.
- Validate frontmatter/structure and run a recall/overlap check after material changes.
- Keep large references and reusable scripts outside `SKILL.md` when that improves progressive loading.
