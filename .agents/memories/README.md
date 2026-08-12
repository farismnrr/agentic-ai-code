# Memories

Durable context that outlives a session: decisions and their reasoning, constraints that are easy to miss from code alone, and traps someone already hit.

Read this index before repeating work in an area. When adding a memory, add it here in the same change. Prefer amending an existing memory over creating a near-duplicate, and delete/supersede entries that stop being true.

New filenames use kebab-case. One historical Plan 028 file predates that convention and keeps its underscore name to avoid breaking references.

## Application and tooling

- [no-ci-local-commit-gates.md](no-ci-local-commit-gates.md) — repository policy: no CI and no unit-test suite; every commit must pass the tracked local lint/typecheck gate without bypassing it.
- [port-3333.md](port-3333.md) — why the local Nuxt server uses port 3333 instead of the usual 3000.
- [pnpm-over-bun.md](pnpm-over-bun.md) — why pnpm was chosen for this project.
- [no-type-aware-linting.md](no-type-aware-linting.md) — why typescript-eslint type-aware lint rules are disabled.
- [007-typecheck-gate-was-silent.md](007-typecheck-gate-was-silent.md) — `nuxt typecheck` has silently passed code that does not compile; use the explicit generated Nuxt project type gate for confidence.
- [013-nuxt-ui-slot-typecheck-gate.md](013-nuxt-ui-slot-typecheck-gate.md) — invalid Nuxt UI `:ui` slot keys can no-op at runtime and require the explicit Vue typecheck gate to catch.
- [014-scripts-dir-not-gated.md](014-scripts-dir-not-gated.md) — one-off `scripts/*.ts` files are not automatically proven by the normal app gates; execute the relevant script.
- [dev-mode-vs-build.md](dev-mode-vs-build.md) — prefer build + preview for final local verification; dev watcher state can become stale after agent branch/file operations.
- [background-command-output.md](background-command-output.md) — do not diagnose long-running background commands from `tail`/CPU alone; inspect full captured output.
- [fontless-build-hang.md](fontless-build-hang.md) — historical `fontless`/Nuxt fonts build-exit hang and its workaround context.
- [fixtures-not-disposable.md](fixtures-not-disposable.md) — inspect fixture-like directories before deleting them; this repo previously kept real config in one.
- [verify-in-a-browser.md](verify-in-a-browser.md) — SSR/build output is not proof that an interactive feature works; browser verification caught a previously shipped break.
- [playwright-testing-real-dev-db.md](playwright-testing-real-dev-db.md) — Playwright can hit the shared dev DB; do not assume test data is isolated.

## Nuxt/application behavior

- [007-workspace-client-routing.md](007-workspace-client-routing.md) — why workspace active state was kept client-side rather than encoded in nested routes.
- [007-9router-config.md](007-9router-config.md) — historical 9Router model-backend wiring context.
- [009-loaded-state-pattern.md](009-loaded-state-pattern.md) — use shared Nuxt state rather than a bare local `ref` for composable loaded-state that must be shared across callers.
- [010-workspace-configured-root.md](010-workspace-configured-root.md) — why workspaces use an operator-configured filesystem root instead of an unrestricted browser.
- [nuxt-ssr-fetch-cookies.md](nuxt-ssr-fetch-cookies.md) — server-side composable fetches need request cookies/context or authenticated calls silently become 401s.
- [015-composable-after-await-breaks-ssr-context.md](015-composable-after-await-breaks-ssr-context.md) — calling Nuxt composables after `await` in plain async functions can lose SSR context.
- [018-sidebar-single-fetch.md](018-sidebar-single-fetch.md) — why the sidebar moved to one joined server response instead of chained client composable fetches.
- [chat-onend-silent-persistence-failure.md](chat-onend-silent-persistence-failure.md) — chat `onEnd` persistence has failed silently/intermittently; logging exists and the underlying race was not originally proven.
- [chat-client-errors-were-silent.md](chat-client-errors-were-silent.md) — provider/chat failures used to be console-only and looked like an unresponsive UI.

## AI/tool orchestration

