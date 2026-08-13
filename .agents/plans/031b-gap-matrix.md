# Plan 031B — Phase 0 Gap Matrix

**Date: 2026-08-13**
**Baseline branch: `refactor/031b-final-architecture-security-and-release-closure`**
**Baseline commit: `8a2193fae0e068237ae0d80dd997d73c1ae2f63d`**

This is a static, source-read-only audit. No builds were run. All line numbers verified with `grep -n` / direct file reads at the baseline commit above.

---

## 1. Ownership / dependency matrix

### server/api/**

| File | Layer | Responsibility | Violates layering | Notes |
|---|---|---|---|---|
| `server/api/chat.post.ts` | api | transport + composition | N | Reference-clean route per 031A |
| `server/api/conversations/[id].delete.ts` | api | transport + direct persistence | Y | `drizzle-orm`, `../../database/schema` (conversations), `useDb()` L1-2,9 |
| `server/api/conversations/[id].put.ts` | api | transport + direct persistence | Y | same pattern, L1-2,23 |
| `server/api/conversations/index.get.ts` | api | transport + direct persistence | Y | L1-2,6 |
| `server/api/conversations/index.post.ts` | api | transport + direct persistence | Y | L1,15 |
| `server/api/sidebar.get.ts` | api | transport + direct persistence (aggregate) | Y | `workspaces, conversations` schema, `useDb()` L1-2,20 |
| `server/api/workspaces/active.put.ts` | api | transport + direct persistence | Y | `users` schema, `useDb()` L1-2,17 |
| `server/api/api-keys/index.get.ts` / `index.post.ts` / `[id].delete.ts` | api | transport + direct persistence | Y | `apiKeys` schema + `useDb()` in all three |
| `server/api/auth/{login,register,verify,reset,forgot}.post.ts` | api | transport + direct persistence + auth business logic | Y | `users`/`verificationTokens`/`workspaces` schema + `useDb()` in each |
| `server/api/devices/index.ts`, `devices/[id]/revoke.post.ts` | api | transport + direct persistence | Y | `#server/database/schema` (`userDevices`), `useDb()` |
| `server/api/mcp-servers/[id]/test.post.ts` | api | transport + direct persistence | Y | `mcpServers` schema, `useDb()` L1-2,17 |
| `server/api/mcp-servers/{index,[id]}.{get,post,put,delete}.ts` | api | transport + mixed utility (delegates to `server/utils/mcp-servers.ts`) | Y (category 4) | no direct DB import, but business/persistence hidden behind a `server/utils` facade |
| `server/api/settings.get.ts` / `settings.put.ts` | api | transport + mixed utility (`server/utils/settings.ts`) | Y (category 4) | `settings.ts` itself does direct `useDb()`/schema access, so route indirectly owns persistence |
| `server/api/settings/models-config.get.ts` | api | transport + mixed utility (`server/utils/providers.ts`, `server/utils/models.ts`) | Y (category 4) | |
| `server/api/mcp/index.ts` | api | transport + mixed utility (settings/workspaces/mcp-servers/messages utils) | Y (category 4) | widest fan-out: 5 utils imports, L4-8 |
| `server/api/providers/**` | api | not read line-by-line this pass | Unconfirmed | not in scope of the grep above (no drizzle/useDb/utils hit found) — spot-check only, treat as unaudited |
| `server/api/models/**` | api | not read line-by-line this pass | Unconfirmed | same as above |
| `server/api/workspaces/{index,[id]}.*` (excl. active.put) | api | not fully audited | Unconfirmed | no drizzle/useDb/utils.workspaces grep hit found in this pass, needs Phase 4 look |
| `server/api/auth/{token refs}` (`auth/*` using `utils/token`) | api | transport + shared helper | N (helper is a pure token util, not persistence) | `generateToken`/`hashToken` are non-DB helpers |
| `server/api/me.get.ts`, `telemetry.post.ts`, `fs/browse.get.ts` | api | not grepped for db/utils this pass | Unconfirmed | out of AE grep scope, worth a Phase 4 look |

