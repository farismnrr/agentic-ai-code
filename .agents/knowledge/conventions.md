# Conventions

- **API responses are shaped server-side, fully — joins, pagination, filtering, all of it.** A screen that needs data from more than one table gets one endpoint that returns it pre-joined, not two endpoints fetched and merged client-side. This isn't a style preference: chaining/orchestrating multiple composable calls to assemble one screen's data (`a().then(b)`, or even two independent calls placed side by side in the same `Promise.allSettled`) repeatedly broke Nuxt's SSR composable context in this codebase — every extra await/`.then()` boundary is a fresh place for that failure class to reappear, and it fails *silently* (see [[015-composable-after-await-breaks-ssr-context]], [[018-sidebar-single-fetch]]). If you're about to write FE code that combines, re-keys, or sequences more than one API response, the fix belongs in the BE endpoint, not in FE orchestration.
- **Use semantic color classes** — `text-default`, `text-muted`, `bg-elevated`, `border-muted`. Never raw palette colors like `text-gray-500`; they break dark mode and theming.
- **The signal colour (cyan/primary) is reserved for things that are actually happening.** Streaming text, a running tool call, a connected MCP server, a focused input. Nothing decorative ever uses it.
- **Brand colors live in `app/app.config.ts`** (`ui.colors.primary` / `ui.colors.neutral`), not hardcoded in components.
- **Icons use `i-{collection}-{name}`** — `lucide` and `simple-icons` are installed locally. Anything else needs a new `@iconify-json/*` dependency.
- **Components and composables are auto-imported.** Don't add manual imports for anything under `app/components/` or `app/composables/`.
- **`<UApp>` must stay as the outermost element** in `app.vue` — toasts, tooltips, and programmatic overlays depend on it.
- **ESLint style** (enforced, set in `nuxt.config.ts`): no trailing commas, 1TBS brace style. Let `pnpm lint:fix` handle formatting rather than hand-formatting.
- **Prefer Nuxt UI components over hand-rolled markup.** Check the library before building a custom button, modal, table, or form field.
- `/` is prerendered (`routeRules` in `nuxt.config.ts`) — keep it free of request-time-only data.

## Gotchas

- `.nuxt/` is generated. Never edit it, but *do* read `.nuxt/ui/<component>.ts` to discover a component's real slots, variants, and default classes.
- Tailwind 4 has no config file here. Theme extensions go in `app/assets/css/main.css` via `@theme`.
- `pnpm-workspace.yaml` disables postinstall builds for several native deps. If a native package misbehaves, that file is the first place to look.
