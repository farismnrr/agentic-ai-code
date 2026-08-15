# Skills, MCP, and agent resources

Use this page as the discoverability index for agent-facing skills and external context sources. The file layout is the source of truth; update this index when skills are added, removed, or moved.

## Shared skills under `.agents/skills/`

Current entries:

- **`nuxt`** — Nuxt project structure, routing, data fetching, SSR-safe state, middleware, plugins, server routes, runtime config, and layers. Source tracked by `skills-lock.json`.
- **`nuxt-ui`** — Nuxt UI components, theming, forms, layouts, props/slots/events, and targeted references. Source tracked by `skills-lock.json`.
- **`ui-animation`** — UI motion/animation guidance for application interactions.
- **`curl-tool`** — symlink to the package's current [`packages/curl-tool/SKILL.md`](../../packages/curl-tool/SKILL.md).
- **`searxng-search-tool`** — symlink to the package's current [`packages/searxng-search-tool/SKILL.md`](../../packages/searxng-search-tool/SKILL.md).

`skills-lock.json` remains at repository root because the `skills` CLI expects it there. For third-party locked skills, update through the skills tooling rather than editing vendored content blindly.

### Package-level skills not mirrored under `.agents/skills/`

These are still authoritative and must be consulted directly when relevant:

- [`packages/terminal-tool/SKILL.md`](../../packages/terminal-tool/SKILL.md) — TypeScript terminal tool factory + current Rust CLI backend.
- [`packages/relay-agent/SKILL.md`](../../packages/relay-agent/SKILL.md) — current Rust MCP relay, local/remote modes, Bubblewrap boundary, and verification.

Do not assume every agent client auto-discovers repository skills. `AGENTS.md` and this index are the portable discovery path; do not add client/vendor-specific discovery symlinks or duplicate skill bodies.

## Nuxt UI MCP server

`.mcp.json` registers the project-scoped Nuxt UI MCP server. Use it when the installed Nuxt UI skill does not provide enough exact API detail for component/composable search, metadata, examples, icons, or migration pages.

Rule of thumb: the skill provides working patterns; MCP provides current exact API surface.

## Repository MCP / relay work

Do not confuse the Nuxt UI documentation MCP above with this application's own MCP/relay surfaces. For product MCP work, read:

- [`project.md`](project.md) for architecture orientation;
- [`../../packages/relay-agent/SKILL.md`](../../packages/relay-agent/SKILL.md) for current relay behavior;
- the canonical [`../memories/README.md`](../memories/README.md#relay-agent-and-mcp-security-invariants) for durable security/protocol decisions;
- [`../plans/030-previous-plans-summary.md`](../plans/030-previous-plans-summary.md) only when historical Plan 028/029/029b context is useful;
- any current `031+` plan file if a new MCP effort has been opened;
- [`../contracts/`](../contracts/) before changing client-visible frozen descriptors.

Plan 030 is history, not current external MCP client integration status. External external MCP client/OAuth behavior must be re-verified when a future task depends on it.

## Agentation — visual feedback

`agentation-vue` provides the development-only visual annotation toolbar; the project MCP configuration also includes the Agentation MCP server when configured.

The Vue package is an unofficial community port, not the official React package. Its previously reviewed version was scanned for unexpected network/eval behavior before adoption. **That review is version-specific. Re-run the security/provenance check when upgrading the package instead of treating an old scan as a permanent guarantee.**

Use Agentation for selector-level visual feedback when it helps, but do not treat visual annotations as a replacement for runtime/browser verification.