Full DB/schema-importing route list confirmed by grep: `conversations/[id].delete.ts`, `conversations/[id].put.ts`, `conversations/index.get.ts`, `conversations/index.post.ts`, `sidebar.get.ts`, `workspaces/active.put.ts`, `api-keys/[id].delete.ts`, `api-keys/index.get.ts`, `api-keys/index.post.ts`, `auth/forgot.post.ts`, `auth/login.post.ts`, `auth/register.post.ts`, `auth/reset.post.ts`, `auth/verify.post.ts`, `devices/[id]/revoke.post.ts`, `devices/index.ts`, `mcp-servers/[id]/test.post.ts` — **17 routes**, none behind `server/application/**`.

### server/application/chat/**

| File | Layer | Responsibility | Violates layering | Notes |
|---|---|---|---|---|
| `execute-chat-turn.ts` | application | chat turn orchestration | Y | L4: `import type { ChatTurnDependencies } from '../../infrastructure/ai/chat-turn-dependencies'` — type-only but contract is infrastructure-owned |
| `history.ts` | application | turn message history | Y | L1: `import { insertUserMessage, loadHistoryMessages } from '../../infrastructure/database/chat'` (value import), L2 type import same module |
| `ownership.ts` | application | tenant ownership resolution | Y | L1-2,4 value imports from `infrastructure/database/{models,providers,chat}`; L3 value import from `../../utils/workspaces` (mixed utility, itself DB+FS) |
| `persistence.ts` | application | assistant message persistence | Y | L1: value import from `../../infrastructure/database/chat` |
| `workspace-context.ts` | application | workspace prompt/context resolution | Y | L1: `import { findUserWorkspace } from '../../utils/workspaces'` — reaches a mixed util that owns DB+filesystem |
| `local-terminal-policy.ts` | application | local-terminal tool policy | Y | L1: value import `infrastructure/database/devices`; L2: value import `infrastructure/ai/local-terminal-tool` (this one is checker-exempted, see AG) |

Every file in `server/application/chat/**` currently imports `server/infrastructure/**` and/or a DB-owning `server/utils/**` module as a **value**, not type-only. `check-architecture.sh` still passes because of the explicit exceptions described in AG below.

### server/infrastructure/**

| File | Layer | Responsibility | Notes |
|---|---|---|---|
| `database/{models,chat,devices,providers}.ts` | infrastructure | Drizzle persistence adapters | correctly placed |
| `mcp/{server-config,mcp-tools}.ts` | infrastructure | MCP client/tool construction | correctly placed |
| `ai/{ai-sdk-stream,context-compaction,langgraph-stream,local-terminal-tool}.ts` | infrastructure | AI SDK/LangGraph stream + tool builders | correctly placed |
| `ai/chat-turn-dependencies.ts` | infrastructure | **owns** `ChatTurnDependencies` contract type + composition factory | This is finding AC's exact location: contract lives here, application imports it (even type-only), L17-27 define the interface from `typeof concreteImpl` |
| `ai/providers/{anthropic-compatible,openai-compatible,vertex-ai,langgraph-model,index}.ts` | infrastructure | provider adapters | in scope for AA (see below) |

### server/utils/**

