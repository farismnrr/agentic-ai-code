# Canonical Memory

**Last compacted: 2026-08-12.** This is the repository's **only durable memory file**. All memory notes that existed before this date were folded into this document. Git history keeps the original long-form notes when forensic detail is needed.

Do **not** add sibling Markdown memory files. Update this file in place: add a concise durable lesson, amend a decision when it changes, and remove stale guidance rather than accumulating near-duplicates.

Current source/config and `.agents/knowledge/` remain authoritative for implementation facts. This file exists for decisions, constraints, failure modes, and non-obvious reasoning that future agents should not have to rediscover.

## Repository policy and verification

- The repository intentionally has **no CI workflow** and **no unit-test suite**. Do not quietly reintroduce either one; changing that policy requires an explicit user decision.
- Every normal local commit must pass the tracked pre-commit gate. `pnpm install` configures `core.hooksPath=.githooks`; `.githooks/pre-commit` runs `pnpm verify:commit`.
- `pnpm verify:commit` runs repository-policy enforcement, agent-doc integrity, `pnpm lint`, and `pnpm typecheck`. A failure means **do not commit**.
- Never use `git commit --no-verify` or disable/replace `core.hooksPath` to evade the gate. With no server-side CI, the local gate and the operator/agent's honesty are the enforcement model.
- `pnpm lint` is cross-stack: ESLint, Rust formatting, and warnings-denied Clippy.
- `pnpm typecheck` is cross-stack: `nuxt prepare --dotenv .env.example`, direct `vue-tsc -p .nuxt/tsconfig.json --noEmit`, then warnings-denied Rust `cargo check`.
- Do **not** simplify the Nuxt/Vue type gate to bare `nuxt typecheck`. That wrapper previously exited successfully while generated-project Vue errors remained. Invalid Nuxt UI `:ui` slot keys were one concrete class of bug caught only by the direct generated-project typecheck.
- Production bundling is a separate concern. Run `pnpm build` when SSR/bundling/runtime output needs proof; the mandatory typecheck gate is intentionally not coupled to a full production build.
- Dependency changes additionally require `pnpm audit` before merge. Security-sensitive Rust/MCP changes may additionally require `cargo audit` and the relevant deterministic scripts under `scripts/`.
- GitHub reporting a PR as mergeable is not verification. PR descriptions should record the actual local commands/results that were run.

## Tooling and local-development decisions

- Use **pnpm**, not Bun, for the workspace. The exact pnpm version is pinned by the root `packageManager` field.
- The normal Nuxt dev port is **3333**. It was chosen because common ports were occupied on the original development environment; do not assume 3000 in docs/scripts.
- Type-aware `typescript-eslint` linting is intentionally not enabled as a second type system. Type correctness is owned by the explicit Nuxt/Vue typecheck gate.
- One-off TypeScript under `scripts/` is not automatically proven merely because application typecheck passes. Execute or otherwise validate the relevant script when changing it.
- For final runtime verification, prefer a clean `pnpm build && pnpm preview` over trusting a long-lived `pnpm dev` process. Nuxt dev watcher/module state has become stale after branch/file operations and produced misleading `ENOTDIR`/stale-module failures.
- A successful SSR/build result is not proof that an interactive UI path works. Browser verification previously caught shipped interaction failures that static/build checks did not.
- Playwright/browser automation can hit the shared development database. Never assume automated browser data is isolated unless the environment explicitly provides isolation.
- Long-running/background commands must be diagnosed from their full captured output and exit state, not just `tail`, CPU usage, or the fact that a process still exists.
- A historical Nuxt/fontless build-exit hang existed. If a build finishes its work but does not exit, inspect the current font/module behavior before inventing unrelated fixes; do not assume the old workaround is still needed.
- Fixture/scratch-looking directories are not automatically disposable. This repository previously kept real configuration in one; inspect contents and references before deleting.

## Nuxt, application, and data-loading invariants

