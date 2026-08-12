# 020 — Extract chat-mode tools into standalone local npm packages + skills

## Context

`server/utils/tools/curl-tool.ts` and `server/utils/tools/searxng-search-tool.ts` currently live inline in the Nitro server, each exporting one LangChain `tool()`, consumed only via `server/utils/langgraph-tools.ts`'s `langgraphTools` array. The ask: turn each into a standalone, locally-installable npm package (installable via `npx`/`pnpm add` from a local path — not published to a public registry), each documented with a repository skill (`SKILL.md`, matching the format already used by `.agents/skills/nuxt`, `.agents/skills/nuxt-ui`, etc. — frontmatter `name`/`description`/`license`, a short intro, usage). This repo is not currently a pnpm workspace (`pnpm-workspace.yaml` has no `packages:` glob, no `packages/` dir exists, no `file:`/`link:`/`workspace:` dependency precedent) — this plan introduces that structure for the first time.

**Two real couplings have to be resolved, not just moved:**
- `curlTool` calls `assertSafeUrl()` from `server/utils/ssrf-guard.ts` — the same SSRF guard `server/utils/mcp-client.ts` uses for MCP connections. `.agents/memories/012-mcp-outbound-tool-loop.md` is explicit that this guard must stay a single shared implementation, not forked. **The extracted package must not bundle its own copy of this security logic** — it takes the guard as a required constructor parameter instead, so the app wires in its real `assertSafeUrl` and any other consumer must consciously supply their own.
- `searxngSearchTool` calls `useRuntimeConfig()` (a Nuxt auto-import) to read `config.searxngBaseUrl` — that composable doesn't exist outside a Nuxt/Nitro process, so the package takes `baseUrl` as a required constructor parameter instead.

Both packages become plain factory functions with no Nuxt/Nitro dependency, callable from any Node context — which is what makes "npx local install, reusable elsewhere" actually true rather than nominal.

## Decisions

