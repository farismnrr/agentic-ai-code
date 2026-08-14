# Canonical Memory

**Last compacted: 2026-08-13.** This is the repository's **only durable memory file**. Do not add sibling Markdown memory files. Current source/config and `.agents/knowledge/` remain authoritative for implementation facts; this file keeps durable decisions, constraints, failure modes, and non-obvious reasoning.

## Repository policy and verification

- The repository intentionally has **no CI workflow** and **no unit-test suite**. Do not add normal test directories/files, package `test` scripts, or Rust `#[cfg(test)]` modules unless the user explicitly changes policy.
- Every normal local commit must pass the tracked pre-commit gate. `pnpm install` configures `.githooks`; the pre-commit hook runs `pnpm verify:commit`. Never use `--no-verify` or disable/replace the hook path.
- `pnpm verify:commit` runs repository-policy checks, agent-doc integrity, architecture-boundary checks, `pnpm lint`, and `pnpm typecheck`. A failure means do not claim the commit is verified.
- `pnpm lint` covers ESLint plus Rust formatting/Clippy. `pnpm typecheck` runs `nuxt prepare --dotenv .env.example`, direct `vue-tsc -p .nuxt/tsconfig.json --noEmit`, then warnings-denied Rust `cargo check`.
- Type-aware typescript-eslint linting is intentionally not enabled as a second type system. Type correctness belongs to the explicit generated Nuxt/Vue typecheck gate.
- Do not replace the Vue gate with bare `nuxt typecheck`; the wrapper previously missed generated-project errors.
- Production bundling is separate: run `pnpm build` when release/runtime output must be proven. Dependency/security-sensitive changes also require the relevant `pnpm audit`, `cargo audit`, and deterministic scripts.
- GitHub mergeability is not verification. Record only commands actually executed successfully.
- `scripts/check-architecture.sh` is mandatory through `pnpm verify:commit` and now rejects representative direct, type-only, transitive-facade, and API-bypass violations.
- Browser/Playwright automation may use the shared development database. Never assume browser-test data is isolated unless the environment explicitly provides isolation.

## Nuxt/application invariants

- Use **pnpm**, not Bun; use the pinned package-manager version.
- Normal Nuxt dev port is **3333**.
- Prefer `pnpm build && pnpm preview` for final runtime verification; long-lived dev state has produced stale-module failures after branch/file changes.
- Browser verification matters: successful static/build output is not proof interactive flows work.
- Authenticated SSR fetches must preserve request cookies/context.
- Do not call Nuxt composables after arbitrary `await` boundaries inside plain async orchestration; this repository has repeatedly lost Nuxt context that way.
- For screens that need related data from multiple tables, prefer one server endpoint returning the joined/ready shape instead of multiple client composables merged after awaits.
- Shared composable state intended across callers must use shared Nuxt state (`useState`).
- `#auth-utils` augmentation belongs under `shared/types/`.
- Chat persistence failures are real server failures; keep user-visible/error logging behavior and do not regress to silent errors.

## Server architecture target

Plan 031 and Plan 031A materially moved the repository toward:

```text
server/api (transport/composition)
  -> server/application (use cases/policies + application-owned contracts)
      <- server/infrastructure (DB / AI / providers / LangGraph / MCP / filesystem/network)
```

Durable final rules:

- API routes own auth, HTTP parsing/validation, dependency composition, and response adaptation—not business/persistence logic.
- Application owns business/use-case semantics and the contracts it consumes; it must not import concrete infrastructure, Drizzle/schema/useDb, H3/Nitro event types, or AI/provider/MCP implementation SDKs.
- Infrastructure implements application contracts and owns concrete persistence/external integrations.
- Avoid generic repositories/services, DI frameworks, service locators, speculative plugin systems, and one-file-per-trivial-wrapper ceremony.
- `server/utils/**` is not automatically a safe/pure layer; inspect transitive ownership and move mixed DB/provider/filesystem/network code to its real owner.
- Frontend feature components are grouped under `app/components/{chat,workspace,settings,shell}/`; root components should be genuinely shared/landing primitives. Do not split components merely by line count.

## AI/chat/tool invariants

