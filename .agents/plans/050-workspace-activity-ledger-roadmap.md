# Plan 050 — Workspace Activity Ledger and Reliable Agent Execution

**Status:** IMPLEMENTED — LOCAL GATES PASSED; LIVE ACCEPTANCE UNPROVEN
**Created:** 2026-08-26

## Goal

Deliver an industry-standard, persistent, per-workspace activity ledger that truthfully records every workspace-scoped operation mediated by `ai-tools relay`, regardless of whether the caller is the first-party Nuxt application, the paired/local-terminal path, external MCP client through remote MCP, another MCP client, or a task/background execution path.

At the same time, simplify and harden the coding execution surface: remove the provider-specific `agent_delegate`/coding-CLI delegation feature, and let agents choose bounded execution timeout plus sync/async behavior for eligible tools so long-running work can be polled/resumed instead of timing out and being restarted from scratch.

The product target is a modern coding-agent workspace history plus a predictable long-running execution model: users can open **Logs** under a workspace, see one chronological stream of reads/searches/commands/Git/code operations and mutations, inspect actor/status/duration/change evidence, expand durable historical diffs for supported structured mutations, and rely on accepted asynchronous work continuing independently of one short MCP request lifetime.

Plan 050 is intentionally one plan file. Its implementation is divided into phases rather than child plans so design, progress, closure evidence, and source of truth remain centralized.

## Success criteria

Plan 050 is complete only when:

1. every relay-mediated workspace operation resolves to one authoritative canonical workspace root and one owned Nuxt workspace binding without trusting a client-supplied workspace UUID;
2. first-party Nuxt MCP, paired/local-terminal calls, direct external MCP client remote MCP, generic MCP clients, and sync/async task/job execution all use the same activity contract;
3. the relay durably records a workspace operation locally before execution when activity logging is configured as required, so Nuxt/network outages do not create silent gaps;
4. relay delivery to Nuxt is authenticated, idempotent, retryable, bounded, and crash-safe;
5. PostgreSQL stores one chronological workspace activity history with ownership-enforced cursor queries, bounded retention, and explicit deletion semantics;
6. `file_edit`, `file_write`, and `apply_patch` preserve accurate historical before/after diffs with additions/deletions and affected paths for supported text mutations;
7. opaque process/background task execution is represented truthfully with lifecycle plus bounded change evidence rather than fabricated exact provenance;
8. activity payloads containing source/diffs are strictly separated from Plan-035/039J OpenTelemetry/Loki telemetry and encrypted at rest with purpose-separated key material;
9. the UI exposes **Logs** under each workspace with live/near-live updates, filters, lazy details/diffs, explicit integrity/completeness states, and no hidden reasoning;
10. deterministic reliability/security acceptance proves no silent loss, duplication, misattribution, cross-tenant leakage, secret leakage, or unsafe failure under restart/outage/quota/concurrency conditions;
11. live acceptance proves local terminal, first-party Nuxt MCP, and direct external MCP activity where the relevant authenticated path is available;
12. final independent review has zero unresolved P0/P1 security, integrity, confidentiality, or architecture findings;
13. the provider-specific `agent_delegate` MCP surface and its external coding-CLI execution plumbing are removed from current catalogs/runtime/config/docs/tests, while unrelated platform-native sub-agent orchestration remains unaffected;
14. agents can request the timeout they need for eligible commands/tools within explicit operator/tool safety ceilings, without the current arbitrary Primary-profile 30-second `terminal_exec` ceiling;
15. eligible long-running tools support explicit `execution_mode` selection (`sync`, `async`, `auto`), with async execution returning a resumable/cancellable task identity instead of holding one RPC open until completion;
16. accepted asynchronous work is not restarted merely because a client request, UI session, or network connection timed out; retries/reconnects converge on the same logical task/activity wherever an idempotency identity is available.

## Verified current state

Verified in the working tree on 2026-08-26:

- Nuxt persists workspaces in `server/database/schema.ts` with `id`, `userId`, `name`, `path`, and timestamps. `server/infrastructure/database/workspaces.ts` validates filesystem existence but does not define a relay activity binding.
- The Rust relay owns canonical multi-root authorization through `WorkspaceAllowlist` in `packages/rust-tools/core/src/workspace_path.rs`; it knows canonical roots, not Nuxt workspace UUIDs/users.
- `packages/rust-tools/infrastructure/src/transport/tools.rs` is the MCP `tools/call` transport choke point and already owns auth context, pre/post hooks, request identity, and bounded monotonic timing.
- `packages/rust-tools/application/src/execution.rs::dispatch_tool_call` is the shared application execution path for native workspace, Git, code/LSP, search, terminal, HTTP/search, and the legacy provider-delegation tool.
- Successful native file mutations already emit `AfterFileChange`, but that hook is intentionally metadata-only and is not historical diff storage.
- `file_edit` already has both `source` and `updated` text before atomic commit; its public result currently returns only path/replacement/change metadata.
- `file_write` returns create/overwrite/byte metadata and does not currently preserve historical before/after content.
- Plan 035/039J observability explicitly forbids source/patch/tool-result contents and private absolute paths from OTel/Loki. Existing telemetry therefore cannot become the product activity database.
- Modern MCP request metadata supports `io.modelcontextprotocol/clientInfo`. The first-party server MCP client sends `{ name: 'ai-code', version: '1.0.0' }`; `app/composables/useRelayAgent.ts` currently omits `clientInfo` on paired/local relay requests.
- Remote OAuth claims contain `sub` and `client_id`, but those identities are distinct from Nuxt `users.id`; `clientInfo` is display metadata, never authorization.
- `local_terminal` executes through `app/composables/chat/local-tool-controller.ts` → `app/composables/useRelayAgent.ts` → loopback relay `tools/call terminal_exec`; it must be logged by the same relay recorder, not by a browser-only special case.
- `terminal_exec` already accepts `timeout_ms`, but `packages/rust-tools/application/src/execution/requests.rs` currently imposes a Primary-profile `1..30000 ms` ceiling and tells callers to use `terminal_job_start` for longer work. This profile-specific ceiling is a usability failure for agent-directed execution and is removed/replaced by reviewed operator/tool bounds in this plan.
- The canonical catalog currently marks `terminal_exec`, `http_fetch`, `web_search`, and legacy `agent_delegate` as task-capable, but `packages/rust-tools/infrastructure/src/transport/tools.rs` only starts MCP task-backed tool calls when the relay is in the Full profile and the client advertises the Tasks extension. Primary currently omits the Tasks extension during initialize.
- `start_tool_task` currently supports `terminal_exec`, `http_fetch`, `web_search`, and `agent_delegate`; `terminal_job_start/get/cancel` provide a separate terminal-only async path. Plan 050 must converge new agent-facing long-running execution on one explicit sync/async policy rather than requiring agents to guess which special tool to call after a timeout.
- `agent_delegate` is still present in the current MCP catalog/profile filtering, execution dispatch, hook/effect policy, provider capability discovery, acceptance scripts, and operator docs. Plan 050 intentionally removes this provider-specific coding-CLI delegation surface instead of extending it.
- Hosted Nuxt and external MCP clients may consume the same public MCP resource while authenticating independently.
- Repository policy has no CI and no unit-test suite; deterministic acceptance/security scripts plus `pnpm verify:commit` are the required local verification model.
- Plan 050 must stay isolated from unrelated predecessor work. If implementation is already active on the dedicated Plan-050 branch/worktree, preserve and reconcile that partial work against this updated plan rather than resetting it; otherwise start from current `main` according to repository policy.

## Scope

### In scope

