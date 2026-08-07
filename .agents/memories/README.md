# Memories

Durable context that outlives a session: decisions and the reasoning behind them, constraints that aren't visible in the code, traps someone already fell into.

One Markdown file per fact, kebab-case, starting with a one-line summary. Don't record what the repo already states — code structure, git history, or anything in [`../knowledge/`](../knowledge/). Record the *why* that isn't written down anywhere else. Delete a memory when it stops being true.

## Index

- [port-3333.md](port-3333.md) — why the dev server isn't on 3000
- [no-type-aware-linting.md](no-type-aware-linting.md) — why typescript-eslint type-aware rules are off
- [pnpm-over-bun.md](pnpm-over-bun.md) — why pnpm was chosen for this project
- [ai-sdk-native-features.md](ai-sdk-native-features.md) — stream simulation and tool approval are built into `ai@7`; don't rebuild them
- [verify-in-a-browser.md](verify-in-a-browser.md) — SSR HTML is not proof the app runs; how a whole plan shipped broken
- [auth-utils-type-augmentation.md](auth-utils-type-augmentation.md) — `#auth-utils` module augmentation must live in `shared/types/`, not `app/types/`, or `session.user` types break
- [background-command-output.md](background-command-output.md) — don't judge a background build/dev command as stuck from `tail` or CPU%; read the full output file
- [fontless-build-hang.md](fontless-build-hang.md) — `pnpm build` finishes but never exits: `fontless`/`@nuxt/fonts` leaks an esbuild service, worked around via `hooks.close` in `nuxt.config.ts`
- [007-workspace-client-routing.md](007-workspace-client-routing.md) — why workspace active state is client-side rather than nested in the URL
- [007-9router-config.md](007-9router-config.md) — how the real model backend is wired up via 9router
- [007-typecheck-gate-was-silent.md](007-typecheck-gate-was-silent.md) — `nuxt typecheck` silently passed with real errors; script now runs `vue-tsc` directly