- Prefer AI SDK/framework-native stream/tool approval behavior over hand-rolled orchestration unless a current limitation is proven.
- LangGraph/LangChain output is bridged into AI SDK UI-stream semantics; preserve valid dynamic-tool states and user-visible stream behavior.
- Chat submit/regenerate/resume semantics belong to application, not database infrastructure.
- Preserve abort/stop behavior, assistant persistence, token/context compaction, reasoning/provider options, approval allow/deny/user-approval semantics, MCP close-once cleanup, and local-terminal behavior during architecture moves.
- Local terminal is client-executed through the paired local relay path (via `ai-tools relay`); do not restore a server-side shell execution path.

## Provider/tenant security invariants

- UI filtering is never authorization. Server-side ownership is authoritative for conversation/model/provider/workspace/default-model/active-workspace/MCP/device references.
- Chat context must reassert conversation → model → provider same-user ownership, including against legacy/corrupt stored references.
- Provider API keys and secret custom-header values remain encrypted/redacted; ordinary DTOs must not return decrypted secret values.
- Editing unrelated provider fields must preserve unchanged secret headers without round-tripping plaintext.
- Legacy plaintext custom headers have an idempotent upgrade path introduced during 031A; preserve it unless a reviewed replacement is introduced.
- Provider outbound URLs are user-controlled security boundaries. Private/loopback/link-local/metadata targets and redirect targets must be blocked according to the reviewed SSRF policy.
- Authenticated provider requests reject cross-origin redirects, so `Authorization`, `x-api-key`, arbitrary custom headers, and future unknown credentials cannot be forwarded to an untrusted origin. Same-origin redirects remain bounded, scheme-safe, and revalidated by the authoritative SSRF policy.
- DNS address validation is not connection IP pinning. Do not claim complete DNS-rebinding protection unless the connection architecture actually changes.

## Rust/native-tool invariants

- Executable terminal/curl/search CLIs are Rust-owned; TypeScript sibling packages remain integration APIs, not executable fallbacks.
- The rust backend in `packages/rust-tools` must remain a Cargo Workspace with independent crates enforcing a Layered Architecture (`core`, `application`, `infrastructure`, `interfaces`, `cli`).
- The `ai-tools relay` production execution boundary is Linux + Bubblewrap + non-root runtime + explicit execution root.
- Filesystem containment is OS-namespace based; do not replace Bubblewrap with fragile shell/path parsing.
- Preserve one authoritative process-safety path: sibling binary resolution, bwrap mounts, execution root, env clearing, safe PATH, output bounds, timeout grace/kill, and process-group cleanup.
- Local relay mode is loopback-oriented. Remote mode is an OAuth Resource Server, not an Authorization Server.
- Remote auth preserves admission-before-expensive-work, trusted-proxy HTTPS policy, asymmetric JWKS verification, issuer/audience/time/signature checks, owner binding, and `relay.coding` scope.
- Current MCP target remains stateless Streamable HTTP `POST /mcp` for `2026-07-28`; client-visible tool catalog/security metadata is frozen contract material and must move deliberately.
- Plan 031A repaired the old Phase 4 discovery fixture and moved the Phase 7 catalog hash into `.agents/contracts/`. Plan 031B removed the optional `typ: JWT` requirement while retaining cheap malformed-token rejection and full verification ordering.
- Docker remains unsupported/deferred without an isolated worker/broker boundary. Never expose host Docker socket/root/privileged namespaces just to claim support.

## Historical wrong turns worth remembering

- Do not infer current architecture from old plan snapshots; several designs were superseded (inbound SSE, Node/WebSocket relay, JS executable CLIs, hardcoded provider assumptions).
- Do not weaken a production security boundary to make a deterministic fixture pass.
- Do not treat grep/source strings as behavior proof when deterministic black-box verification is practical.
- Do not create a unit-test suite merely to replace explicit deterministic security/contract scripts.
- Fixture-looking directories can contain real configuration; inspect references before deleting.

## Planning reset and active plan