- per-workspace activity identity and binding;
- one versioned activity contract and safe operation summaries;
- relay-local durable journal/outbox;
- authenticated relay-to-Nuxt ingestion;
- first-party Nuxt/local-terminal caller metadata;
- direct remote MCP and generic-client activity;
- synchronous and task/job lifecycle capture;
- removal of the provider-specific `agent_delegate` coding-CLI delegation surface and dead provider-only plumbing;
- agent-selected bounded execution timeout for eligible tools;
- explicit `sync`/`async`/`auto` execution selection for reviewed task-capable tools, using resumable/cancellable task identity rather than special-case restart behavior;
- structured workspace/Git/code/process/background-task operation summaries;
- exact historical structured mutation diffs;
- bounded change evidence for opaque writers;
- encrypted relay spool and PostgreSQL payload persistence;
- PostgreSQL schema/query/retention/deletion;
- workspace Logs UI with live updates, filters, detail, and diff review;
- confidentiality, restart, outage, quota, concurrency, idempotency, and adversarial acceptance;
- operator/user documentation and configuration.

### Out of scope

- hidden model reasoning or chain-of-thought;
- reintroducing provider-specific external coding-CLI delegation or parsing provider-private transcripts after `agent_delegate` removal;
- replacing OTel/Loki operational telemetry;
- persisting arbitrary terminal stdout/stderr, raw MCP request/response bodies, prompts/model messages, auth headers, cookies, environment variables, or provider secrets;
- syscall-level surveillance of arbitrary non-relay processes;
- claiming cryptographic tamper-proof/compliance-grade immutability without an independent trust/key store;
- production deployment/restart or irreversible deletion without separate authorization;
- introducing a unit-test framework or remote CI workflow.

## Architecture and product decisions

### AD-001 — Product activity is not telemetry

Workspace activity is user-facing product data. OTel/Loki remains sanitized operational telemetry and continues to reject source/patch/tool-result content. Activity diffs or raw source-bearing payloads must never be copied into telemetry attributes, logs, spans, or errors.

### AD-002 — Relay is the observation authority

Workspace activity is recorded at the relay execution boundary, not reconstructed by Nuxt/browser UI after the fact. Browser state may configure/view activity, but it is never the authoritative recorder.

### AD-003 — Durable local journal before remote delivery

The relay owns a bounded encrypted local journal/outbox. In **required** mode, a workspace-scoped operation must obtain a durable local `started` record before execution. Sink unavailability does not silently drop activity: records remain queued and are retried idempotently.

If a terminal outcome cannot be journaled after execution, the durable start remains evidence and recovery marks the operation interrupted/unknown rather than pretending success.

### AD-004 — Nuxt/Postgres is the product read model

The relay journal is an execution-side durability/outbox boundary, not the UI database. Nuxt/Postgres owns the user-facing history, workspace ownership, retention, pagination, encrypted payload access, and server-shaped APIs.

### AD-005 — Workspace identity is canonical-root based

Relay operations resolve their authoritative containing workspace root through the existing `WorkspaceAllowlist`/path policy. Nuxt maps a relay-derived opaque root fingerprint to an owned workspace. MCP clients cannot choose a Nuxt `workspaceId` in activity payloads.

Nested `cwd` values map to their containing authorized root. Global relay administration that has no meaningful workspace scope is not falsely attached to the active workspace.

### AD-006 — Actor attribution is truthful and layered

Persist separate facts for transport/source channel, bounded client-reported `clientInfo`, source identity/binding, and optional opaque OAuth client fingerprint when justified. `clientInfo` is display metadata only.

If identifying metadata is absent or untrusted, the UI shows **External MCP client** rather than guessing. A `external MCP client` label is used only when actual reviewed metadata supports it.

### AD-007 — Exact diffs only where they are provable

Structured mutations use relay-owned before/after state to produce exact historical diffs. Opaque terminal/provider subprocesses do not claim exact historical diffs unless the implementation can actually prove them. They receive truthful lifecycle plus bounded mutation summary/evidence.

### AD-008 — Sensitive payloads are encrypted and lazy-loaded

List/filter metadata stays bounded and non-secret. Source-bearing diff payloads are encrypted separately with purpose-separated application key material. Relay spool payloads use a relay-local protected key outside workspace roots. List/live endpoints never return full diff plaintext.

### AD-009 — Idempotency is end-to-end

Each relay installation has a stable source identity; each logical operation has a stable collision-resistant `activity_id` and source ordering facts. Nuxt ingestion enforces uniqueness and legal lifecycle transitions so retries cannot duplicate entries or regress terminal states.

### AD-010 — Retention is explicit and bounded

Define documented default retention (initial target: 90 days, subject to implementation review), bounded configurable options, and explicit clear-history behavior. Retention deletes metadata and encrypted payload together. Workspace deletion cascades history. Acknowledged relay outbox rows are pruned independently after a shorter bounded local TTL.

### AD-011 — Do not fake tamper-proof compliance

V1 uses append-oriented durable journaling, authenticated ingestion, immutable operation identity, idempotent transitions, checksums, and no ordinary historical edit API. Do not market it as WORM or cryptographically tamper-proof.

### AD-012 — Activity failure behavior is explicit

`off` mode preserves backward compatibility. In `required` mode, inability to durably journal a workspace operation before execution fails closed. Nuxt sink outage alone does not block execution after local durability succeeds.

### AD-013 — Remove provider-specific coding-CLI delegation

The current `agent_delegate` MCP feature is removed from Full/Primary catalogs and runtime execution. Provider-specific external coding-CLI discovery, allowlisting, auth-root/sandbox mounting, configuration, docs, and current acceptance contracts that exist only to support `agent_delegate` must be deleted or retired after dependency review. Do not keep a hidden compatibility alias that still launches those CLIs.

This removal does **not** prohibit the coding agent implementing this plan from using platform-native sub-agents for bounded parallel work/review, and it does not remove unrelated first-party multi-agent/sub-agent orchestration unless source inspection proves it is merely another entrypoint into the same deprecated provider-CLI delegation path.

### AD-014 — Agent-selected timeout, operator-bounded

Eligible blocking tools expose a clear requested execution timeout. The agent/client chooses `timeout_ms` based on the operation it intends to run; the relay enforces reviewed tool-specific and operator-configured maxima. Primary must not impose an arbitrary 30-second ceiling solely because of profile choice. `timeout_ms = 0` semantics must be explicit: deadline-free only when operator policy permits it; otherwise a configured operator maximum remains authoritative.

Tool execution timeout, MCP/HTTP request deadline, background task lifetime, retention TTL, and cancellation timeout are separate concepts and must not be conflated.

### AD-015 — Explicit sync/async/auto execution with resumable task identity

Task-eligible tools expose one bounded `execution_mode` preference: `sync`, `async`, or `auto` (default reviewed during implementation). `sync` waits for the direct tool result subject to requested/effective timeout. `async` returns promptly with a standard MCP task identity and continues execution independently of the initiating RPC. `auto` follows a deterministic documented server policy based on task eligibility/client capability and must never surprise callers with silent duplicate execution.

The relay should prefer the MCP Tasks lifecycle instead of proliferating operation-specific `*_job_start` tools. Existing `terminal_job_start/get/cancel` may remain temporarily for backward compatibility, but first-party and documented agent workflows should converge on standard task-backed execution. Primary should advertise the same server Tasks capability when the runtime can actually honor it.

If `async` is explicitly requested but the client does not support Tasks, fail with a clear bounded capability error rather than silently downgrading to sync. For mutating async operations, use a stable bounded idempotency/execution identity where needed so a lost response/retry can converge on the accepted task instead of executing the mutation twice.

## Phase overview

| Phase | Goal | Exit criterion |
| --- | --- | --- |
| PHASE-01 | Activity contract, workspace identity, and caller facts | one bounded versioned contract and authoritative workspace/source model are frozen |
| PHASE-02 | Relay durable journal and reliable execution runtime | required-mode durability, legacy delegation removal, agent-selected timeout, sync/async task lifecycle, local capture, retry/recovery work without telemetry leakage |
| PHASE-03 | Mutation diffs and change evidence | structured text mutations preserve exact history; Git/opaque writers report truthful bounded evidence |
| PHASE-04 | Nuxt ingestion, persistence, retention, and query | authenticated idempotent ingestion maps only to owned workspaces and serves encrypted cursor history |
| PHASE-05 | Workspace Logs UI and live review | every workspace exposes a polished durable timeline with lazy details/diffs and resumable updates |
| PHASE-06 | Reliability, security, live acceptance, and closure | composed outage/security/live matrix passes and docs/source/plan truth is reconciled |

