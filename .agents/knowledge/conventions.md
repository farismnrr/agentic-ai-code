# Conventions

- **Use semantic color classes** — `text-default`, `text-muted`, `bg-elevated`, `border-muted`. Never raw palette colors like `text-gray-500`; they break dark mode and theming.
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
