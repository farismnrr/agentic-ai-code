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

Do not assume every package skill is auto-discovered by every agent client. This index exists so missing discovery symlinks do not make a skill invisible.

## external MCP client skill discovery

`.external-mcp/skills/*` contains external MCP client Code discovery symlinks into selected `.agents/skills/*` entries. The real shared skill content stays under `.agents/` or the package skill it links to.

Do not copy skill bodies into `.external-mcp/skills/`; duplicated guidance will drift.

## Nuxt UI MCP server

`.mcp.json` registers the project-scoped Nuxt UI MCP server. Use it when the installed Nuxt UI skill does not provide enough exact API detail:

- component/composable search;
- component metadata, props, slots, and events;
- examples and templates;
- valid icon lookup;
- documentation/migration pages.

Rule of thumb: the skill provides working patterns; MCP provides current exact API surface.

## Repository MCP / relay work

Do not confuse the Nuxt UI documentation MCP above with this application's own MCP/relay surfaces. For product MCP work, read:

- [`project.md`](project.md) for architecture orientation;
- [`../../packages/relay-agent/SKILL.md`](../../packages/relay-agent/SKILL.md) for current relay behavior;
- [`../plans/029-external-mcp-native-mcp-integration.md`](../plans/029-external-mcp-native-mcp-integration.md) and [`../plans/029b-external-mcp-mcp-production-hardening.md`](../plans/029b-external-mcp-mcp-production-hardening.md) for current external MCP client integration status;
- [`../contracts/`](../contracts/) before changing client-visible frozen descriptors.

## Agentation — visual feedback

`agentation-vue` provides the development-only visual annotation toolbar; the project MCP configuration also includes the Agentation MCP server when configured.

The Vue package is an unofficial community port, not the official React package. Its previously reviewed version was scanned for unexpected network/eval behavior before adoption. **That review is version-specific. Re-run the security/provenance check when upgrading the package instead of treating an old scan as a permanent guarantee.**

Use Agentation for selector-level visual feedback when it helps, but do not treat visual annotations as a replacement for runtime/browser verification.