---

# PHASE-01 — Activity contract, workspace identity, and caller facts

**Goal:** freeze one vendor-neutral activity vocabulary and identity model before persistence/UI implementation.

## TASK-001 — Define the versioned activity envelope

**Outcome:** one application-owned contract covers synchronous, async task/job, denied/failed, and background operations.

**Planned files:**
- create/modify `packages/rust-tools/application/src/activity.rs` or a cohesive `activity/` module if maintainability requires it;
- modify `packages/rust-tools/application/src/lib.rs`;
- create `.agents/contracts/050-activity-event-v1.json`.

**Required contract facts:**
- contract version independent of MCP protocol version;
- stable `activity_id` reused across lifecycle transitions/retries;
- stable source ID plus source sequence/order facts;
- lifecycle/status: `started`, `running`, `ok`, `error`, `denied`, `cancelled`, `interrupted`;
- canonical tool/category/effect classes;
- relay-derived workspace-root fingerprint/binding candidate, never Nuxt UUID authority;
- actor/source/channel facts;
- occurrence/ingestion timestamps plus monotonic duration where available;
- bounded relative target summary;
- safe result/error classification without raw internal errors;
- change evidence class (`exact`, `summary`, `unavailable`, `not_applicable`) plus payload-reference/completeness metadata;
- strict per-field/string/path/list/payload bounds;
- safe execution facts where relevant: requested/effective timeout, requested/effective execution mode, and task correlation identity without raw command/result payload duplication.

**Rules:**
- reuse existing canonical tool IDs and effect taxonomy;
- never serialize raw MCP arguments/results, prompts, auth, env, arbitrary stdout/stderr, or raw Error objects;
- unknown contract versions fail closed at ingestion;
- ordering never trusts wall clock alone;
- illegal lifecycle regressions are rejected.

**Validation:** deterministic fixture round-trips valid events and rejects unknown versions, oversized fields, forbidden data classes, and illegal state transitions.

**Commit boundary:** `feat(activity): define workspace activity contract`

## TASK-002 — Define per-tool safe presentation facts

**Outcome:** entries remain useful without becoming a raw argument/result archive.

**Rules by family:**
- filesystem: relative path/range/counts, never read contents;
- search: bounded redacted query summary, scope/result count, never raw result bodies;
- terminal/process: executable plus bounded redacted argv summary, cwd-relative scope, exit/result class, no arbitrary stdout/stderr;
- Git: operation/ref/relative paths/result class, never credential-helper output;
- LSP/code: operation/path/symbol summary/result count, no source bodies or unsafe diagnostic text;
- background/task execution: task state, execution mode, requested/effective timeout, effect scope, and change-evidence class without private transcript/reasoning;
- network-only tools without resolvable workspace context remain unscoped.

**Validation:** representative tool fixtures from every category prove forbidden fields are absent.

**Commit boundary:** `feat(activity): classify relay operation summaries`

## TASK-003 — Resolve authoritative workspace scope

**Outcome:** every workspace-scoped event uses the same path authority as execution.

**Planned files:**
- reuse/modify `packages/rust-tools/core/src/workspace_path.rs` only where necessary;
- add activity scope adapter in the application activity module;
- reuse workspace/Git/code/request builders that already canonicalize `cwd`/paths.

**Requirements:**
- omitted `cwd` resolves exactly as the tool actually executes;
- nested `cwd`/path resolves to `WorkspaceAllowlist::containing_root` after canonical validation;
- protected-path/containment policy is reused, not reimplemented in weaker logging code;
- task/job activity inherits the original workspace rather than re-guessing during polling/cancel;
- async execution retains the original workspace/activity identity across polling, reconnect, cancellation, and completion;
- define workspace-add/remove/control-event scoping explicitly;
- generate a stable cross-language root fingerprint from canonical root bytes; it must not enter OTel/Loki.

**Validation:** primary root, dynamic sibling, nested cwd, missing cwd, revoked root, symlink, outside-root, and multiple-root cases.

**Commit boundary:** `feat(activity): resolve authoritative workspace scope`

## TASK-004 — Capture truthful caller/source identity

**Outcome:** UI can distinguish first-party/local/remote/external sources without using display labels for authorization.

**Planned files:**
- `packages/rust-tools/infrastructure/src/transport/tools.rs`;
- `packages/rust-tools/interfaces/src/mcp.rs` as needed;
- `app/composables/useRelayAgent.ts`;
- review `server/infrastructure/mcp/client.ts`.

**Requirements:**
- extract bounded `RequestMeta.client_info` for modern MCP calls;
- persist transport/auth source facts separately from client-reported label;
- preserve first-party server `MCP_CLIENT_INFO`;
- add one shared AI Code `clientInfo` constant to local relay `tools/call`, job, and lifecycle requests;
- missing/invalid clientInfo → `External MCP client`;
- malicious/spoofed clientInfo cannot widen authority or select workspace;
- OAuth client identity, if stored, is opaque/bounded and never token material.

**Validation:** first-party server, paired/local relay, external client with metadata, external client without metadata, and spoofed metadata fixtures.

**Commit boundary:** `feat(activity): capture truthful relay caller metadata`

## TASK-005 — Freeze PHASE-01 acceptance

**Planned files:**
- `.agents/contracts/050-activity-event-v1.json`;
- `scripts/verify-050-activity-contract.ts`;
- `package.json` `verify:050` may compose phase-specific scripts while keeping Plan 050 as one plan.

**Required cases:** contract versioning, state legality, field bounds, no raw arguments/results/auth/env/prompts, stable root fingerprint, generic actor fallback, and strict telemetry separation.

**Phase exit criteria:**
- [x] event contract v1 is explicit, bounded, and versioned;
- [x] canonical effect/tool vocabularies are reused;
- [x] workspace scope comes from relay authority, not client UUIDs;
- [x] local relay sends clientInfo consistently;
- [x] actor metadata is separate from authorization;
- [x] exact/summary/unavailable evidence classes are explicit;
- [x] deterministic contract acceptance passes.

---

# PHASE-02 — Relay durable journal and reliable execution runtime

**Goal:** make the relay the crash-safe observation authority, remove the legacy provider-CLI delegation surface, and give agents a predictable bounded sync/async execution model so long-running work can continue without being restarted after short request timeouts.

## TASK-006 — Add validated activity configuration

**Planned files:** `packages/rust-tools/core/src/config.rs`, `packages/rust-tools/core/src/config/cli.rs`, `.env.example`, later operator docs.

**Requirements:**
- explicit `off` and `required` modes; never overload telemetry flags;
- reviewed config for state directory, HTTPS sink URL, source token, spool quota, acknowledged-record local retention;
- default state under owner XDG/local state, never workspace/execution root;
- sink is operator config, never tool/request input;
- token redacted from presentation/logging;
- exporter networking belongs to relay process and does not widen Bubblewrap terminal/agent network authority.

**Commit boundary:** `feat(activity): add relay activity configuration`

## TASK-007 — Persist source identity and local encryption key

**Outcome:** one relay installation has restart-stable source identity and a protected payload key.

**Requirements:**
- generate source ID and random 256-bit local payload key once;
- state dir `0700`, key/state files `0600` on Linux;
- no symlink/final-target escape;
- key separate from Nuxt ingestion token;
- versioned local state format and bounded recovery errors.

**Validation:** fresh start/restart, permissions, symlink/unsafe path, wrong/lost-key cases.

**Commit boundary:** `feat(activity): persist relay activity source identity`

## TASK-008 — Add encrypted SQLite journal/outbox

**Outcome:** start/outcome records and payload references survive crash/restart and can be delivered idempotently.

