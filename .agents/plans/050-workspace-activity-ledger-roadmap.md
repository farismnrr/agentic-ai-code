# Plan 050 — Workspace Activity Ledger

**Status:** PLANNED
**Created:** 2026-08-26

## Goal

Deliver an industry-standard, persistent, per-workspace activity ledger that truthfully records every workspace-scoped operation mediated by `ai-tools relay`, regardless of whether the caller is the first-party Nuxt application, the paired/local-terminal path, external MCP client through remote MCP, another MCP client, or a delegated/background execution path.

The product target is a modern coding-agent workspace history: users can open **Logs** under a workspace, see one chronological stream of reads/searches/commands/Git/code operations and mutations, inspect actor/status/duration/change evidence, and expand durable historical diffs for supported structured mutations.

Plan 050 is intentionally one plan file. Its implementation is divided into phases rather than child plans so design, progress, closure evidence, and source of truth remain centralized.

## Success criteria

Plan 050 is complete only when:

1. every relay-mediated workspace operation resolves to one authoritative canonical workspace root and one owned Nuxt workspace binding without trusting a client-supplied workspace UUID;
2. first-party Nuxt MCP, paired/local-terminal calls, direct external MCP client remote MCP, generic MCP clients, task/job execution, and relevant delegated/background execution all use the same activity contract;
3. the relay durably records a workspace operation locally before execution when activity logging is configured as required, so Nuxt/network outages do not create silent gaps;
4. relay delivery to Nuxt is authenticated, idempotent, retryable, bounded, and crash-safe;
5. PostgreSQL stores one chronological workspace activity history with ownership-enforced cursor queries, bounded retention, and explicit deletion semantics;
6. `file_edit`, `file_write`, and `apply_patch` preserve accurate historical before/after diffs with additions/deletions and affected paths for supported text mutations;
7. opaque process/delegated execution is represented truthfully with lifecycle plus bounded change evidence rather than fabricated exact provenance;
8. activity payloads containing source/diffs are strictly separated from Plan-035/039J OpenTelemetry/Loki telemetry and encrypted at rest with purpose-separated key material;
9. the UI exposes **Logs** under each workspace with live/near-live updates, filters, lazy details/diffs, explicit integrity/completeness states, and no hidden reasoning;
10. deterministic reliability/security acceptance proves no silent loss, duplication, misattribution, cross-tenant leakage, secret leakage, or unsafe failure under restart/outage/quota/concurrency conditions;
11. live acceptance proves local terminal, first-party Nuxt MCP, and direct external MCP activity where the relevant authenticated path is available;
12. final independent review has zero unresolved P0/P1 security, integrity, confidentiality, or architecture findings.

## Verified current state

Verified in the working tree on 2026-08-26:

- Nuxt persists workspaces in `server/database/schema.ts` with `id`, `userId`, `name`, `path`, and timestamps. `server/infrastructure/database/workspaces.ts` validates filesystem existence but does not define a relay activity binding.
- The Rust relay owns canonical multi-root authorization through `WorkspaceAllowlist` in `packages/rust-tools/core/src/workspace_path.rs`; it knows canonical roots, not Nuxt workspace UUIDs/users.
- `packages/rust-tools/infrastructure/src/transport/tools.rs` is the MCP `tools/call` transport choke point and already owns auth context, pre/post hooks, request identity, and bounded monotonic timing.
- `packages/rust-tools/application/src/execution.rs::dispatch_tool_call` is the shared application execution path for native workspace, Git, code/LSP, search, terminal, HTTP/search, and delegated-agent tools.
- Successful native file mutations already emit `AfterFileChange`, but that hook is intentionally metadata-only and is not historical diff storage.
- `file_edit` already has both `source` and `updated` text before atomic commit; its public result currently returns only path/replacement/change metadata.
- `file_write` returns create/overwrite/byte metadata and does not currently preserve historical before/after content.
- Plan 035/039J observability explicitly forbids source/patch/tool-result contents and private absolute paths from OTel/Loki. Existing telemetry therefore cannot become the product activity database.
- Modern MCP request metadata supports `io.modelcontextprotocol/clientInfo`. The first-party server MCP client sends `{ name: 'ai-code', version: '1.0.0' }`; `app/composables/useRelayAgent.ts` currently omits `clientInfo` on paired/local relay requests.
- Remote OAuth claims contain `sub` and `client_id`, but those identities are distinct from Nuxt `users.id`; `clientInfo` is display metadata, never authorization.
- `local_terminal` executes through `app/composables/chat/local-tool-controller.ts` → `app/composables/useRelayAgent.ts` → loopback relay `tools/call terminal_exec`; it must be logged by the same relay recorder, not by a browser-only special case.
- Hosted Nuxt and external MCP clients may consume the same public MCP resource while authenticating independently.
- Repository policy has no CI and no unit-test suite; deterministic acceptance/security scripts plus `pnpm verify:commit` are the required local verification model.
- Plan 050 must not be implemented from unrelated dirty predecessor work. At implementation start, re-check current branch/worktree, close/reconcile earlier active plans, and branch from current `main` according to repository policy.