- Workspace active state is an application/session concern rather than a nested-route identity. Current routing should not be redesigned merely to encode workspace activity in URLs.
- Workspaces use an **operator-configured filesystem root**. Do not replace that boundary with unrestricted filesystem browsing or silently fall back to the whole machine.
- Shared composable loaded-state that is meant to be shared across callers must use shared Nuxt state (`useState`) rather than a bare local `ref` created independently per caller.
- Authenticated SSR fetches need request cookies/context. Server-side composable calls that drop request context can silently become `401` even though the browser session is valid.
- **Do not call Nuxt composables after an arbitrary `await` inside a plain async orchestration function.** This repository repeatedly hit `NUXT_E1001`/lost SSR context when composable bodies such as `useRequestFetch()` were reached after extra await/`.then()` boundaries.
- For a screen that needs related data from multiple tables, prefer **one server endpoint that returns the joined/ready shape** instead of multiple client composables fetched and merged together. The sidebar was moved to one `/api/sidebar` response specifically to remove this SSR-context failure class.
- The sidebar/workspace pattern is therefore: invoke composable-backed work while Nuxt context is still valid, do joins server-side, and assign one coherent client state result.
- `#auth-utils` session/user type augmentation belongs under `shared/types/` so both server and generated Nuxt typing see it reliably.
- Chat persistence in `onEnd` has historically failed silently/intermittently; logging was added because a UI-complete stream did not guarantee persistence succeeded. Treat persistence failures as a real server concern, not merely a display issue.
- Chat/provider failures were once console-only and made the UI look inert. User-visible error propagation is required; do not regress to silent client errors.
- The old hardcoded 9Router-only model wiring is historical. Current provider/model behavior is user-configurable; verify current source/config rather than restoring an old plan's provider assumptions.

## AI, chat, and tool-orchestration decisions

- AI SDK 7 already provides native stream/tool approval capabilities used by this repository. Do not rebuild approval/state machinery by hand without a concrete need.
- Outbound chat tool calling uses the framework/provider tool flow and approval semantics rather than a hand-rolled SSE tool loop.
- The old application-level inbound MCP SSE/session transport is **historical**. Do not confuse it with the current native Rust relay, whose target is stateless MCP Streamable HTTP over `POST /mcp`.
- LangGraph/LangChain output is bridged into AI SDK UI-stream semantics. Preserve valid dynamic-tool states and the existing UI-stream contract when changing orchestration.
- Forced tool choice through the installed LangChain agent path proved unreliable for explicit `@search`. The shipped behavior invokes the search tool directly for that explicit mention and then emits compatible tool UI chunks.
- Prefer framework-native features first; only introduce custom protocol/orchestration layers when the native path demonstrably cannot express the required behavior.

## Rust CLI migration invariants

- The standalone executable CLI layer for `terminal-tool`, `curl-tool`, and `searxng-search-tool` is **Rust-owned**. There is no supported permanent JavaScript CLI fallback and no npm `bin` mapping that should be restored.
- The Nuxt/TypeScript package APIs remain valid application integration surfaces. The Rust migration replaced executable CLIs, not the entire web/runtime stack.
- Plan 031 introduced and documented the intended layering boundary: TS/server code composes application and integration concerns, while Rust owns executable tools and relay protocol/security execution; Plan 031A owns the follow-up work required where source still contradicts that intended boundary.
- Repository development uses the pinned Rust toolchain (currently Rust 1.95.0); package MSRV is a separate compatibility floor.
- The migration was done under strict behavioral parity: arguments, stdout/stderr shape, exit codes, timeouts, and failure behavior matter. A rewrite is not acceptable merely because the happy path works.
- Terminal execution must manage child/process-group lifetime so timeout/termination does not leave descendants behind. Process cleanup is part of the contract, not optional polish.
- Native CLI exit codes are compatibility surface. Do not casually collapse distinct usage/runtime failures into one generic exit status.
- `curl-tool` SSRF protection validates the initial destination and every redirect hop, including DNS resolution to private/loopback/link-local ranges. Redirect validation uses a custom bounded policy.
- `curl-tool --no-guard` is the explicit user-visible production bypass for SSRF checks. Do not add hidden fixture/env bypasses to the production binary.
- Initial-request DNS rebinding is an accepted residual risk because the validated IP is not pinned through connection establishment. Do not claim complete DNS-rebinding protection unless the connection architecture changes.
- SearXNG migration/acceptance relied on deterministic fixtures rather than live search variability; preserve deterministic protocol checks when behavior changes.
- Native packages coexist with pnpm workspace packages; package metadata/wrappers are integration surfaces, while executable ownership stays in the Rust workspace.
- Native release is a **manual/operator** action. There is no automated GitHub Actions release workflow. Build from the reviewed commit, use the pinned toolchain, publish SHA-256 checksums, and preserve the Linux relay platform/security contract.
- Supply-chain/security posture favors locked dependencies, warnings-denied checks, and clean `cargo audit` without convenience waivers. A previous RustSec dependency path was removed by switching the JWT crypto provider rather than ignoring the advisory.
- Historical JS→Rust benchmarks showed the native migration was worthwhile, but benchmark numbers are historical evidence, not a permanent performance guarantee.

## Relay Agent and MCP security invariants