- **New `packages/` directory, wired into a real pnpm workspace.** `pnpm-workspace.yaml` gets `packages: - 'packages/*'` added to its existing `allowBuilds`/`overrides` config (nothing else in that file changes).
- **Package names**: `@ai-code/curl-tool` and `@ai-code/searxng-search-tool`, both `"private": true` (local-only, never meant for the public registry — `private: true` doesn't block `workspace:*`/local installs, only `npm publish`).
- **Each package exports a factory, not a pre-built tool instance** — `createCurlTool({ assertSafeUrl }) => Tool` and `createSearxngSearchTool({ baseUrl }) => Tool`. The app's `server/utils/langgraph-tools.ts` calls these with its real `assertSafeUrl` (still from `server/utils/ssrf-guard.ts`, untouched) and `useRuntimeConfig().searxngBaseUrl`. This is a deliberate DI boundary, not incidental — it's what keeps the SSRF guard singular and the package Nuxt-free.
- **Each package also ships a `bin` CLI** — a small script (`bin/cli.mjs`) so `npx @ai-code/curl-tool <url>` / `npx @ai-code/searxng-search-tool <query> --base-url <url>` actually does something standalone, not just an importable function. This is what makes "installable via npx" concretely true rather than only `pnpm add`-able.
- **App consumes them via `workspace:*`** — root `package.json` gets `"@ai-code/curl-tool": "workspace:*"` and `"@ai-code/searxng-search-tool": "workspace:*"` as real dependencies (first `workspace:` precedent in this repo); `server/utils/tools/curl-tool.ts` and `server/utils/tools/searxng-search-tool.ts` are deleted, `server/utils/langgraph-tools.ts` imports the packages instead.
- **Skills are authored locally, not GitHub-sourced.** `skills-lock.json` + `npx skills update` is for *externally* sourced skills (each existing entry has a GitHub `source`); these two are original docs about this repo's own code, so they don't belong in that lockfile. Each package gets its own `SKILL.md` at `packages/<name>/SKILL.md`, following the same frontmatter/structure as `.agents/skills/nuxt/SKILL.md` (`name`, `description`, `license`, intro, a "how to use" section with both the factory and the CLI). Shared discovery belongs under `.agents/skills/`; repository-owned client-specific discovery links are not required.

## Changes

### `packages/curl-tool/`
- `package.json` (name `@ai-code/curl-tool`, private, `type: module`, `bin: { curl-tool: ./bin/cli.mjs }`, deps: `@langchain/core`, `zod`).
- `src/index.ts` — `createCurlTool({ assertSafeUrl }: { assertSafeUrl: (url: URL, context: string) => Promise<void> })`, same `tool()` body as today's `curl-tool.ts` minus the import, calling the injected `assertSafeUrl`.
- `bin/cli.mjs` — argv-driven standalone runner (URL arg, optional `--method`/`--header`/`--body`), using a permissive no-op guard by default with a `--no-guard`-style opt-out flag documented as unsafe for open/public use — actual default behavior TBD during implementation, but must not silently skip guarding without the caller choosing that.
- `SKILL.md`.

### `packages/searxng-search-tool/`
- `package.json` (name `@ai-code/searxng-search-tool`, private, `type: module`, `bin: { searxng-search-tool: ./bin/cli.mjs }`, deps: `@langchain/core`, `zod`).
- `src/index.ts` — `createSearxngSearchTool({ baseUrl }: { baseUrl: string })`, same `tool()` body as today's `searxng-search-tool.ts` minus `useRuntimeConfig()`.
- `bin/cli.mjs` — argv-driven standalone runner (query arg, `--base-url` flag, required or defaulting to `http://127.0.0.1:8888` per the container this repo already runs).
- `SKILL.md`.

### App wiring
1. `pnpm-workspace.yaml`: add `packages: - 'packages/*'`.
2. Root `package.json`: add both packages as `workspace:*` dependencies; `pnpm install` to link them.
3. `server/utils/langgraph-tools.ts`: import `createCurlTool`/`createSearxngSearchTool`, call with `assertSafeUrl` (from `./ssrf-guard`) and `useRuntimeConfig().searxngBaseUrl` respectively; `langgraphTools` array unchanged in shape.
4. Delete `server/utils/tools/curl-tool.ts`, `server/utils/tools/searxng-search-tool.ts`, and the now-empty `server/utils/tools/` directory.
5. Symlink `.agents/skills/curl-tool -> ../../packages/curl-tool` and `.agents/skills/searxng-search-tool -> ../../packages/searxng-search-tool` so the package skills remain discoverable through the shared agent guidance tree.

## Out of scope

- Publishing either package to the public npm registry.
- Extracting `assertSafeUrl` itself into a package (it stays app-owned, shared between `mcp-client.ts` and the curl tool exactly as today — only *injected into* the curl package, not moved).
- Any change to `langgraph-chat.ts`'s `@search` forcing logic or its currently-broken `MultipleToolsBoundError` bug (separate, already-flagged issue on `feat/019-p1-search-trigger`, not part of this plan).
- Registering these skills in `skills-lock.json` (that mechanism is for GitHub-sourced skills only).

## Verification

- `pnpm install` — confirms the workspace links resolve (`node_modules/@ai-code/curl-tool` etc. become symlinks to `packages/*`).
- `pnpm lint`, `pnpm typecheck`, `pnpm build` — confirm the app still compiles against the package imports.
- `npx --package=./packages/curl-tool curl-tool https://example.com` and `npx --package=./packages/searxng-search-tool searxng-search-tool "bnsp"` (or `pnpm --filter` equivalents) — confirm each CLI runs standalone, outside the Nuxt app.
- Manual, in a chat-mode conversation: confirm `curl`/`searxng_search` tool calls still work exactly as before the extraction (no behavior change expected, only where the code lives).
- Confirm `server/utils/ssrf-guard.ts` still has exactly one implementation, still shared by `mcp-client.ts`, and that the curl package's guard is unmistakably the *same* function object passed in, not a re-implementation.