- Plans `001` through `029b` are historical and summarized by Plan 030.
- Independent plans use numeric IDs; explicit lowercase-letter follow-ups remain in the same plan family and do not consume the next numeric ID.
- Plan 031 was administratively closed after its implementation pass; unresolved strict-audit work moved to Plan 031A.
- On **2026-08-13**, Plan 031A was administratively closed after its hardening pass; its remaining strict-review findings were explicitly handed to Plan 031B and are now fixed and verified there. Plan 031A remains closed and must not be reopened.
- **Plan 031B — `031b-final-architecture-security-and-release-closure.md` is CLOSED after remediation commit `bd22cc6` and final documentation closure commit `44207e5`.** Remediation commit `bd22cc6` restores application ownership boundaries: API routes consume application use cases through Nitro request context, concrete adapters are composed in the infrastructure/plugin edge, application modules own their contracts and do not import infrastructure implementations, and architecture checks cover direct, type-only, facade, and API bypasses. Provider credential containment, repository-wide layering, utility ownership cleanup, JWT compatibility, and deterministic acceptance remain implemented.
- Phase 12 authenticated/browser and live-provider flows remain explicitly unproven because the required automation/credentials were unavailable; do not mark those checklist items complete or infer them from static/build checks.
- Do not create Plan 031C merely to move an unfinished 031B blocker elsewhere. Add same-scope findings to 031B unless the user explicitly changes scope.
- **Plan 032 — `032-packages-layered-architecture-refactor.md` is CLOSED after remediation commit `8130756`.** The monolithic `rust-tools` crate was dissolved into a proper Cargo Workspace with separate `core`, `application`, `infrastructure`, `interfaces`, and `cli` crates to enforce Layered Architecture, SOLID, DRY, and KISS principles.
- **Plan 033 — `033-unified-binary-refactor.md` is CLOSED.** The separate Rust CLI tools (`curl-tool`, `relay-agent`, `searxng-search-tool`, `terminal-tool`) were consolidated into a single unified `ai-tools` binary using `clap` subcommands. The Nuxt application's Langchain tools and execution boundary were updated to call the unified binary.
- **Plan 034 — `034-server-layered-architecture-refactor.md` is CLOSED.** The Nuxt server directory was refactored to enforce Layered Architecture, DRY, KISS, and SOLID. Core logic moved to `server/core`, infrastructure adapters moved out of `server/utils` to `server/infrastructure`, and presentation routes in `server/api` and `server/routes` were decoupled from infrastructure by injecting dependencies via `event.context.application`. Strict architectural boundaries now pass `pnpm verify:commit`.
- **Plan 035 — `035-end-to-end-observability-and-secure-telemetry.md` is CLOSED** after implementation branch `feat/035-p0-observability-contract`, final commit `6b820e9` (remediation round 2; the round-1 close at `2aabaad` was reopened after the user's own review found three P1 gaps a first independent review had missed — see the plan file's status block for the full round-2 lane history). Durable final behavior:
  - `server/core/**` (not just `errors/`) must never import `server/infrastructure/**` directly. Private diagnostic logging from `server/core` flows via a `logPayload` attached to the thrown error's private `data`, read back and logged by the Nitro global error handler using the request-scoped `event.context.application.observability.logger` — capability injection through the existing Nitro-event-context pattern, not a DI framework or service locator.
  - Any fetch-consuming call site that needs same-origin trace continuity (the global `$fetch`, the AI SDK chat transport, and any future one) must go through the single `createTracedFetch()` primitive in `app/utils/trace-context.ts` — do not let a new transport/HTTP client silently bypass it the way `DefaultChatTransport` originally did.
  - Key-allowlisting (`sanitizeAttributes`) is necessary but not sufficient — `redactSecrets()`/Rust `redact_secrets()` (deterministic regex, not AI/heuristic) must also mask credential-shaped substrings *inside* allowed values (`error.message`, `stack`, free-form diagnostics), on BOTH the Loki `emit()` path and any parallel stdout/console path (`logger.ts`'s `consolaSafe()` wrapper exists specifically because these are two separate output paths that both need redaction independently).
  - Any response path that returns tool/operation results in a 200 body (MCP tool-results, chat stream tool-output, or similar) is an error-confidentiality surface just like a 5xx — raw caught-error text must never reach it; use the same generic-message + private-telemetry-diagnostic split.
  - `pnpm typecheck`'s `vue-tsc -p .nuxt/tsconfig.json` does not cover `server/**` (see [[ai-code-server-typecheck-gap]] auto-memory) — every remediation-round-2 worker still introduced at least one broken-import/missing-import bug caught only by LSP cross-checking or live acceptance testing, never by the mandated gate. Treat `pnpm verify:commit` passing as necessary, not sufficient, for `server/**` changes.
  - Every inbound Nuxt request gets a server-generated `requestId` (`server/plugins/application.server.ts`), returned as `x-request-id` on success and failure and included in every generic 5xx body.
  - `server/core/errors/http.ts`'s `problem()` never serializes `cause`/dynamic `detail`/`extra` to the client for `status >= 500`; `internal()`/`badGateway()` take a private `cause` logged server-side only. The global Nitro error handler (`server/core/errors/index.ts`) defensively re-strips those fields for any `problem()` output and forces a generic body for raw/unhandled exceptions. Rust relay applies the same split in `packages/rust-tools/infrastructure/src/transport.rs` — all `McpError::Internal`/5xx bodies are static strings; dynamic upstream/OIDC/JWKS text goes only to `tracing::error!`.
  - Application-owned observability contract: `server/application/observability/contracts.ts`'s `RequestTelemetryContext` (`withSpan`/`event`/`error`) is the only observability surface `server/application`/`server/core` may depend on; OTel SDK types stay in `server/infrastructure/observability/**`. Rust mirrors this — `core`/`application` crates have zero `opentelemetry`/`tracing-opentelemetry` dependency; init/export lives in `infrastructure`/`cli`.
  - One allowlist sanitizer per runtime (`server/infrastructure/observability/sanitize.ts`, Rust `observability.rs`'s `safe_log_field`) is the single chokepoint every log/span attribute passes through — unknown keys dropped, values length-capped, control chars stripped. Loki labels stay `{job, level}` only; `trace_id`/`span_id`/`request.id` live in the structured body, never as labels.
  - `/api/telemetry` (`server/api/telemetry.post.ts`) is authenticated, rate-limited (20 req/min/user), schema-strict (fixed event-name vocabulary in `shared/utils/telemetry.ts`, 50 records/batch, 16 attrs/record, 256/512-char caps, 64KB body cap), strips client-supplied service identity and raw user IDs, and validates/discards malformed `trace.id`/`span.id`/`request.id` rather than trusting them.
  - Outbound trace-header propagation is fail-closed by default: `otel-preload.mjs` registers a no-inject `TextMapPropagator` (extract-only) so no outbound HTTP call from the Nuxt process gets a `traceparent` injected automatically; `baggage` propagation is never registered. There is currently no Nuxt-server-initiated HTTP call to the Rust relay (the relay is reached from the user's own local machine), so there is no live first-party propagation target today — a future call site must add narrow, explicit injection rather than removing the no-inject default.
  - Real Jaeger/Loki evidence for one happy-path and one controlled-error-path journey (Nuxt and Rust) is captured under `.agents/contracts/035-evidence/`, including a canary-secret negative test (no leakage found) and `/api/telemetry` abuse tests. A fresh independent source/security review found no P0/P1 issues; informational follow-ups (a JSON-RPC 200 tool-result path at `server/api/mcp/index.ts:96-97` that still returns raw upstream error text, and no secret-pattern scrubbing inside logged error messages) are deferred, not blocking.
  - While exercising Phase 11 acceptance, several pre-existing (pre-Plan-035) bugs unrelated to this plan's scope were found and fixed because they blocked verification: broken relative imports in `server/infrastructure/mcp/client.ts`, `server/infrastructure/ai/langgraph/langgraph-tools.ts`, and `server/infrastructure/mcp/test-server.ts` (wrong `ssrf-guard`/`schema`/`createMcpClient` import paths — `pnpm typecheck`'s `vue-tsc -p .nuxt/tsconfig.json` does not cover `server/**`, so these only surfaced via `pnpm build`/runtime); missing `useDb` imports in 5 server files that left API-key auth non-functional in the production build; a missing `Bearer ` prefix strip in API-key verification; two telemetry-endpoint status-code bugs (429/400 misclassified as 500); and a `target/` entry missing from `.dockerignore` that made every `docker compose build` transfer 25GB+ of Rust build artifacts as context.
- The next independent numeric plan is **036**.

## Maintenance rule

Keep this file concise and current. Prefer durable invariants over audit chronology. Remove stale guidance when decisions change; Git history preserves forensic detail.
