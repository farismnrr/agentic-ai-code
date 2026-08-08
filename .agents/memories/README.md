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
- [007-typecheck-gate-was-silent.md](007-typecheck-gate-was-silent.md) — `pnpm typecheck` (`nuxt typecheck`) can silently pass real type errors; two fixes were tried and reverted, so `pnpm build && vue-tsc -p .nuxt/tsconfig.json --noEmit` is still the only real gate — confirmed again in plan 012, where `nuxt typecheck` passed on code that didn't compile at all
- [012-mcp-inbound-sse-transport.md](012-mcp-inbound-sse-transport.md) — inbound MCP server uses SSE + an in-memory session map, not Streamable HTTP as planned — single-instance only
- [012-mcp-outbound-tool-loop.md](012-mcp-outbound-tool-loop.md) — outbound chat tool-calling runs on `streamText` + `@ai-sdk/openai-compatible` + `toolApproval`, not hand-rolled SSE parsing; denying a tool blocks execution but doesn't stop the model from retrying the call
- [fixtures-not-disposable.md](fixtures-not-disposable.md) — check what's actually inside `fixtures/` before deleting it; `models.ts` was real config, not a stub
- [009-loaded-state-pattern.md](009-loaded-state-pattern.md) — use `useState`, not a bare `ref`, for a composable's own "have I loaded yet" flag, or it won't be shared across call sites
- [nuxt-ssr-fetch-cookies.md](nuxt-ssr-fetch-cookies.md) — a composable's bare `$fetch('/api/...')` silently 401s during SSR; needs `useRequestFetch()` on the server
- [010-workspace-configured-root.md](010-workspace-configured-root.md) — why workspace folders use one operator-configured root (OpenClaw precedent) instead of an unrestricted filesystem browser
- [dev-mode-vs-build.md](dev-mode-vs-build.md) — use `pnpm build && pnpm preview` for local verification, not `pnpm dev`; kill and restart `preview` on every rebuild, it doesn't pick up a new `.output` on its own
- [013-nuxt-ui-slot-typecheck-gate.md](013-nuxt-ui-slot-typecheck-gate.md) — a wrong `:ui` slot key on a Nuxt UI component silently no-ops at runtime; only `vue-tsc -p .nuxt/tsconfig.json --noEmit` catches it, not `nuxt build`
- [014-scripts-dir-not-gated.md](014-scripts-dir-not-gated.md) — `scripts/*.ts` one-off scripts aren't covered by build/typecheck/lint; a broken one only surfaces by actually running it
- [chat-onend-silent-persistence-failure.md](chat-onend-silent-persistence-failure.md) — `chat.post.ts`'s `onEnd` can fail to persist the assistant message with zero error output, intermittently; now logged, root cause (suspected abort-controller race) still open
- [015-composable-after-await-breaks-ssr-context.md](015-composable-after-await-breaks-ssr-context.md) — calling a composable after an `await` inside a plain (non-component) async function breaks Nuxt SSR context; surfaces only as `NUXT_E1001`, silently swallowed by `Promise.allSettled`
- [018-langgraph-ui-stream-bridge.md](018-langgraph-ui-stream-bridge.md) — bridging LangGraph streamEvents (v2) to AI SDK's createUIMessageStream requires manual mapping and accumulation for persistence