- [ai-sdk-native-features.md](ai-sdk-native-features.md) — AI SDK 7 already provides stream simulation/tool approval features; do not rebuild them without a reason.
- [012-mcp-inbound-sse-transport.md](012-mcp-inbound-sse-transport.md) — historical app-level inbound MCP SSE/session transport decision; do not confuse it with the later Rust relay's stateless Streamable HTTP target.
- [012-mcp-outbound-tool-loop.md](012-mcp-outbound-tool-loop.md) — outbound chat tool-calling uses AI SDK/OpenAI-compatible tool approval rather than a hand-rolled SSE loop.
- [018-langgraph-ui-stream-bridge.md](018-langgraph-ui-stream-bridge.md) — LangGraph/LangChain to AI SDK UI-stream bridge behavior and valid dynamic-tool states.
- [019-search-forced-tool-choice-unreliable.md](019-search-forced-tool-choice-unreliable.md) — forced tool choice through the installed LangChain agent path was unreliable; `@search` calls the tool directly instead.
- [auth-utils-type-augmentation.md](auth-utils-type-augmentation.md) — `#auth-utils` module augmentation must live in `shared/types/` for `session.user` typing to work.

## Plan 027 — Rust CLI migration

- [027-cli-behavior-dependency-matrix.md](027-cli-behavior-dependency-matrix.md) — boundary and behavior matrix for the migrated native CLI tools.
- [027-rust-architecture-toolchain.md](027-rust-architecture-toolchain.md) — Rust workspace/toolchain architecture and separation from the Nuxt app.
- [027-strict-differential-parity.md](027-strict-differential-parity.md) — migration parity rules used while replacing the JavaScript CLIs.
- [027-terminal-tool-process-safety.md](027-terminal-tool-process-safety.md) — terminal child-process, timeout, and cleanup invariants.
- [027-process-termination-contract.md](027-process-termination-contract.md) — process termination/descendant cleanup contract.
- [027-exit-code-contract.md](027-exit-code-contract.md) — native CLI exit-code compatibility contract.
- [027-curl-tool-ssrf-policy.md](027-curl-tool-ssrf-policy.md) — curl-tool SSRF and redirect policy decisions.
- [027-searxng-deterministic-fixtures.md](027-searxng-deterministic-fixtures.md) — deterministic SearXNG migration fixture strategy.
- [027-pnpm-workspace-integration.md](027-pnpm-workspace-integration.md) — how the Rust binaries coexist with the pnpm/TypeScript workspace packages.
- [027-rust-release-supply-chain.md](027-rust-release-supply-chain.md) — native artifact/release supply-chain decisions and rollback posture.
- [027-supply-chain-policy.md](027-supply-chain-policy.md) — concise dependency/supply-chain policy from the migration.
- [027-performance-benchmark.md](027-performance-benchmark.md) — recorded JS→Rust performance evidence.
- [027-zero-js-cli-cutover.md](027-zero-js-cli-cutover.md) — hard cutover invariant: Rust owns the executable CLI layer; no permanent JavaScript CLI fallback or npm bin mapping.
- [027-final-closeout.md](027-final-closeout.md) — Plan 027 completion summary and final invariants.

## Plan 028 — relay-agent Rust rewrite

- [028-relay-agent-phase19-security-decisions.md](028-relay-agent-phase19-security-decisions.md) — Bubblewrap containment, JWKS/OAuth, privilege, and zero-bypass security decisions.
- [028_relay_agent_rewrite_review_notes.md](028_relay_agent_rewrite_review_notes.md) — final Rust relay/E2E review notes, including bwrap mount-order and shell-compatibility lessons. Historical underscore filename retained.

## Plan 029 / 029b — ChatGPT MCP integration

- [029-chatgpt-mcp-integration-decisions.md](029-chatgpt-mcp-integration-decisions.md) — frozen Plan 029 target: stateless `POST /mcp`, external OAuth provider model, and coarse `relay.coding` capability.
- [chatgpt-mcp-integration-insights.md](chatgpt-mcp-integration-insights.md) — concise integration observations captured alongside Plan 029.
- [029-phase6-chatgpt-e2e-acceptance.md](029-phase6-chatgpt-e2e-acceptance.md) — repository acceptance harness exists, but live ChatGPT/OAuth evidence remains an operator gate.
- [029-phase7-published-app-lifecycle.md](029-phase7-published-app-lifecycle.md) — frozen public tool-catalog lifecycle and republish/Refresh rules.
- [029b-trusted-proxy-boundary.md](029b-trusted-proxy-boundary.md) — remote relay proxy trust is explicit and peer-scoped; forwarded headers are not trusted automatically.
- [029b-docker-capability-blocker.md](029b-docker-capability-blocker.md) — Docker remains intentionally disabled until an isolated daemon/broker/worker boundary exists.

## Index integrity rule

Every Markdown file in this directory except this README must appear above. If a memory is removed or renamed, update incoming links and this index in the same change. Do not create a placeholder memory merely to satisfy an old reference; repair the stale reference or recover the actual durable decision instead.
