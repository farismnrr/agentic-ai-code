# Skills and MCP

## Skills

Installed in [`../skills/`](../skills/); `.external-mcp/skills/*` are symlinks into it so external MCP client Code discovers them automatically. Consult them before writing code.

- **`nuxt`** — project structure, routing, data fetching, SSR-safe state, middleware, plugins, server routes, runtime config, layers. Source: `onmax/nuxt-skills`.
- **`nuxt-ui`** — the 125+ Nuxt UI components, theming, forms, layouts. Its `references/` subdirectory holds targeted guides; load only the ones relevant to the task. Source: official `nuxt/ui` repo.

`skills-lock.json` at the repo root records their source and version. Update with `npx skills update`.

## Nuxt UI MCP server

`.mcp.json` registers the Nuxt UI MCP server (`https://ui.nuxt.com/mcp`), project-scoped. Use it for anything the skill doesn't cover — exact props, slots, and events:

- `search-components` / `search-composables` — find by name or description
- `get-component` — full docs with examples; `get-component-metadata` — props/slots/events only
- `get-example` / `list-examples` — real-world usage
- `search-icons` — returns valid `i-{prefix}-{name}` icon names
- `search-documentation` / `get-documentation-page` — the docs site itself
- `list-templates` / `get-template` / `get-migration-guide`

Rule of thumb: the skill tells you **which** component to use and **how** to build well; the MCP tells you **what the API is**.
