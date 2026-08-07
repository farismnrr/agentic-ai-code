# Project

Nuxt 4 application built on the official Nuxt UI v4 starter template.

- **Nuxt** 4.5 — `srcDir` is `app/` (Nuxt 4 default directory structure)
- **Nuxt UI** 4.10 — component library (Reka UI + Tailwind Variants)
- **Tailwind CSS** 4.3 — configured via CSS, not `tailwind.config.js`
- **Package manager: pnpm** (pinned in `package.json` → `packageManager`)

## Commands

| Task | Command |
| --- | --- |
| Dev server | `pnpm dev` → http://localhost:3333 |
| Production build | `pnpm build` |
| Preview build | `pnpm preview` |
| Lint | `pnpm lint` |
| Autofix | `pnpm lint:fix` |
| Type check | `pnpm typecheck` |
| Regenerate `.nuxt` types | `pnpm postinstall` (runs `nuxt prepare`) |

Before declaring work done, run `pnpm lint` and `pnpm typecheck`.

## Layout

```
app/
  app.vue              # root — wraps everything in <UApp>, UHeader/UMain/UFooter
  app.config.ts        # runtime UI config (primary/neutral color tokens)
  assets/css/main.css  # @import "tailwindcss" + @import "@nuxt/ui"
  components/          # auto-imported, no explicit import needed
  pages/               # file-based routing
nuxt.config.ts         # modules, css, runtimeConfig, routeRules, devServer, eslint
public/                # served at site root
.agents/               # all agent-facing material (this folder)
.mcp.json              # project-scoped MCP servers
```
