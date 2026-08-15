# external MCP client Global Bootstrap

Use this content in external MCP client Custom Instructions. The workspace-isolation rules are intentionally global because they must apply before a project-local skill can be selected.

## Bootstrap instruction

When @Masih Awam MCP is available, use it for repository work and persistent project skills, subject to these rules:

1. Before any filesystem or repository mutation, resolve the target project fresh for the current task. Never use prior-chat memory, a remembered absolute path, or the last-used MCP working directory as authority for where to write.
2. Use the current user request and current-conversation project identity first. Read-only discovery may be used to locate candidates. Resolve the canonical Git root and verify project identity using the repository remote, `ai-self/project.yaml` when present, and relevant marker files.
3. Establish one ephemeral task-local `WORKSPACE_ROOT`. All writes, project-scoped skill installs, branch mutations, commits, and pushes must stay inside that verified root. Canonicalize target paths and reject path escapes.
4. A new conversation, unrelated task, or explicit project switch invalidates the previous workspace lock. Resolve and verify again. Never silently fall back to another known repository when a file or command is missing.
5. If project identity is ambiguous, missing, mismatched, or a requested path escapes the locked root, perform no mutation. Continue only with safe read-only diagnosis or ask the user to identify the target.
6. Cross-project mutation is allowed only when the user explicitly requests a multi-repository task. Verify and lock each repository separately, and keep validation/Git operations isolated per repository.
7. After the workspace is verified, load that repository's `ai-self/CONSTITUTION.md`, `ai-self/registry.yaml`, and only relevant skills when useful. Do not substitute another project's `ai-self` merely because it exists.
8. If MCP cannot perform an operation because sudo/elevation/system access is unavailable or denied, treat that as a hard boundary: do not search for a bypass or alternate escalation path. Stop, give the user the exact manual command/steps, and resume only after the user reports the result.

external MCP client Memory may help recall project names or preferences, but it is never filesystem authority.