| File | Classification | Notes |
|---|---|---|
| `rate-limit.ts`, `token.ts`, `is-unique-violation.ts`, `db.ts`, `http-errors.ts`, `logger.ts`, `mailer.ts`, `otel.ts` | pure/shared server utility (mostly) | `db.ts` itself is the `useDb()` factory — arguably infrastructure, not a plain helper |
| `fs-browse.ts` | database/filesystem integration | filesystem access, should likely be `infrastructure/filesystem` |
| `mcp-servers.ts`, `mcp-client.ts` | application policy / infrastructure integration mix | business + external MCP client construction, unclear owner |
| `api-key.ts` | crypto/security helper | verify function, arguably infrastructure/security |
| `messages.ts`, `models.ts` | application policy / persistence mix (unread in full this pass) | not grepped line-by-line |
| `settings.ts` | **mixed — DB access + application dependency** | L1-2: `drizzle-orm`, `userSettings, users` schema; L6,76: `useDb()`; **L3: `import { resolveOwnedModelContext } from '../application/chat/ownership'`** — a `server/utils/**` file importing FROM `server/application/**`, i.e. the *opposite* direction of the intended API→application→infrastructure flow. This is worse than a simple mis-layered util; it is a reverse dependency the architecture checker does not check for at all. |
| `workspaces.ts` | **mixed — DB + filesystem, imported by application** | L1-2: `drizzle-orm`+schema; L3: `fs` (`node:fs/promises`); L4: `./fs-browse`; `useDb()` at L7,32,55,100,122. Imported as a value by `server/application/chat/ownership.ts` and `workspace-context.ts` |
| `providers.ts` | application policy / infrastructure integration mix | L2: imports infra `database/providers`; L5: imports infra `ai/providers/index`; L3: `./crypto`. Orchestrates provider CRUD + secret encryption + model discovery — this is a use case, arguably belongs in `server/application/providers.ts` per Plan text |
| `crypto.ts` | infrastructure/security | pure Node `crypto` + runtime config, no DB — correctly a narrow crypto helper, but path-wise should likely live under an infrastructure/security location per AF's target |
| `ssrf-guard.ts` | infrastructure/network policy | see AA/AB — currently the sole SSRF policy implementation, referenced from `server/utils` path though it is pure network security logic |
| `langgraph-tools.ts`, `langgraph-chat.ts` | unread this pass | not classified — flag for Phase 4/5 |

### packages/rust-tools/src/relay_agent

| File | Notes |
|---|---|
| `auth.rs` | `is_structurally_plausible_jwt()` at L87-126 requires `typ` field (L115-118, L126: `typ_is_jwt && alg_is_nonempty_string`) — see AH |
| `execution.rs` | L22 comment claims `[run_sandboxed]`; actual public entrypoint is `dispatch_tool_call` at L215 — see AI |
| `transport.rs` | L870 calls `execution::dispatch_tool_call(&tool, &call.arguments, &state.config)` — single call site, consistent with "one authoritative path" claim once the comment is fixed |

---

## 2. Findings AA–AM

### AA — Provider redirect credential leakage (P0)
**Status: CONFIRMED**
Evidence: `server/utils/ssrf-guard.ts` L72: `const SENSITIVE_REQUEST_HEADERS = ['authorization', 'cookie', 'proxy-authorization']`; stripped only on cross-origin/downgrade at L119-123. `x-api-key` and arbitrary `customHeaders` names are not in this denylist, so they are forwarded unchanged across an origin change. Matches the plan's exact description of the baseline behavior — code is materially unchanged from the cited `b43f1fe9` baseline.

### AB — phase9-ssrf-redirect-guard.sh doesn't exercise a real redirect hop
**Status: CONFIRMED**
Evidence: `scripts/phase9-ssrf-redirect-guard.sh` L83: `await safeFetch(\`http://127.0.0.1:${redirectPort}/public\`)`. The initial URL is itself `127.0.0.1` (loopback), which `assertSafeUrl` rejects at the very first hop (`isDisallowedIPv4` L14: `a === 127`) before the request ever reaches the fixture server's redirect response. The script's "redirect-to-loopback rejection" assertion (L88) passes because of the initial-URL rejection, not because the redirect branch (L118-124 in ssrf-guard.ts) executed. No assertion anywhere in the script proves the follow-up fetch was/was not called. Matches the plan's description exactly.

### AC — Application contracts owned by infrastructure
**Status: CONFIRMED**
Evidence: `server/application/chat/execute-chat-turn.ts` L4: `import type { ChatTurnDependencies } from '../../infrastructure/ai/chat-turn-dependencies'`. The contract itself, `server/infrastructure/ai/chat-turn-dependencies.ts` L17-27, defines every field as `typeof concreteImplementation` (e.g. `resolveModelConfig: typeof resolveModelConfig`), i.e. derived from infrastructure implementation types. Import is type-only, but ownership and shape both originate in infrastructure, exactly as described.

