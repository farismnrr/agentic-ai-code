# ai-self

Persistent self-improvement workspace for ChatGPT + Masih Awam MCP.

## Layout
- `CONSTITUTION.md` — behavioral and safety rules.
- `registry.yaml` — index of reusable skills and local tools.
- `skills/` — reusable procedural knowledge.
- `tools/` — local scripts/binaries callable through `terminal_exec`.
- `lessons/` — durable corrections and reusable lessons.
- `policies/` — autonomy and approval boundaries.

## Intended loop
1. Load the constitution and registry when a complex technical task benefits from persistent procedural context.
2. Load only relevant skills.
3. Perform the task using Masih Awam MCP.
4. Reflect after substantial work.
5. Persist only reusable improvements.
6. Validate new or changed tools/skills before relying on them.

This directory is not a custom RAG system and should not duplicate ChatGPT native Memory.
