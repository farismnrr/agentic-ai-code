# The Nuxt way — working agreement

**Follow the idiomatic Nuxt way for everything in this repo — dependencies, config, code, and file placement. Prefer the framework's own mechanism over a generic JS/Vue solution, even when the generic one is familiar or shorter.**

Before writing code or adding a dependency:

1. **Consult the installed skills first** — `nuxt` for framework work, `nuxt-ui` for anything visual. They encode the conventions this project expects. See [`resources.md`](resources.md).
2. **Use the `nuxt-ui` MCP server for component APIs** (props, slots, events, examples) instead of guessing or recalling.
3. **When neither covers it, check the official docs** (nuxt.com, ui.nuxt.com, nuxt.com/modules) rather than reasoning from Vue/Vite knowledge alone. Nuxt-specific behavior often contradicts the generic answer.
4. **If a Nuxt-native mechanism exists, use it.** Reach for a third-party or hand-rolled approach only when nothing built in covers the case, and say why.

## Adding dependencies

| What | How |
| --- | --- |
| A Nuxt module | `pnpm dlx nuxi module add <name>` |
| A Nuxt module, dev-only | `pnpm dlx nuxi module add <name> --dev` |
| A plain runtime library | `pnpm add <pkg>` |
| A plain dev tool | `pnpm add -D <pkg>` |
| Upgrading Nuxt itself | `pnpm dlx nuxi upgrade` |

- **`nuxi module add` is mandatory for Nuxt modules.** It resolves a version compatible with this Nuxt release *and* registers the module in `nuxt.config.ts`. A bare `pnpm add` does neither, and a module that isn't in `modules[]` silently does nothing.
- **Check nuxt.com/modules before adding a non-Nuxt library.** If an official or community module wraps it, use the module — you get auto-imports, SSR handling, and devtools integration for free.
- **pnpm only.** Never npm/yarn/bun; it desyncs `pnpm-lock.yaml`. The manager is pinned in `package.json` → `packageManager`.
- After any install, run `pnpm postinstall` if types or auto-imports look stale, then `pnpm lint` and `pnpm typecheck`.

## Prefer the Nuxt-native mechanism

| Instead of | Use |
| --- | --- |
| `process.env.X` in app code | `useRuntimeConfig()` |
| `axios` / bare `fetch` | `$fetch`, `useFetch`, `useAsyncData` |
| Manual `import` of your own components/composables | auto-imports (`app/components/`, `app/composables/`) |
| `ref` in module scope for shared state | `useState()` (SSR-safe) |
| Vue Router config | file-based routing in `app/pages/` |
| `document` / `window` at setup top level | `import.meta.client` guard or `onMounted()` |
| A standalone Express/Fastify API | Nitro server routes in `server/api/` |
| `vue-meta` or manual `<head>` edits | `useHead()` / `useSeoMeta()` |
| Adding Prettier | the already-enabled ESLint stylistic rules |
| `tailwind.config.js` | `@theme` in `app/assets/css/main.css` (Tailwind 4) |
| A hand-built button/modal/table/form field | the equivalent Nuxt UI component |