**Planned files:**
- reviewed Rust SQLite dependency if needed;
- `packages/rust-tools/infrastructure/src/activity/{mod,journal,crypto}.rs` or equivalent cohesive shape.

**Requirements:**
- prefer mature SQLite rather than an ad-hoc lossy log if build/audit on supported Linux target is acceptable;
- transactional metadata + encrypted payload reference;
- WAL/durability settings reviewed for the single relay process;
- source/activity ID, sequence, lifecycle, safe metadata, encrypted payload/checksum, delivery state, attempts, timestamps;
- plaintext source-bearing payload absent from DB/WAL;
- indexes for undelivered order and stale-running recovery;
- quota never deletes unacknowledged rows to create capacity.

**Validation:** abrupt reopen, plaintext canary scan, quota/full/corrupt journal.

**Commit boundary:** `feat(activity): add durable relay activity journal`

## TASK-009 — Compose a narrow application recorder boundary

**Outcome:** execution code does not depend on SQLite/HTTP/OTel implementation types.

**Requirements:**
- application-facing start/outcome/payload append contract plus no-op/off implementation;
- infrastructure composes concrete recorder/exporter;
- required-mode journal failure is distinguishable from tool failure;
- application crate remains free of SQLite/reqwest/OTel implementation dependencies.

**Commit boundary:** `refactor(activity): compose relay activity recorder`

## TASK-010 — Capture synchronous tool lifecycle

**Planned files:** `packages/rust-tools/infrastructure/src/transport/tools.rs`, plus execution owners only where authoritative activity facts must surface.

**Requirements:**
- resolve workspace using PHASE-01 policy;
- durably journal `started` before workspace execution in required mode;
- record approval-required/blocked/denied without approval tokens/raw hook payloads;
- record success/error/cancel and monotonic duration;
- existing `relay.tool.dispatch` telemetry remains payload-free;
- auth/validation failures with no safely resolvable workspace remain security telemetry, not fabricated workspace history.

**Commit boundary:** `feat(activity): record synchronous relay operations`

## TASK-011 — Generalize task/job lifecycle and cancellation

**Requirements:**
- bind task/job ID to originating activity/workspace/source facts;
- one logical activity across queued/running/completed/failed/cancelled;
- `tasks/get` polling does not create duplicate primary activity rows;
- cancellation is a control fact without conflicting terminal outcomes;
- preserve actual execution duration separately from request/RPC duration;
- restart marks orphaned nonterminal activity interrupted/unknown;
- accepted async work survives initiating request disconnect/timeout and remains queryable/cancellable by stable task identity;
- retry/reconnect must converge on an existing accepted async task when the client supplies/retains the required idempotency identity rather than blindly starting a duplicate.

**Validation:** start→poll→complete, start→cancel, lost response/retry, reconnect, and restart scenarios.

**Commit boundary:** `feat(execution): harden resumable task lifecycle`

## TASK-012 — Make local terminal first-class activity

**Requirements:**
- local `tools/call`, jobs, and lifecycle requests send shared AI Code clientInfo;
- keep existing `agentSession` correlation where supported;
- `terminal_exec` is recorded by the same relay path as any other MCP client;
- browser `addToolOutput` is never persistence authority;
- relay connection failure before accepted execution creates no false operation.

**Validation:** real loopback activity survives page refresh/browser close because persistence is relay-owned.

**Commit boundary:** `feat(activity): include paired local terminal activity`

## TASK-013 — Remove legacy provider delegation and add adaptive execution controls

### A. Remove `agent_delegate` and provider-only coding-CLI plumbing

**Required outcome:** current runtime/tooling no longer exposes or launches provider-specific external coding CLIs through `agent_delegate`.

**Requirements:**
- remove `agent_delegate` from Full/Primary MCP catalogs, profile counts/contracts, task dispatch, normal dispatch, hook/effect policy, capability filtering, and current user/operator docs;
- remove provider discovery/config/allowlist/auth-root/sandbox code that exists only for this tool after confirming it has no unrelated security/runtime owner;
- retire/update active verification scripts that require `agent_delegate` while preserving historical plan records as history rather than rewriting what Plan 046/048 originally delivered;
- remove dead dependencies and feature flags where they become unused;
- do not retain a hidden compatibility alias or generic shell wrapper that still launches the removed provider CLIs;
- do not confuse this product removal with platform-native sub-agent usage by the implementation agent or unrelated internal orchestration.

**Validation:** current MCP catalogs contain no `agent_delegate`; no current config/docs advertise provider CLI delegation; no runtime route can invoke the removed provider launcher; normal native workspace/Git/code/terminal/task functionality remains green.

### B. Standardize agent-selected timeout

**Requirements:**
- keep/add `timeout_ms` on eligible blocking tools where a caller-controlled execution deadline is meaningful;
- remove the Primary-only 30-second `terminal_exec` cap; honor the requested timeout up to tool/operator policy regardless of Full/Primary profile;
- define requested vs effective timeout explicitly and expose safe effective facts in task/activity metadata;
- keep operator maximums authoritative and reject/normalize out-of-policy requests deterministically;
- document `timeout_ms = 0` semantics and never treat it as unbounded when operator policy configured a maximum;
- keep HTTP transport/request timeout separate from tool runtime timeout and background task lifetime.

### C. Add explicit `execution_mode: sync | async | auto`

**Requirements:**
- add execution-mode selection only to reviewed task-eligible tools; do not add meaningless async knobs to tiny bounded metadata reads;
- at minimum support the current useful task-capable set after delegation removal (`terminal_exec`, safe task-capable `http_fetch` methods, and `web_search`), then review remote Git/forge/network operations that have real timeout pressure and add them only when lifecycle/cancellation/idempotency semantics are sound;
- `sync` waits for the direct result using requested/effective timeout;
- `async` returns a standard MCP task promptly and continues independently; Primary and Full advertise Tasks when actually supported;
- `async` + client without Tasks support returns a clear capability error and does not silently execute synchronously;
- `auto` follows one deterministic documented rule and never silently duplicates work;
- mutating async operations require/reuse a stable execution/idempotency identity where necessary to make retry after a lost response safe;
- first-party clients should prefer standard MCP Tasks over special-case `terminal_job_*`; keep terminal job tools only as compatibility surface until removal is separately justified;
- activity records requested/effective execution mode and one lifecycle for the logical operation rather than a new row per poll.

**Validation:** Primary `terminal_exec` can request >30s within operator max; over-max is rejected; sync direct result works; async returns quickly and completes via poll; cancellation works; reconnect/lost-response does not restart accepted work; unsupported-client async fails clearly; auto policy is deterministic; one activity lifecycle remains stable.

**Commit boundary:** `feat(execution): simplify delegation and add adaptive task execution`

## TASK-014 — Add asynchronous authenticated exporter and recovery

**Requirements:**
- HTTPS bounded batches in source order using source bearer credential;
- cap batch count/body/concurrency;
- 2xx acknowledgment must identify accepted/duplicate activities;
- timeout/429/5xx retry with bounded exponential backoff+jitter and reviewed retry hints;
- persistent 401/403 degrades/stops credential hammering without printing token/body;
- ack marking transactional; acknowledged rows retained briefly then pruned;
- graceful shutdown flush is best-effort and bounded;
- startup marks stale running rows interrupted, requeues unacked rows, exposes metadata-only backlog health;
- required mode rejects new workspace execution when quota cannot admit the start record.

**Validation:** success, duplicate ack, timeout, 429, 5xx, 401/403, malformed response, crash during retry, quota, corrupt DB, lost key.

**Commit boundary:** `feat(activity): deliver and recover relay activity outbox`