## Scope

### In scope

- per-workspace activity identity and binding;
- one versioned activity contract and safe operation summaries;
- relay-local durable journal/outbox;
- authenticated relay-to-Nuxt ingestion;
- first-party Nuxt/local-terminal caller metadata;
- direct remote MCP and generic-client activity;
- synchronous and task/job lifecycle capture;
- structured workspace/Git/code/process/delegated operation summaries;
- exact historical structured mutation diffs;
- bounded change evidence for opaque writers;
- encrypted relay spool and PostgreSQL payload persistence;
- PostgreSQL schema/query/retention/deletion;
- workspace Logs UI with live updates, filters, detail, and diff review;
- confidentiality, restart, outage, quota, concurrency, idempotency, and adversarial acceptance;
- operator/user documentation and configuration.

### Out of scope

- hidden model reasoning or chain-of-thought;
- provider-specific parsing of delegated-agent private transcripts to fabricate relay-observed actions;
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

## Phase overview

| Phase | Goal | Exit criterion |
| --- | --- | --- |
| PHASE-01 | Activity contract, workspace identity, and caller facts | one bounded versioned contract and authoritative workspace/source model are frozen |
| PHASE-02 | Relay durable journal and complete execution capture | required-mode pre-execution durability, task/local/delegated capture, retry/recovery work without telemetry leakage |
| PHASE-03 | Mutation diffs and change evidence | structured text mutations preserve exact history; Git/opaque writers report truthful bounded evidence |
| PHASE-04 | Nuxt ingestion, persistence, retention, and query | authenticated idempotent ingestion maps only to owned workspaces and serves encrypted cursor history |
| PHASE-05 | Workspace Logs UI and live review | every workspace exposes a polished durable timeline with lazy details/diffs and resumable updates |
| PHASE-06 | Reliability, security, live acceptance, and closure | composed outage/security/live matrix passes and docs/source/plan truth is reconciled |

---

# PHASE-01 — Activity contract, workspace identity, and caller facts

**Goal:** freeze one vendor-neutral activity vocabulary and identity model before persistence/UI implementation.

## TASK-001 — Define the versioned activity envelope

**Outcome:** one application-owned contract covers synchronous, task/job, denied/failed, and delegated operations.

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
- strict per-field/string/path/list/payload bounds.

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
- delegated agents: provider/role/state/effect scope/change-evidence class, no private transcript/reasoning;
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
- delegated scope comes from the bounded workspace root already supplied to delegation policy;
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
- [ ] event contract v1 is explicit, bounded, and versioned;
- [ ] canonical effect/tool vocabularies are reused;
- [ ] workspace scope comes from relay authority, not client UUIDs;
- [ ] local relay sends clientInfo consistently;
- [ ] actor metadata is separate from authorization;
- [ ] exact/summary/unavailable evidence classes are explicit;
- [ ] deterministic contract acceptance passes.

---

# PHASE-02 — Relay durable journal and complete execution capture

**Goal:** make the relay the crash-safe observation authority and ensure local terminal/tasks/delegated execution cannot bypass activity capture.

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

## TASK-011 — Capture task/job lifecycle and cancellation

**Requirements:**
- bind task/job ID to originating activity/workspace/source facts;
- one logical activity across queued/running/completed/failed/cancelled;
- `tasks/get` polling does not create duplicate primary activity rows;
- cancellation is a control fact without conflicting terminal outcomes;
- preserve actual execution duration separately from request duration;
- restart marks orphaned nonterminal activity interrupted/unknown.

