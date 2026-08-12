# OpenClaw-Style Configured Root for Workspaces

Context: When implementing workspaces backed by real filesystem folders (Plan 010), we considered several models.

1. Unrestricted Filesystem Browser: Allow the user to browse anywhere on the server.
2. Configured Root: A single server-wide configured root directory (like OpenClaw), where users pick workspaces as subdirectories within this root.

Decision: We explicitly rejected the "browse anywhere" unrestricted filesystem picker in favor of the **OpenClaw-style configured root** (`NUXT_WORKSPACES_ROOT`).

Reasoning:
- **Security & Multi-Tenancy**: The app is architecturally a multi-tenant web server (Postgres-backed accounts, login). Allowing an authenticated web user to browse the unrestricted server filesystem is inherently dangerous. Local-only coding assistants do not solve multi-user filesystem access because they run under the local operator's OS permissions.
- **OpenClaw Precedent**: OpenClaw, the closest legitimate analog, handles this by using an operator-configured root (`agents.defaults.workspace`), enforcing that workspaces are real directories *within* that explicitly declared boundary. We adopted this exact same pattern.
- **Future Growth**: For now, it's a shared single root. If the app becomes a genuine multi-tenant SaaS, the model easily adapts to per-user roots (e.g. `agents.entries.*.workspace`-style).

A future agent tempted to add an unrestricted "browse anywhere" mode should heed this decision: it was explicitly rejected because it violates the security boundaries of a multi-user web architecture. Instead, configure roots at the deployment level.