**Phase exit criteria:**
- [x] required mode durably records before workspace execution;
- [x] sink outage creates no silent gap;
- [x] restart/retry is idempotent and quota-safe;
- [x] local terminal uses the same recorder as remote MCP;
- [x] task/job outcomes follow actual execution;
- [x] `agent_delegate` and provider-only coding-CLI delegation are absent from current runtime/catalog/docs;
- [x] agent-selected timeout works within operator/tool bounds without a Primary-only 30-second ceiling;
- [x] reviewed tools support deterministic `sync`/`async`/`auto` execution and accepted async work is resumable/cancellable without blind restart;
- [x] journal payload is encrypted and owner-only;
- [x] telemetry contains no activity payload or credentials.

---

# PHASE-03 — Mutation diffs and change evidence

**Goal:** preserve exact historical evidence where the relay owns before/after state and truthful summaries where it does not.

## TASK-015 — Define bounded change evidence model

**Required fields:** relative path, change type (`create`, `modify`, `delete`, `rename` only where provable), additions/deletions, content kind, completeness, evidence class, normalized unified diff payload, integrity/checksum metadata.

**Rules:**
- `exact` requires authoritative before/after comparison;
- `summary` means mutation known but exact historical content incomplete/unavailable;
- `unavailable` is not rewritten to `no_change`;
- `not_applicable` only for non-mutating operations;
- binary/non-UTF8 never coerced into lossy text;
- diff/source encrypted before durable journal storage;
- counts derive from stored evidence, not client claims;
- preview/dry-run is never presented as applied.

**Commit boundary:** `feat(activity): define mutation change evidence`

## TASK-016 — Use a deterministic bounded text diff implementation

Prefer a mature deterministic Rust diff library when it reduces correctness risk; audit dependency/license/build impact. Preserve newline-at-EOF semantics and bound CPU/memory by current file ceilings/activity payload limits.

**Validation:** insert/delete/replace/repeated-lines/no-final-newline/near-limit golden fixtures.

**Commit boundary:** `feat(activity): generate deterministic text diffs`

## TASK-017 — Preserve exact `file_edit` history

Reuse already-loaded `source` and computed `updated`; generate diff after edit preflight succeeds and before old state is discarded. `changed=false` stores no diff. Existing atomic/no-follow/identity/mode safety remains untouched. Activity evidence stays internal rather than expanding public MCP result unless separately reviewed.

**Commit boundary:** `feat(activity): preserve file edit history`

## TASK-018 — Preserve exact `file_write` history

For create: empty-before vs content. For overwrite: read the bounded existing regular file through the same secure descriptor/identity path before atomic replacement. Do not reopen via unsafe path after validation. Binary/non-UTF8 replacement gets summary only.

**Commit boundary:** `feat(activity): preserve file write history`

## TASK-019 — Preserve actual `apply_patch` history

Reuse preflight-loaded originals/final buffers; generate per-file exact diff only after validation succeeds. Dry-run remains preview. Evidence must reflect actual commit/rollback guarantees and never pretend multi-file atomicity beyond implementation truth.

**Commit boundary:** `feat(activity): preserve applied patch history`

## TASK-020 — Classify Git mutation evidence

Separate metadata-only Git mutations from working-tree mutations. For restore/clean/reset/cherry-pick/revert/merge/rebase, use existing structured Git state/bounded before-after inspection where safe. Only call evidence exact when actually proven; otherwise store summary with affected paths/status and reason exact history is unavailable. Never create stash/temp commits solely for logging.

**Commit boundary:** `feat(activity): classify git mutation evidence`

## TASK-021 — Add bounded opaque-process/background-task change evidence

Reuse reviewed metadata fingerprint/snapshot approach before/after opaque writers where scalable. For Git workspaces, collect bounded structured post-execution status/change summaries with protected paths filtered and external helpers disabled. `no_change` requires real evidence; incomplete snapshot → `unavailable`. Never persist stdout/stderr as filesystem truth. Async/background execution must preserve the same evidence semantics as sync execution.

**Commit boundary:** `feat(activity): summarize opaque workspace mutations`

**Phase exit criteria:**
- [x] file edit/write/patch preserve exact text diffs for supported successful mutations;
- [x] additions/deletions/affected paths derive from actual evidence;
- [x] dry-run/no-op/error states are truthful;
- [x] binary/unsupported content uses bounded summary only;
- [x] Git never overclaims exactness;
- [x] terminal/background-task evidence distinguishes summary/unavailable/no-change;
- [x] diff payload is encrypted before journal persistence and excluded from telemetry.

---

# PHASE-04 — Nuxt ingestion, persistence, retention, and query

**Goal:** build the authenticated product persistence boundary and one server-shaped read model for the Logs UI.

## TASK-022 — Add activity persistence schema

**Planned tables/responsibilities:**
- `relay_activity_sources`: source ID, owner user FK, optional device FK, safe label/kind, token hash/prefix, created/last-seen/revoked timestamps;
- `relay_activity_workspace_bindings`: source FK, workspace FK, root fingerprint, timestamps, unique source+fingerprint;
- `workspace_activity`: source/activity identity, workspace FK, source ordering, contract version, actor/channel/tool/category/effects/status, relative target, timing, change metadata, occurrence/ingestion timestamps;
- `workspace_activity_payloads`: activity FK, payload kind/version, encrypted envelope, checksum, byte counts, completeness/chunk metadata.

**Requirements:**
- unique source/activity identity and useful source-sequence indexes;
- cursor index `(workspace_id, started_at DESC, id DESC)` or reviewed equivalent immutable tuple;
- no plaintext tokens/keys;
- workspace deletion cascades history;
- migration generated against current schema at implementation time because predecessor plans may advance numbering;
- migration forward/rollback/recovery reviewed in disposable DB.

**Commit boundary:** `feat(activity): add workspace activity persistence schema`

## TASK-023 — Add purpose-separated activity encryption

**Planned files:** `server/infrastructure/activity/crypto.ts`, server-only runtime config and `.env.example`.

Use AES-256-GCM or reviewed equivalent with random nonce and dedicated activity key/domain separation. Authenticate activity identity/version as AAD where practical. Decryption/tamper errors fail closed without logging ciphertext/plaintext.

**Commit boundary:** `feat(activity): encrypt activity payloads at rest`

## TASK-024 — Add scoped source enrollment/revocation

**Planned ownership:** `server/application/activity.ts`, `server/infrastructure/database/activity.ts`, composition and bounded API routes/shared validation.

**Requirements:**
- >=256-bit source bearer token returned once, hash stored at rest with safe prefix;
- source bound to authenticated user and optional same-user paired device;
- safe metadata-only list;
- idempotent revocation immediately blocks ingestion;
- source credential authorizes only activity ingestion/binding, not normal app APIs;
- creation/rotation rate-limited and telemetry payload-free.

**Commit boundary:** `feat(activity): add relay activity source enrollment`

## TASK-025 — Bind relay root fingerprints to owned workspaces

Compute the same fingerprint from server-resolved canonical workspace path. Match only owner workspaces, require exact unambiguous match, persist source+root→workspace binding, reject stale/path-changed fingerprints, support bounded bind handshake for newly authorized relay roots, and never auto-create a Nuxt workspace from relay input.

**Commit boundary:** `feat(activity): bind relay roots to owned workspaces`

## TASK-026 — Add authenticated idempotent ingestion

**Planned API:** `server/api/activity/ingest.post.ts` or equivalent layered route.

**Requirements:**
- authenticate source bearer before expensive processing where practical;
- exact content type, body/event/field/payload limits;
- body `userId`/`workspaceId` cannot grant authority;
- ownership/binding reasserted server-side;
- one transaction for idempotent insert/legal transition + matching encrypted payload;
- duplicate/reordered retries converge; terminal state cannot regress;
- bounded accepted/duplicate/rejected acknowledgment; no source-bearing echo;
- last-seen updates safe;
- telemetry limited to operation/outcome/version/source class/count/byte buckets/duration, never actor free text, commands, paths, diffs, raw body, or raw DB errors.

**Commit boundary:** `feat(activity): ingest idempotent relay activity batches`

## TASK-027 — Add owned list/detail/diff APIs