**Validation:** start→poll→complete, start→cancel, retry, restart scenarios.

**Commit boundary:** `feat(activity): track relay task and job lifecycle`

## TASK-012 — Make local terminal first-class activity

**Requirements:**
- local `tools/call`, jobs, and lifecycle requests send shared AI Code clientInfo;
- keep existing `agentSession` correlation where supported;
- `terminal_exec` is recorded by the same relay path as any other MCP client;
- browser `addToolOutput` is never persistence authority;
- relay connection failure before accepted execution creates no false operation.

**Validation:** real loopback activity survives page refresh/browser close because persistence is relay-owned.

**Commit boundary:** `feat(activity): include paired local terminal activity`

## TASK-013 — Capture delegated/background execution truthfully

**Planned files:** `packages/rust-tools/application/src/execution/agent.rs`, `agent_snapshot.rs`, task lifecycle binding.

**Requirements:**
- provider/role/task lifecycle plus workspace/effect scope;
- bounded opaque parent/child correlation only;
- reuse existing workspace metadata snapshot to classify mutation where appropriate;
- do not parse provider-private transcripts into synthetic relay tool events;
- feed exact/summary/unavailable evidence to PHASE-03 without overclaiming.

**Commit boundary:** `feat(activity): record delegated agent lifecycle`

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
- [ ] required mode durably records before workspace execution;
- [ ] sink outage creates no silent gap;
- [ ] restart/retry is idempotent and quota-safe;
- [ ] local terminal uses the same recorder as remote MCP;
- [ ] task/job outcomes follow actual execution;
- [ ] delegated activity remains provider-neutral/truthful;
- [ ] journal payload is encrypted and owner-only;
- [ ] telemetry contains no activity payload or credentials.

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

## TASK-021 — Add bounded process/delegated change evidence

Reuse reviewed metadata fingerprint/snapshot approach before/after opaque writers where scalable. For Git workspaces, collect bounded structured post-execution status/change summaries with protected paths filtered and external helpers disabled. `no_change` requires real evidence; incomplete snapshot → `unavailable`. Never persist stdout/stderr as filesystem truth.

**Commit boundary:** `feat(activity): summarize opaque workspace mutations`

**Phase exit criteria:**
- [ ] file edit/write/patch preserve exact text diffs for supported successful mutations;
- [ ] additions/deletions/affected paths derive from actual evidence;
- [ ] dry-run/no-op/error states are truthful;
- [ ] binary/unsupported content uses bounded summary only;
- [ ] Git never overclaims exactness;
- [ ] terminal/delegated evidence distinguishes summary/unavailable/no-change;
- [ ] diff payload is encrypted before journal persistence and excluded from telemetry.

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
- [ ] source credentials are scoped, hashed-at-rest, revocable;
- [ ] root binding maps only to owned workspaces;
- [ ] ingestion is bounded, idempotent, transition-safe;
- [ ] source-bearing payload is purpose-separated encrypted;
- [ ] list/detail/diff authorization and cursor pagination are deterministic;
- [ ] live resume uses durable cursor and excludes full diff;
- [ ] retention/clear cannot resurrect stale pre-clear activity;
- [ ] telemetry remains payload-free.

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
- [ ] every workspace exposes Logs without sidebar spam;
- [ ] timeline shows truthful actor/action/target/status/time/duration/change evidence;
- [ ] cursor/filter behavior is deterministic;
- [ ] live resume converges to durable history;
- [ ] exact diffs are lazy-loaded and safely rendered;
- [ ] summary/unavailable/preview/interrupted states are explicit;
- [ ] retention/clear semantics are clear and safe;
- [ ] production build/preview browser acceptance passes.

---

# PHASE-06 — Reliability, security, live acceptance, and closure

**Goal:** falsify the composed system before marking Plan 050 complete.

## TASK-036 — Add one composed Plan-050 verifier

**Planned files:** `scripts/verify-050-workspace-activity-ledger.sh` plus focused TypeScript/Rust acceptance helpers as justified; `package.json` exposes one `verify:050` entrypoint.