### AD — Application calls concrete DB/AI infrastructure directly
**Status: CONFIRMED**
Evidence (value imports, not type-only):
- `server/application/chat/history.ts` L1: `import { insertUserMessage, loadHistoryMessages } from '../../infrastructure/database/chat'`
- `server/application/chat/persistence.ts` L1: `import { cacheLastMeasuredTokens, findLastMessage, insertAssistantMessage, updateAssistantMessage } from '../../infrastructure/database/chat'`
- `server/application/chat/ownership.ts` L1-2,4: value imports from `infrastructure/database/{models,providers,chat}`
- `server/application/chat/local-terminal-policy.ts` L2: value import from `infrastructure/ai/local-terminal-tool`
- workspace context: `server/application/chat/workspace-context.ts` L1 and `ownership.ts` L3 both import `findUserWorkspace` from `../../utils/workspaces`, which itself does `useDb()`/Drizzle (`server/utils/workspaces.ts` L1-2,7,32,55,100,122) and raw filesystem access (L3-4, `node:fs/promises`, `./fs-browse`).
All exactly as described in the plan; none of this has been remediated yet.

### AE — Layering must be checked beyond chat.post.ts
**Status: CONFIRMED**
Evidence: 17 routes under `server/api/**` import `drizzle-orm`/`../database/schema`/`#server/database/schema` and call `useDb()` directly (full list in the matrix above — `conversations/*`, `sidebar.get.ts`, `workspaces/active.put.ts`, `api-keys/*`, `auth/*.post.ts`, `devices/*`, `mcp-servers/[id]/test.post.ts`). An additional 6+ routes (`mcp-servers/{index,[id]}.*`, `settings.get.ts`, `settings.put.ts`, `settings/models-config.get.ts`, `mcp/index.ts`) route through mixed `server/utils/**` facades that themselves own persistence. `server/application/**` contains only the `chat` feature — no `conversations.ts`, `workspaces.ts`, `providers.ts`, or `settings.ts` application modules exist yet. This is a repository-wide gap, not just a chat-route issue.

### AF — server/utils/** mixed ownership
**Status: CONFIRMED**
Evidence: see the utils classification table above. Notably:
- `server/utils/workspaces.ts` — DB (`drizzle-orm`, schema, `useDb()`) + filesystem (`node:fs/promises`, `fs-browse`) in one file, imported directly by two `server/application/chat/**` files.
- `server/utils/settings.ts` — DB access (L1-2,6,76) **and** a reverse import from `server/application/chat/ownership.ts` (L3: `import { resolveOwnedModelContext } from '../application/chat/ownership'`). This is a `server/utils` → `server/application` dependency, which is backwards relative to the intended API→application→infrastructure direction and is not caught by any existing architecture check.
- `server/utils/providers.ts` — orchestrates provider CRUD + secret encryption + model discovery by importing infra `database/providers` and infra `ai/providers/index` directly; this is a use case sitting in `utils` rather than `application`.
- `server/utils/crypto.ts` — the one genuinely narrow file: pure `node:crypto` + runtime config, no DB/business logic.
- `server/utils/ssrf-guard.ts` — pure network security policy, no DB, but path-wise still under the generic `utils` bucket the plan wants collapsed.

