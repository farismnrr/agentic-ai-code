# Project

Nuxt 4 application built on the official Nuxt UI v4 starter template.

- **Nuxt** 4.5 — `srcDir` is `app/` (Nuxt 4 default directory structure)
- **Nuxt UI** 4.10 — component library (Reka UI + Tailwind Variants)
- **Tailwind CSS** 4.3 — configured via CSS, not `tailwind.config.js`
- **Package manager: pnpm** (pinned in `package.json` → `packageManager`)

## Commands

| Task | Command |
| --- | --- |
| Local verification | `pnpm build && pnpm preview` → http://localhost:3333 (see note below) |
| Dev server (rarely — see note) | `pnpm dev` → http://localhost:3333 |
| Production build | `pnpm build` |
| Preview build | `pnpm preview` — serves the real `.output`, same as production |
| Lint | `pnpm lint` |
| Autofix | `pnpm lint:fix` |
| Type check | `pnpm typecheck` |
| Audit dependencies | `pnpm audit` — must report zero before any merge |
| Regenerate `.nuxt` types | `pnpm postinstall` (runs `nuxt prepare`) |

Before declaring work done, run `pnpm lint`, `pnpm typecheck` and `pnpm audit`. All three gate every PR in CI.

**Run the built app, not `pnpm dev`, for local verification.** `pnpm dev`'s on-demand Vite/Nitro compilation is slow to warm up and — worse — its file-watcher gets confused by the branch switches, worktree removals, and file moves that happen constantly during agent work, producing spurious `ENOTDIR`/stale-module errors that look like real bugs but aren't (see `.agents/memories/dev-mode-vs-build.md`). After every merge to `dev`, rebuild (`rm -rf .nuxt .output && pnpm build`) and run `pnpm preview` — it serves the actual `.output`, starts instantly since nothing compiles per-request, and is what CI/production actually run. Reach for `pnpm dev` only when actively iterating on a single file's HMR feedback loop, not as the default way to check the app works.

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