The verifier must drive contract, relay durability, diff, persistence/API, and UI-contract acceptance in dependency order and include composed source→journal→ingestion→query assertions. Missing required local dependencies must fail nonzero rather than becoming success.

**Target:** pass three consecutive times on the final implementation candidate.

**Commit boundary:** `test(activity): compose workspace activity acceptance`

## TASK-037 — Prove crash/restart/outage/quota integrity

Required scenarios:
- kill after durable start before dispatch;
- kill during long-running job;
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
Use real `local-tool-controller` → `useRelayAgent` → loopback relay path with harmless workspace-scoped command. Confirm journal, server ingestion, Logs row, AI Code/local actor facts, and persistence across browser refresh/close/reopen.

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
- [ ] composed Plan-050 verifier passes three consecutive times;
- [ ] crash/restart/outage/quota tests show no silent accepted-operation loss;
- [ ] duplicate/out-of-order/concurrent delivery converges to one truthful lifecycle entry;
- [ ] cross-user/cross-workspace/source-token attacks fail safely;
- [ ] plaintext canary sweep is clean across spool/DB/telemetry/stdout/stderr/non-diff APIs;
- [ ] existing protected-path/OAuth/Git/task/hook/telemetry regressions remain green;
- [ ] production-built Logs UI works against real persisted activity;
- [ ] local terminal end-to-end is proven;
- [ ] first-party Nuxt MCP end-to-end is proven;
- [ ] external/direct MCP is proven when available, with actor label based only on actual metadata;
- [ ] generic MCP compatibility remains vendor-neutral;
- [ ] journaling overhead/storage/backlog behavior is measured and accepted;
- [ ] fresh independent review has zero unresolved P0/P1;
- [ ] docs/knowledge/memory/source/plan truth is reconciled;
- [ ] repository verification/build/audit gates pass.

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
- **Multi-instance live updates:** DB cursor is authoritative; process-local pub/sub is insufficient.
- **Migration overlap with predecessor plans:** generate/review migration against current main at implementation time.
- **Live connector unavailable:** mark external proof UNPROVEN; never fabricate identity/evidence.
- **Performance pressure:** never move sink networking onto critical path or weaken durability/fsync silently merely to hit a target.

Rollback may disable activity mode/capture/UI while preserving already-recorded encrypted history/spool. Do not delete retained product history merely because the feature is temporarily disabled. Source revocation and activity-mode disablement are separate from historical data deletion.

## Final acceptance criteria

- [ ] One workspace has one ordered persistent activity history independent of caller/client.
- [ ] First-party Nuxt MCP, local terminal, remote/generic MCP, task/job, and delegated activity use the same contract.
- [ ] Required mode durably records start intent before workspace execution.
- [ ] Sink outage/restart/duplicate delivery cannot silently lose or duplicate operations.
- [ ] Workspace attribution is based on canonical relay roots and owned bindings, never body-supplied Nuxt IDs.
- [ ] File edit/write/patch history includes accurate affected paths, additions/deletions, and expandable historical diff.
- [ ] Opaque writer limitations are explicit and never masquerade as complete provenance.
- [ ] Source-bearing payloads are encrypted at rest and decrypted only after workspace ownership checks.
- [ ] OTel/Loki contains no activity diff/source/credential leakage.
- [ ] Logs appears under each workspace and supports cursor pagination, filtering, live resume, details, and lazy diffs.
- [ ] Retention, clear-history, and workspace deletion have deterministic removal semantics.
- [ ] Protected-path, cross-workspace, token-revocation, oversized-body, malformed-event, retry, cancellation, and concurrency attacks fail safely.
- [ ] Live local-terminal and first-party MCP evidence is recorded; direct external client evidence is recorded when available and never guessed.
- [ ] Repository verification/build/audit and Plan-050 deterministic acceptance pass.
- [ ] Final independent review reports zero unresolved P0/P1 findings.

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

Do not begin Plan 050 from unrelated dirty predecessor work. At execution start: re-check repository identity/Git state, ensure earlier active plans are reconciled, return to current `main`, create a short-lived implementation branch, execute this plan in order, update this single file's checklists/status truthfully, and follow repository verification/PR policy.

Plan closure is not authorization to deploy/restart production or perform irreversible external changes; those remain separate operator actions.