### AG — check-architecture.sh loopholes
**Status: CONFIRMED**
Evidence: `scripts/check-architecture.sh`:
- L36: `ai_pkg_violations=... | grep -vE ':[0-9]+:import type '` — explicit type-only exception for `ai`/`@ai-sdk`/`@langchain` imports.
- L51: `infra_ai_mcp_violations=... | grep -vE ':[0-9]+:import type ' | grep -v 'infrastructure/ai/local-terminal-tool'` — explicit type-only exception **and** an explicit `local-terminal-tool` infrastructure carve-out (matches the plan's cited loophole verbatim).
- The checker only scans for Drizzle-schema/`drizzle-orm`/`ai`/`@ai-sdk`/`@langchain`/`infrastructure/ai`/`infrastructure/mcp` patterns; it does **not** check `server/application` → `server/infrastructure/database/**` value imports at all (confirmed AD violations above are not caught), nor does it check `server/api/**` for direct DB/schema imports (confirmed AE violations above are not caught), nor the reverse `server/utils` → `server/application` dependency found in AF. The checker is materially narrower than the violations that exist in current source, i.e. it can be green while AC/AD/AE/AF are all still true.

### AH — Cheap JWT precheck requires typ:"JWT"
**Status: CONFIRMED**
Evidence: `packages/rust-tools/src/relay_agent/auth.rs` L115-118:
```rust
let typ_is_jwt = header_json
    .get("typ")
    ...
    .map(|typ| typ.eq_ignore_ascii_case("JWT"))
```
L126: `typ_is_jwt && alg_is_nonempty_string` — the function returns `false` (rejects) whenever `typ` is absent or not "JWT", exactly the over-strict behavior described.

### AI — Execution comment names nonexistent run_sandboxed
**Status: CONFIRMED**
Evidence: `packages/rust-tools/src/relay_agent/execution.rs` L22: `/// shared process-safety path in [\`run_sandboxed\`] so no tool can bypass it.` — `grep -rn "run_sandboxed"` across `relay_agent/` returns only this one doc-comment reference; the actual shared entrypoint is `dispatch_tool_call` (L215 of `execution.rs`, called once from `transport.rs` L870). No `run_sandboxed` symbol exists anywhere.

### AJ — Tenant ownership re-audit (spot check only, not full)
**Status: CONFIRMED as still-open surface, not yet fully re-verified**
Spot evidence: `server/application/chat/ownership.ts` centralizes conversation→user, model→provider, and workspace ownership checks (`findUserModel` L25, `findUserProvider` L26, `findUserWorkspace` L40 via utils, `findUserConversation` L54), all delegating to infrastructure functions with `userId` as an explicit parameter (`server/infrastructure/database/models.ts` L27 `findUserModel(userId, id)`, `providers.ts` L28 `findUserProvider(userId, id)`) — the ownership-scoping shape itself looks intact at a glance. However, this was only a spot check per the task instructions; a full two-user BOLA matrix (Plan Phase 8) has not been run and is out of scope for Phase 0. Not marking ALREADY-RESOLVED because the plan explicitly requires this be re-verified fresh after the AC-AF architecture moves happen, which have not happened yet.

### AK — Provider secret lifecycle re-audit (spot check only)
**Status: not independently re-verified this pass — deferred**
`server/utils/providers.ts` calls `encryptSecret` (from `./crypto`) before delegating to infra `insertUserProvider`/`updateUserProvider`, consistent with encryption-at-rest still being wired up, but DTO/redaction/log-safety behavior was not read this pass. Flag for a dedicated Phase 8 check rather than guessing a status here.

### AL — Frontend foldering
**Status: ALREADY-RESOLVED / matches description**
Evidence: `find app/components -maxdepth 2 -type d` returns exactly `app/components/chat`, `app/components/shell`, `app/components/settings`, `app/components/workspace` — matches the plan's description. `AppSidebar.vue` and `AppSidebarWorkspaceDialogs.vue` live under `app/components/shell/`, and a `useSidebarData.ts` composable exists separately, consistent with sidebar data-fetching being decomposed out of the component itself. No content-level regression audit was performed (out of scope for Phase 0); structure-level claim holds.

### AM — Stale doc/comment claims
**Status: ALREADY-RESOLVED**
Evidence: `.agents/knowledge/project.md` L60: *"at the Plan 031B baseline this direction is **not yet fully closed in source**... Do not describe the architecture as fully closed until [...] 031b-final-architecture-security-and-release-closure.md ... reaches its Definition of Done."* L67: *"Plan 031B owns closing current false-green loopholes"*. `.agents/memories/README.md` L79 describes the JWT `typ` issue as an open Plan 031B item, not a closed Phase 4/7 finding. `grep -rn "closed by Plan 031A\|10/10"` across `AGENTS.md`, `.agents/knowledge/project.md`, `.agents/memories/README.md` returns no matches. The only "closed" hit is `AGENTS.md` L15, which refers to historical plans through 029b being compacted/closed in Plan 030 — unrelated to 031A/031B. Docs already reflect the truthful open state at this baseline; AM's described staleness has already been corrected (likely during the b43f1fe9 Phase 13 honesty commit noted in git log).

---

## 3. Summary

- **CONFIRMED:** AA, AB, AC, AD, AE, AF, AG, AH, AI (9)
- **Spot-checked / deferred to later phase (not a final status):** AJ, AK (2)
- **ALREADY-RESOLVED:** AL, AM (2)
- **SUPERSEDED:** none

No finding was found to be already fixed by source changes beyond docs (AL/AM). AA–AI all require Plan 031B implementation work exactly as scoped.