- `relay-agent` is the native Rust MCP coding server. Its production execution boundary is **Linux + Bubblewrap (`bwrap`) + non-root runtime + explicit execution root**.
- Filesystem containment is enforced at the OS namespace boundary, not by trying to parse every shell argument. Do not replace Bubblewrap containment with a fragile command/path denylist.
- Bubblewrap mount order matters; careless bind ordering can shadow the intended workspace or weaken usability. Preserve the reviewed mount/security shape when modifying sandbox setup.
- The relay resolves trusted sibling binaries relative to its own executable rather than trusting an arbitrary `$PATH`; installation-directory writability is therefore part of the trust boundary.
- Local mode is loopback-oriented. Remote mode is an OAuth **Resource Server**, not an Authorization Server and not a client-registration database.
- Remote tokens are validated through asymmetric JWKS with canonical issuer/audience/resource checks, owner binding, expiry/time validation, and the coarse `relay.coding` capability.
- The canonical remote resource is the externally reachable HTTPS MCP URL. Keep protected-resource metadata, JWT audience/resource policy, and client-visible auth metadata aligned.
- Trusted-proxy behavior is **explicit and peer-scoped**. Remote mode must not automatically trust `Forwarded`/`X-Forwarded-*`; a spoofed forwarded HTTPS header from an arbitrary peer must not create transport trust.
- Docker is intentionally **unsupported/deferred** unless an isolated worker/broker/daemon boundary is introduced. Never expose the host Docker socket, host root mounts, privileged mode, host namespaces, or equivalent host-control surface just to claim Docker support.
- Current MCP target is `2026-07-28`, stateless Streamable HTTP `POST /mcp`, with strict protocol/header/meta validation and server-side authorization before execution.
- The client-visible tool catalog is frozen/reviewable contract material. Runtime `tools/list` descriptors, OAuth `securitySchemes`, risk annotations, and the reviewed snapshot must move together deliberately.
- Completed `tools/call` results use the required `resultType: "complete"`; business/tool failures remain normal completed tool results with `isError: true` rather than being misrepresented as protocol failures.
- ChatGPT-compatible OAuth metadata/challenge support was hardened so HTTP auth enforcement and MCP result-level `mcp/www_authenticate` metadata derive from the same resource policy without letting unauthorized requests reach dispatch.
- Client confirmation UI, MCP annotations, or descriptive metadata are **not** security controls. Signature/issuer/audience/owner/scope checks and sandbox boundaries remain authoritative server-side enforcement.
- Historical repository checks proved protocol/security behavior through deterministic black-box scripts. They did **not** prove a live external ChatGPT/OAuth tenant integration.

## Historical incidents and tempting wrong fixes

- Do not infer current architecture from a plan snapshot. Several earlier designs were intentionally superseded: inbound SSE, Node/WebSocket relay, JavaScript executable CLIs, hardcoded provider assumptions, and no-jail/local relay behavior.
- Do not weaken a security boundary just to make a deterministic harness easier. Fixture-only exceptions must be narrow, explicit, and impossible to enable in production builds.
- Do not treat grep/source-string checks as proof of protocol behavior when a black-box HTTP assertion can verify the real boundary.
- Do not create a unit-test suite merely to replace explicit deterministic protocol/security acceptance scripts; the repository's current policy is still no unit-test suite.

## Planning reset — 2026-08-12

The user explicitly **closed every plan that existed at the time of this compaction**, including work that an older plan still labeled in-flight or externally unverified. The purpose is to refresh planning data rather than carry stale checklists forever.

Consequences:

- Plans `001` through `029b` are historical and are summarized in [`../plans/030-previous-plans-summary.md`](../plans/030-previous-plans-summary.md).
- A historically unchecked item is **not active work** after this reset. If it matters again, inspect current source and current external behavior, then create a fresh numbered plan.
- Historical live ChatGPT/OAuth acceptance was not proven; Docker was historically deferred. Those facts remain useful context, but neither is an automatically open plan after the reset.
- Independent future plans start at **031** and use separate incrementing numeric files. A closed plan may have an explicit lowercase-letter follow-up (for example `031a-...md`) when the user wants unresolved audit/hardening work to remain in that parent plan family. Lettered follow-ups remain separate files and do not change the next independent numeric sequence.
- On **2026-08-13**, Plan 031 was administratively closed after its implementation pass. The strict post-refactor audit findings were transferred to [`../plans/031a-refactor-hardening-and-architecture-closure.md`](../plans/031a-refactor-hardening-and-architecture-closure.md). Closing 031 does not assert those findings were fixed.
- Plan **031A** is the active follow-up for the unresolved Plan 031 audit findings. The next independent numeric plan is **032**.
- Future plan files remain separate and are **not** automatically compacted into Plan 030.

## Maintenance rule

Keep this file useful, not large. Capture the durable decision, why it exists, and the wrong-but-tempting alternative in a few bullets. Prefer current invariants over chronology. When a fact becomes obvious from source/config and no longer needs reasoning context, delete or shorten it here.