**List API:** one ready-to-render metadata page, ownership checked, cursor not offset, bounded filters/page sizes, relative paths only, no full diff.

**Detail/diff APIs:** assert user→workspace→activity ownership; decrypt only on explicit diff fetch; set no-store/private cache policy; bound response/chunking; tampered ciphertext gives generic safe error.

**Planned routes:**
- `server/api/workspaces/[id]/activity/index.get.ts`;
- `server/api/workspaces/[id]/activity/[activityId].get.ts`;
- `server/api/workspaces/[id]/activity/[activityId]/diff.get.ts`.

**Commit boundaries:**
- `feat(activity): query workspace activity history`
- `feat(activity): serve owned activity details and diffs`

## TASK-028 — Add resumable live/near-live updates

Postgres history/cursor remains authoritative. Prefer same-origin SSE with resumable cursor only if current Nitro/H3/Postgres support is clean; otherwise use bounded cursor polling while page is visible. Process-local pub/sub alone is insufficient. If LISTEN/NOTIFY is used, notifications carry IDs only and clients still resolve authorized data through query logic. Full diff never rides the live channel.

**Validation:** reconnect resumes without duplicates/gaps and does not depend on viewer/emitter hitting the same Nuxt process.

**Commit boundary:** `feat(activity): add resumable workspace activity updates`

## TASK-029 — Add retention and clear-history semantics

**Requirements:**
- documented default retention and bounded options;
- cleanup in bounded batches, metadata+payload together;
- harmless concurrent cleanup across app instances;
- explicit clear-history endpoint with ownership/destructive confirmation semantics;
- clear preserves source enrollment/binding unless separately revoked;
- clear records a source-sequence/watermark so delayed pre-clear outbox rows do not repopulate old history;
- workspace deletion cascades activity.

**Commit boundary:** `feat(activity): enforce activity retention and safe clearing`

**Phase exit criteria:**
- [x] source credentials are scoped, hashed-at-rest, revocable;
- [x] root binding maps only to owned workspaces;
- [x] ingestion is bounded, idempotent, transition-safe;
- [x] source-bearing payload is purpose-separated encrypted;
- [x] list/detail/diff authorization and cursor pagination are deterministic;
- [x] live resume uses durable cursor and excludes full diff;
- [x] retention/clear cannot resurrect stale pre-clear activity;
- [x] telemetry remains payload-free.

---

# PHASE-05 — Workspace Logs UI and live review

**Goal:** expose a polished one-workspace/one-timeline experience without turning the sidebar into raw diagnostic output.

## UX principles

1. one workspace, one timeline;
2. action/actor/target/status/time first, heavy detail on demand;
3. primary/cyan means currently active/live only;
4. exact/summary/unavailable/preview/interrupted are visually truthful;
5. no raw tool JSON or hidden reasoning;
6. relative paths by default;
7. diff is lazy-loaded only;
8. cursor history, not offsets;
9. reconnect resumes from durable state;
10. clear-history is explicit and separate from source revocation.

## TASK-030 — Add Logs navigation and page shell

**Planned files:**
- modify `app/components/shell/AppSidebar.vue`;
- create `app/pages/workspaces/[id]/logs.vue`;
- create `app/components/workspace/WorkspaceActivityView.vue`;
- add `app/components/workspace/activity/` components only where responsibilities justify them.

**Requirements:**
- one stable **Logs** item under each workspace group, not raw entries in sidebar;
- preserve collapsed/sidebar/chat behavior;
- direct reload/SSR-safe route;
- initial data from one server-shaped activity response;
- compact workspace header with live/refresh and retention/clear actions;
- skeleton, owned-not-found/error retry, empty states.

**Commit boundary:** `feat(activity): add workspace logs navigation and page`

## TASK-031 — Render timeline, filters, and cursor pagination

**Row facts:** timestamp, actor/source, operation/category, relative target, status, duration, affected files, additions/deletions, evidence class, detail affordance.

**Requirements:**
- Nuxt UI primitives and semantic theme classes;
- status not communicated by color alone;
- friendly bounded errors;
- generic external actor fallback;
- keyboard/screen-reader support;
- bounded category/status/source/path/tool filters reflected in route query where useful;
- debounce text filter;
- append older pages by server cursor with activity-ID dedupe; reset cursor on filter changes.

**Commit boundary:** `feat(activity): render and filter workspace activity timeline`

## TASK-032 — Add resumable live updates

Consume PHASE-04 SSE/polling contract, persist last durable cursor/activity identity, merge running→terminal by activity ID, reject UI-side terminal regression, reduce polling while hidden when applicable, and show subtle reconnect/degraded state. Full diffs remain lazy.

**Commit boundary:** `feat(activity): stream workspace log updates`

## TASK-033 — Add activity detail and historical diff review

**Detail:** safe actor/source facts, lifecycle/duration/tool/category/effects/target/result/affected paths/task correlation, explicit preview/no-op/interrupted/summary/unavailable states; no source token IDs/hashes or absolute root.

**Diff viewer:** only for exact available evidence; render file headers/hunks/context/+/- as inert text with whitespace/long-line handling, totals, completeness/truncation/chunk continuation. Never `v-html` source. Summary/unavailable gets truthful explanation, not fake empty diff.

**Commit boundaries:**
- `feat(activity): inspect activity details`
- `feat(activity): review historical workspace diffs`

## TASK-034 — Add retention/clear UX and polish

Display retention policy/options if exposed. Clear requires explicit workspace confirmation and explains that future activity continues and relay access is not revoked. After success reset timeline/cursor/watermark; post-clear new activity still appears.

Complete distinct empty/filter-empty/error/reconnect states, keyboard operation, dark/light, narrow/mobile layouts, truncation/tooltips, and maintainability review. Avoid unnecessary new public composable files.

**Commit boundary:** `fix(activity): polish workspace logs experience`

## TASK-035 — Verify production-built browser behavior

Build fresh output and use `pnpm preview`; do not trust stale dev state. Exercise direct Logs reload, running→terminal update, filter, older-page load, detail/diff, reconnect, clear-history, light/dark, narrow layout, keyboard behavior, and confirm source-bearing payload does not leak into unrelated telemetry/network calls.

**Phase exit criteria:**
- [x] every workspace exposes Logs without sidebar spam;
- [x] timeline shows truthful actor/action/target/status/time/duration/change evidence;
- [x] cursor/filter behavior is deterministic;
- [x] live resume converges to durable history;
- [x] exact diffs are lazy-loaded and safely rendered;
- [x] summary/unavailable/preview/interrupted states are explicit;
- [x] retention/clear semantics are clear and safe;
- [ ] production build/preview browser acceptance passes — UNPROVEN: no browser runtime/fixture was available in this environment.

---

# PHASE-06 — Reliability, security, live acceptance, and closure

**Goal:** falsify the composed system before marking Plan 050 complete.

## TASK-036 — Add one composed Plan-050 verifier

**Planned files:** `scripts/verify-050-workspace-activity-ledger.sh` plus focused TypeScript/Rust acceptance helpers as justified; `package.json` exposes one `verify:050` entrypoint.

The verifier must drive contract, relay durability, adaptive execution, diff, persistence/API, and UI-contract acceptance in dependency order and include composed source→journal→ingestion→query assertions. It must also prove the current tool catalog no longer exposes `agent_delegate`, Primary/Full execution capability negotiation is truthful, requested/effective timeout policy is enforced, and sync/async/auto task behavior is deterministic. Missing required local dependencies must fail nonzero rather than becoming success.

**Target:** pass three consecutive times on the final implementation candidate.

**Commit boundary:** `test(activity): compose workspace activity acceptance`

## TASK-037 — Prove crash/restart/outage/quota integrity

