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

## Agentation — visual feedback

Click an element in the running app, leave a note, and get selectors an agent can grep for — instead of describing "the blue button in the sidebar".

- **Toolbar**: `agentation-vue`, mounted by [`app/plugins/agentation.client.ts`](../../app/plugins/agentation.client.ts). Dev-only and client-only.
- **MCP server**: `agentation` in `.mcp.json` (`npx -y agentation-mcp server`, stdio). Official package; lets the agent receive annotations directly.

**Provenance, because it matters here:** the official `agentation` package is **React-only** and does not endorse any Vue port. `agentation-vue` is an unofficial community port by a different maintainer (`Blaked84`), v0.3.0, ~2.9k weekly downloads against the official package's ~950k.

It was scanned before adoption — across all 107 dist files: no `fetch`/XHR/WebSocket/`sendBeacon`, no `eval`/`new Function`, no external URLs. It touches `localStorage` for settings and the clipboard for output, which is exactly what it claims. **Re-run that scan on any upgrade** — the guarantee is version-specific, not permanent.

The toolbar is mounted from the plugin into its own root rather than placed as a tag in `app.vue`. A tag there compiles to a `resolveComponent` call that survives into the production bundle as dead code. Verified: zero references to agentation across all 661 files of `.output/`.
