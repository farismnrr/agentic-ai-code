# Conventions

- **API responses are shaped server-side, fully — joins, pagination, filtering, all of it.** A screen that needs data from more than one table gets one endpoint that returns it pre-joined, not two endpoints fetched and merged client-side. Chaining/orchestrating multiple composable calls to assemble one screen repeatedly broke Nuxt's SSR composable context in this codebase; see the canonical memory's [Nuxt/application invariants](../memories/README.md#nuxt-application-and-data-loading-invariants). If FE code is about to combine, re-key, or sequence multiple API responses for one screen, strongly prefer moving that shape to the backend endpoint.
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

## Maintainability and ownership

- Apply DRY, pragmatic SOLID, Layered Architecture, YAGNI, and KISS to modules and feature ownership; extract shared **knowledge/policy**, not coincidental syntax.
- Treat `scripts/check-maintainability.mjs` thresholds as guardrails: >400 source lines and 13–15 direct implementation files require review; >500 lines and >15 files require a responsibility split or an exact, documented cohesion exception.
- Never create one-file-per-function folders, pass-through wrappers, DI/service-locator frameworks, or speculative extension systems merely to satisfy a metric.
- `app/composables/` is a narrow framework-cohesive exception at 16 direct files because those files are public Nuxt auto-import `use*` entrypoints. Keep internal chat controllers/helpers under feature subfolders, but do not move public composables just to reduce the count or add wrapper files with no ownership value.
- When architecture/module/folder ownership changes, update relevant human docs and `.agents/` guidance in the same task before closure.
- Tests are named for behavior/features, never plan numbers. Web tests belong under `test/`; Rust tests belong under package `tests/`. Do not add `verify-*`, `phase-*`, or acceptance scripts under `scripts/` for feature work.