Required scenarios:
- Primary `terminal_exec` with requested timeout above 30 seconds succeeds when within operator/tool policy;
- timeout above configured operator/tool maximum fails before execution with a bounded error;
- explicit sync execution waits for direct result without being silently converted to async;
- explicit async returns a task promptly, continues after initiating RPC disconnect/timeout, and completes through task polling;
- async request from a client without Tasks capability fails clearly without silently running sync;
- retry/reconnect with stable execution identity converges on the accepted task rather than launching duplicate mutation/process work;
- cancellation terminates the owned task/process tree and produces one terminal lifecycle;
- kill after durable start before dispatch;
- kill during long-running job/task;
- kill after terminal journal commit before server ack;
- kill after server ack before local ack mark;
- corrupt/truncated journal;
- missing/wrong local encryption key;
- sink unavailable across multiple operations;
- 429/Retry-After, 5xx, timeout, persistent 401/403;
- spool quota full fails closed before new workspace execution;
- acknowledged prune vs unacknowledged retention;
- recovery drains in sequence and converges without duplicates.

## TASK-038 — Prove concurrency/order/clear semantics

Required scenarios:
- concurrent read/search/edit/terminal in one workspace;
- concurrent activity across two authorized workspaces with no cross-bind;
- same batch delivered concurrently twice;
- outcome-before-start retry order;
- terminal state cannot regress;
- cursor pagination while new events arrive;
- clear watermark racing delayed pre-clear outbox.

## TASK-039 — Attack source auth, ownership, and resource limits

Required attacks:
- missing/malformed/wrong/revoked bearer source;
- body-supplied foreign user/workspace;
- foreign/stale/ambiguous root fingerprint;
- cross-workspace/cross-user detail/diff/clear;
- old source token after rotation/revocation;
- oversized body/batch/event/string/path/effects/path count/diff;
- unknown contract version;
- malformed ciphertext/checksum/chunk metadata;
- conflicting IDs/sequences/control characters;
- event/source-creation/live-connection flooding.

Expected: bounded 4xx/429/rejections, no foreign mutation, no process crash/unbounded memory, no raw body/error logging.

## TASK-040 — Run plaintext/secret canary sweep

Place controlled canaries in diff text, command/search summaries, relative paths, fake source token, encrypted payload. Inspect relay SQLite/WAL, PostgreSQL metadata/ciphertext, Nuxt/relay stdout/stderr, Loki, Jaeger/OTel, list/live APIs, errors, browser network/console.

Expected source-bearing content appears only through explicitly authorized decryption/diff response. Tokens must not remain in logs/DB after one-time creation/config boundary. Protected-path denial must never capture protected file content.

## TASK-041 — Re-run established security regression contracts

At minimum relevant coverage for:
- current MCP profile/catalog contracts proving `agent_delegate` is absent and no dead provider-only capability is advertised;
- protected credential paths;
- symlink/no-follow/atomic mutation;
- OAuth/scope/owner binding;
- terminal network isolation and optional sockets;
- Git protected path/ref/helper policy;
- hooks/approval effect parity;
- task cancellation/process cleanup;
- Plan-035 telemetry confidentiality;
- architecture/maintainability boundaries.

Never weaken an existing boundary to make Plan 050 pass.

## TASK-042 — Live product and caller acceptance

### Paired/local terminal
Use real `local-tool-controller` → `useRelayAgent` → loopback relay path with harmless workspace-scoped commands. Confirm journal, server ingestion, Logs row, AI Code/local actor facts, and persistence across browser refresh/close/reopen. Exercise at least one caller-selected timeout above the old 30-second Primary ceiling (without needing to actually consume the full duration) and one async task path that is polled to completion rather than restarted.

### First-party Nuxt MCP
Use configured first-party MCP against target workspace for representative read plus safe structured mutation. Confirm one activity per real tool operation and exact diff for structured mutation, with no duplicate from chat UI/telemetry.

### Direct external MCP / external MCP client
When deployment/connector is available and separately authorized, execute harmless read/search and optional controlled structured mutation. Confirm the operation appears without Nuxt being the caller. Inspect actual `clientInfo`: show `external MCP client` only when real metadata supports it; otherwise display `External MCP client` and mark exact actor identification UNPROVEN.

### Generic client
Exercise standards-compliant MCP with and without optional clientInfo; both must work under existing auth/policy and differ only in presentation metadata.

Any unavailable live external proof is explicitly **UNPROVEN**, never inferred.

## TASK-043 — Measure performance and operability

Measure same-binary/environment baseline with activity off vs required mode. Exporter network delay must be absent from tool critical path after local journal commit. Initial review target: p95 journal bookkeeping overhead no more than 25 ms above off-mode for a repeated no-diff read fixture; if environment makes that unrealistic, document measured cause and require explicit review rather than weakening durability silently.

Measure diff generation separately near file-size limits. Verify metadata-only operator diagnostics for queued rows/bytes, delivery/degraded state, acknowledged pruning, retention, and clear storage reclamation without decrypting payload.

## TASK-044 — Fresh independent security/architecture review

Review relay vs Nuxt ownership/layering, source credential scope, workspace attribution/IDOR, journal durability/failure semantics, state transitions/idempotency, encryption/key handling, telemetry separation, diff TOCTOU/protected-path safety, local-terminal completeness, actor spoofing, retention/clear race, multi-instance live behavior, and overengineering/maintainability.

**Exit:** zero unresolved P0/P1. Confirmed lower-severity findings are fixed or explicitly accepted with rationale in this Plan 050 file.

## TASK-045 — Reconcile docs and durable knowledge

Update only where final behavior requires:
- `README.md`;
- `docs/architecture.md`;
- `docs/configuration.md`;
- `docs/remote-mcp.md`;
- `packages/relay-agent/SKILL.md`;
- `.agents/knowledge/project.md`;
- relevant `.agents/knowledge/tooling.md` / `resources.md`;
- `.agents/memories/README.md` with durable decisions/traps only;
- `ai-self/` only if genuinely reusable procedural learning emerges.

Document activity as product data separate from telemetry; required/off mode; state dir/source enrollment/sink/backlog/recovery; retention; exact-diff guarantee and opaque-writer limitations; truthful clientInfo/actor behavior; only verification actually performed.

## TASK-046 — Run final repository gates

Required final commands:

```bash
pnpm verify:050
pnpm verify:commit
pnpm build
pnpm build:tools
pnpm audit
git diff --check
```

Run `cargo audit` when Rust dependency/security surface changes and all relevant deterministic security/contract scripts affected by relay workspace/Git/tasks/hooks/telemetry behavior.

Do not claim a command passed unless it actually did.

**Phase exit criteria:**
- [x] composed Plan-050 verifier passes three consecutive times;
- [ ] crash/restart/outage/quota tests show no silent accepted-operation loss;
- [ ] duplicate/out-of-order/concurrent delivery converges to one truthful lifecycle entry;
- [ ] cross-user/cross-workspace/source-token attacks fail safely;
- [ ] plaintext canary sweep is clean across spool/DB/telemetry/stdout/stderr/non-diff APIs;
- [x] `agent_delegate`/provider-CLI removal and adaptive timeout/sync-async execution acceptance are green;
- [x] existing protected-path/OAuth/Git/task/hook/telemetry regressions remain green;
- [ ] production-built Logs UI works against real persisted activity — UNPROVEN: no browser/runtime fixture was available.
- [ ] local terminal end-to-end is proven;
- [ ] first-party Nuxt MCP end-to-end is proven;
- [ ] external/direct MCP is proven when available, with actor label based only on actual metadata — UNPROVEN: no authorized connector/source was available.
- [ ] generic MCP compatibility remains vendor-neutral;
- [ ] journaling overhead/storage/backlog behavior is measured and accepted;
- [x] fresh independent review has zero unresolved P0/P1;
- [x] docs/knowledge/memory/source/plan truth is reconciled;
- [x] repository verification/build/audit gates pass.

## Closure evidence — 2026-08-26

The implementation was completed on the dedicated `feat/050-workspace-activity-ledger` branch from `main`. The relay now has the versioned activity contract, canonical-root attribution, truthful layered actor/client metadata, required-mode encrypted SQLite journaling, authenticated bounded export/retry/recovery, lifecycle-safe terminal outcomes, exact structured mutation evidence, and fail-closed quota/key/corruption behavior. Nuxt/Postgres now has scoped source enrollment and revocation, owned root bindings, encrypted payload storage, idempotent ingestion, cursor/detail/diff/clear/retention APIs, and the workspace Logs surface. The provider-specific `agent_delegate` surface and coding-CLI plumbing were removed from the current catalog/runtime/config/docs, and eligible execution supports bounded caller-selected timeout plus explicit `sync`/`async`/`auto` behavior with task idempotency.

Local evidence completed:

- `pnpm verify:050` passed three consecutive times, including the real encrypted journal acceptance example.
- `pnpm verify:commit` passed, including architecture, maintainability, lint, typecheck, repository policy, and all configured contract gates.
- `pnpm lint`, Nuxt preparation/typecheck, Rust workspace check with `-D warnings`, `pnpm check:architecture`, `node scripts/check-maintainability.mjs`, `pnpm build`, `pnpm build:tools`, `pnpm audit`, `cargo audit`, `git diff --check`, and the affected Plan 046/047/048 regression scripts passed.
- Independent review found and the implementation fixed terminal-outcome error loss, spoofable actor attribution, and unbounded chunked ingestion. The reviewers re-checked the fixes and reported zero unresolved P0/P1 findings.

The following acceptance remains explicitly **UNPROVEN**, not inferred: live PostgreSQL migration/ingestion against an enrolled source, paired/local-terminal end-to-end activity, first-party Nuxt MCP activity, browser acceptance against persisted Logs, direct external MCP/external MCP client connector activity, generic-client live compatibility, adversarial database/connector/canary sweeps, and measured performance/backlog behavior. The local environment had no activity payload secret, authorized enrollment/source fixture, or available browser runtime; no shared database mutation or deployment was performed. These are external/environmental blockers for live closure and require an authorized operator fixture and deployment/runtime access.

## Deployment-boundary follow-up — 2026-08-27

The production Nuxt runtime was deployed as a Docker-only image on port 3333;
the image and running container contain no Rust workspace, relay package,
native target, adapter package, or `ai-tools` binary. The Rust relay remains a
separate active user-systemd service on its inspected host binary path and
loopback port 47821. Nuxt chat-mode `curl` and web search now route through the
first-party MCP adapter, but authenticated Nuxt-to-relay execution remains
UNPROVEN because the three private `NUXT_REMOTE_MCP_*` values were empty on the
machine. The old project image tag was removed selectively; no orphan cleanup,
global prune, or unrelated Docker-project cleanup was performed.

---

## Risks and rollback

- **Silent sink gaps:** relay-local durable outbox is mandatory in required mode; browser callbacks are insufficient.
- **Wrong-workspace attribution:** raw cwd/client UUID is stale/spoofable; use authoritative containing-root resolution + owned binding.
- **Actor spoofing:** clientInfo is presentation only; authorization stays with actual transport/source/OAuth policy.
- **Activity store becomes a secret dump:** fixed per-tool schemas, redaction, encrypted payloads, protected paths, and canary sweeps are mandatory.
- **Telemetry regression:** explicit architecture/security checks must prove no source/diff payload reaches OTel/Loki.
- **Relay disk exhaustion:** quota + no unacked pruning; required mode fails closed rather than silently overwriting evidence.
- **Large diff amplification:** bound generation/storage, lazy-load payload, expose completeness/truncation.
- **Opaque writer false precision:** summary/unavailable is preferable to fabricated exactness.
- **Unbounded agent timeout:** caller-selected timeout never overrides operator/tool safety ceilings; deadline-free execution exists only when operator policy explicitly allows it.
- **Async duplicate execution after lost response:** stable task/execution identity plus idempotent acceptance must prevent a reconnect/retry from blindly launching the same mutating work twice.
- **Capability mismatch:** explicit async must fail clearly when the client cannot consume MCP Tasks; do not silently downgrade modes or advertise Tasks in profiles that cannot honor them.
- **Legacy delegation removal regression:** remove provider-only code only after dependency tracing so unrelated native sandbox/auth/sub-agent features are not accidentally deleted.
- **Multi-instance live updates:** DB cursor is authoritative; process-local pub/sub is insufficient.
- **Migration overlap with predecessor plans:** generate/review migration against current main at implementation time.
- **Live connector unavailable:** mark external proof UNPROVEN; never fabricate identity/evidence.
- **Performance pressure:** never move sink networking onto critical path or weaken durability/fsync silently merely to hit a target.

Rollback may disable activity mode/capture/UI while preserving already-recorded encrypted history/spool. Do not delete retained product history merely because the feature is temporarily disabled. Source revocation and activity-mode disablement are separate from historical data deletion.

## Final acceptance criteria

- [x] The implementation provides one ordered persistent activity history per workspace independent of caller/client.
- [x] First-party Nuxt MCP, local terminal, remote/generic MCP, and sync/async task/job activity route through the same relay contract (live end-to-end proof remains unproven).
- [x] `agent_delegate` and provider-specific coding-CLI delegation are removed from the current MCP/runtime/config/docs surface without breaking unrelated native/sub-agent capabilities.
- [x] Agents can request appropriate tool timeouts within operator/tool limits, including Primary `terminal_exec` requests above 30 seconds when policy permits.
- [x] Reviewed long-running tools support explicit `sync`/`async`/`auto`; async work returns a resumable/cancellable task identity and does not restart blindly after client/RPC timeout.
- [x] Required mode durably records start intent before workspace execution.
- [x] Sink outage/restart/duplicate delivery has bounded retry, recovery, idempotent ingestion, and no-unacked-prune safeguards (full live outage/concurrency proof remains unproven).
- [x] Workspace attribution is based on canonical relay roots and owned bindings, never body-supplied Nuxt IDs.
- [x] File edit/write/patch history includes accurate affected paths, additions/deletions, and expandable historical diff.
- [x] Opaque writer limitations are explicit and never masquerade as complete provenance.
- [x] Source-bearing payloads are encrypted at rest and decrypted only after workspace ownership checks.
- [x] OTel/Loki contains no activity diff/source/credential leakage by contract and source checks.
- [x] Logs appears under each workspace and supports cursor pagination, filtering, live resume, details, and lazy diffs (browser/runtime proof remains unproven).
- [x] Retention, clear-history, and workspace deletion have deterministic removal semantics in the implementation.
- [ ] Protected-path, cross-workspace, token-revocation, oversized-body, malformed-event, retry, cancellation, and concurrency attacks fail safely — implementation checks are present, but the full live adversarial matrix was not executable here.
- [ ] Live local-terminal and first-party MCP evidence is recorded; direct external client evidence is recorded when available and never guessed.
- [x] Repository verification/build/audit and Plan-050 deterministic acceptance pass.
- [x] Final independent review reports zero unresolved P0/P1 findings.

## Execution handoff

Implementation proceeds **phase by phase in this single plan file**:

```text
PHASE-01 Contract + Identity
   ↓
PHASE-02 Relay Journal + Capture
   ↓
PHASE-03 Diff Evidence
   ↓
PHASE-04 Nuxt Persistence/API
   ↓
PHASE-05 Logs UI
   ↓
PHASE-06 Reliability/Security/Closure
```

PHASE-03 and PHASE-04 may overlap only after PHASE-01 contract and the PHASE-02 recorder/outbox envelope are stable; otherwise keep execution serialized for simpler review and closure.

If Plan 050 has not started yet, begin from a clean short-lived implementation branch based on current `main`. If implementation is already active on the dedicated Plan-050 branch/worktree, continue there and reconcile these new requirements against the current partial source rather than resetting/restarting completed work. In either case, re-check repository identity/Git state, preserve unrelated work, update this single file's checklists/status truthfully, and follow repository verification/PR policy.

Plan closure is not authorization to deploy/restart production or perform irreversible external changes; those remain separate operator actions.
