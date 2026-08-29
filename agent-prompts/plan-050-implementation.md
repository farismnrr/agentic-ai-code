# Plan 050 Implementation Agent Instructions

Implement `.agents/plans/050-workspace-activity-ledger-roadmap.md` end-to-end in the `ai-code` repository.

This file is an execution prompt, not a separate plan. The single source of truth for scope, phases, tasks, architecture, and Definition of Done remains `.agents/plans/050-workspace-activity-ledger-roadmap.md`.

## Mission

Complete Plan 050 to production-quality standards. Do not merely make checklist text look complete. Inspect the current source, reconcile the plan against reality, implement what is actually missing, validate the composed behavior, and update Plan 050 truthfully as work progresses.

Plan 050 now also includes two execution-runtime requirements that supersede older delegated-agent wording in this brief:

1. **Remove the product/runtime `agent_delegate` feature** and the provider-specific coding-CLI plumbing that exists only to support it. This is separate from your ability to use platform-native sub-agents while implementing/reviewing the plan.
2. **Add agent-controlled reliable execution:** eligible tools accept caller-selected bounded `timeout_ms` and explicit `execution_mode: sync | async | auto`, with standard resumable/cancellable MCP Tasks for async execution so accepted work does not restart merely because one RPC/client session times out.

Do not create `050A`, `050B`, or other child plan files. Keep Plan 050 as one numbered plan with phases inside it.

## Start safely

Before mutation:

1. Read and obey `AGENTS.md`, `.agents/README.md`, relevant `.agents/knowledge/**`, `.agents/memories/README.md`, `ai-self/CONSTITUTION.md`, `ai-self/project.yaml`, and relevant `ai-self/skills/**`.
2. Read the entire `.agents/plans/050-workspace-activity-ledger-roadmap.md`.
3. Verify repository identity and `origin` match `https://github.com/farismnrr/ai-code.git`.
4. Inspect current branch, HEAD, upstream, worktrees, recent history, staged/unstaged/untracked files, and active merge/rebase state.
5. Do not overwrite, absorb, reset, clean, or stage unrelated work.
6. If Plan 050 has not started, use a clean dedicated short-lived branch/worktree from current `main`, preferably `feat/050-workspace-activity-ledger`.
7. If Plan 050 is already active on that dedicated branch/worktree, continue from the current partial implementation and reconcile these new requirements without resetting/restarting completed Plan-050 work. If unrelated previous work is still dirty or undelivered, isolate it safely rather than mixing it in.

Never use `--no-verify`, force push, destructive reset, or hidden credential access.

## Use sub-agents intelligently

Use sub-agents when they materially improve speed, independence, or review quality. Do not delegate blindly and do not keep all work in the parent agent when a bounded independent task is a good fit.

Strong candidates for sub-agent delegation include:

- Rust relay activity contract/journal architecture review;
- SQLite/outbox durability and crash-safety review;
- mutation diff correctness and TOCTOU/security review;
- Nuxt/PostgreSQL persistence, encryption, ingestion, and ownership review;
- Logs UI/UX implementation or review;
- adversarial security review;
- independent final falsification/review;
- targeted source discovery where multiple subsystems can be inspected in parallel.

Delegation rules:

- Give each sub-agent a narrow explicit scope, relevant files, expected output, and acceptance criteria.
- Make each sub-agent read current source and Plan 050 before acting.
- Do not let two write-capable agents edit the same files concurrently.
- Prefer isolated Git worktrees for parallel write-capable agents.
- Parent agent owns architecture, integration, security decisions, verification, Git delivery, and final truth.
- Treat sub-agent output as evidence/input, not automatically trusted truth.
- Parent must review every sub-agent change/finding before integration.
- Use a fresh independent sub-agent near closure to try to falsify the final implementation.

## Execution-runtime simplification and reliability

### Remove `agent_delegate`

The current product/runtime delegation feature is intentionally being removed.

Remove `agent_delegate` from the current Full/Primary MCP catalog and runtime, including task/normal dispatch, hook/effect policy, capability filtering, current contracts, current acceptance scripts, operator/client docs, and provider-specific configuration/discovery/sandbox/auth-root code that exists only to launch external coding CLIs.

Do not leave a hidden alias that still launches provider-specific coding CLIs. Preserve historical Plan 046/048 records as historical truth; update current contracts/docs rather than rewriting history.

This does **not** mean you should avoid sub-agents while implementing Plan 050. Platform-native sub-agents used for bounded engineering/review work are separate from the removed MCP `agent_delegate` product surface. Do not remove unrelated first-party multi-agent/sub-agent functionality unless source tracing proves it is only another path into the same deprecated provider-CLI launcher.

### Agent-selected timeout

For eligible blocking tools, the agent/client chooses the requested `timeout_ms` appropriate to the operation. The relay remains authoritative for tool-specific and operator-configured maximums.

Requirements:

- remove the Primary-only 30-second `terminal_exec` restriction;
- do not make tool timeout depend on Full vs Primary profile;
- preserve/introduce explicit operator/tool ceilings and deterministic rejection when exceeded;
- define `timeout_ms = 0` clearly: deadline-free only when operator policy permits it;
- keep tool execution deadline separate from MCP HTTP request timeout, task lifetime, retention TTL, and cancellation grace period;
- expose only safe requested/effective timeout metadata in activity/task presentation.

### Explicit sync / async / auto

Reviewed task-capable tools should accept one explicit execution preference:

- `sync` — wait for the direct result subject to requested/effective timeout;
- `async` — return promptly with a standard MCP task identity and continue independently of the initiating request;
- `auto` — follow one deterministic documented policy based on tool eligibility/client capability.

At minimum cover `terminal_exec`, safe task-capable `http_fetch` methods, and `web_search` after `agent_delegate` removal. Review remote Git/forge/network operations that genuinely suffer long-running timeout pressure, but only add async support when their lifecycle, cancellation, and idempotency semantics are sound. Do not add async knobs to every tiny bounded read tool.

Primary and Full should advertise MCP Tasks when they can actually honor them. If `async` is explicitly requested and the client does not support Tasks, return a clear bounded capability error; never silently downgrade to sync.

Prefer standard MCP Tasks over proliferating tool-specific `*_job_start` APIs. Existing `terminal_job_start/get/cancel` may remain temporarily for compatibility, but first-party/documented workflows should converge on the standard task lifecycle.

Accepted async work must survive the initiating RPC disconnect/timeout. Polling/reconnect/cancellation should operate on the same logical task/activity. Mutating async operations must use/reuse a stable bounded execution/idempotency identity where necessary so a lost response/retry does not blindly execute the mutation twice.

Verification must prove: Primary timeout >30s within policy, over-max rejection, direct sync, prompt async task return, poll-to-completion, cancellation/process cleanup, unsupported-client async failure, deterministic auto behavior, lost-response/reconnect dedupe, and one stable activity lifecycle.

## Core product outcome

Deliver one persistent chronological activity history per workspace covering all relevant operations actually mediated by the relay, regardless of caller.

Required caller/execution coverage includes:

- first-party Nuxt MCP;
- paired/local terminal;
- remote MCP clients;
- direct external MCP where current connector/deployment access permits live verification;
- synchronous native tools;
- task/job lifecycle;
- Git and code/LSP operations;
- terminal/process execution;
- background/task execution through the relay task lifecycle.

The user-facing Logs experience must truthfully show reads/searches/actions/mutations, actor/source facts, target, status, duration, change counts/evidence, and historical structured diffs where the relay can prove them.

## Architectural invariants

### Relay is the observation authority

Record workspace activity at the relay execution boundary. Browser callbacks, chat rendering, frontend `addToolOutput`, model responses, and OTel telemetry are not authoritative evidence that an operation occurred.

### Local terminal is mandatory

The first-party local path (`local-tool-controller` -> `useRelayAgent` -> loopback MCP relay -> `terminal_exec`) must use the same relay activity recorder as other clients. Do not add a frontend-only logger.

### Product activity is separate from telemetry

Preserve Plan-035/039J confidentiality. Do not copy activity payloads into Loki, Jaeger, OTel attributes, stdout/stderr, or ordinary server logs.

Never leak there:

- source/diff/file contents;
- patch contents;
- prompts/messages;
- raw tool request/result JSON;
- arbitrary terminal stdout/stderr;
- auth headers/cookies/tokens;
- environment variables;
- provider credentials;
- private absolute paths when unnecessary.

### Durable relay-local journal/outbox

Implement the Plan 050 durable journal boundary. In required mode, a workspace-scoped operation must obtain a durable local `started` record before execution. Sink/network failure after local durability must not silently lose activity and must not put Nuxt network latency on the normal execution critical path.

Required behavior includes:

- stable relay source identity across restart;
- encrypted source-bearing local payload storage;
- bounded durable journal/outbox;
- async authenticated idempotent exporter;
- retry/backoff for retryable sink failures;
- explicit 401/403 degraded/revoked behavior;
- acknowledged-row pruning only after durable server receipt;
- no silent pruning/overwrite of unacknowledged rows to satisfy quota;
- required-mode fail-closed if the journal is unwritable/full/corrupt or cannot durably admit a new start;
- restart recovery that preserves evidence and marks stale nonterminal operations interrupted/unknown rather than inventing success.

Prefer a mature transactional embedded store such as SQLite if current dependency/build audit supports the design. Do not replace durability with an ad-hoc text log merely to avoid a dependency.

### Authoritative workspace identity

Do not trust a client-supplied Nuxt workspace UUID as activity authority.

Resolve workspace scope through existing relay canonical path policy and `WorkspaceAllowlist`/containing-root behavior. Ingestion maps a relay-derived opaque/canonical-root fingerprint to a user-owned Nuxt workspace. Nested cwd maps to its containing authorized root. Global/no-root operations must not be falsely attached to a workspace.

Cross-user, stale, ambiguous, or foreign-root mappings fail closed.

### Truthful actor identity

Keep these separate:

- authenticated activity source identity;
- transport/channel/auth mode;
- bounded client-reported MCP `clientInfo`;
- optional opaque/hashed OAuth client fingerprint where useful;
- UI display label.

`clientInfo` is display metadata only and never grants authority.

Show a specific external-client label only if actual request metadata or another reviewed verifiable mapping supports it. Otherwise display the generic label `External MCP client`; never infer client identity from timing, User-Agent heuristics, or assumptions.

Ensure paired/local relay requests send the first-party clientInfo consistently.

### Exact change evidence only where provable

For structured mutations such as `file_edit`, `file_write`, and `apply_patch`, generate historical evidence from authoritative before/after state owned by the mutation path before old state is discarded.

Addition/deletion counts must derive from stored evidence, not caller claims.

For Git working-tree mutations, terminal commands, or opaque background/task execution, do not manufacture exact provenance. Use explicit evidence classes such as:

- `exact`;
- `summary`;
- `unavailable`;
- `not_applicable`.

Use `no_change` only when the reviewed evidence mechanism actually proves it. Binary/non-UTF8 data must not be lossily coerced into text diffs.

### Secure Nuxt/PostgreSQL read model

PostgreSQL is the product history/read model; relay SQLite is execution-side durability/outbox only.

Implement the Plan 050 server boundary, including:

- scoped relay activity source enrollment;
- high-entropy raw token shown once;
- hash-only token persistence;
- source revocation;
- source/user/root/workspace binding;
- idempotent bounded batch ingestion;
- legal lifecycle state transitions;
- duplicate/out-of-order delivery reconciliation;
- encrypted source-bearing payload persistence;
- ownership-enforced list/detail/diff/clear APIs;
- deterministic cursor pagination rather than offsets;
- bounded retention;
- workspace deletion semantics;
- clear-history watermark/source-sequence semantics so delayed pre-clear relay events do not resurrect cleared history.

Every user-facing activity route must reassert workspace ownership server-side. Frontend filtering/state is never authorization.

### Purpose-separated encryption

Use authenticated encryption for source-bearing activity payloads. Do not blindly reuse provider-secret key material. Use a dedicated activity key or explicitly reviewed domain separation.

Wrong keys or tampered ciphertext must fail closed without plaintext/ciphertext leakage in logs/errors.

List/live responses remain metadata-only. Full historical diffs are lazy-loaded only after ownership checks.

## UI requirements

Add a stable `Logs` destination under each workspace in the existing sidebar. Do not place raw activity entries inside the sidebar.

Provide a dedicated workspace Logs page using existing Nuxt/Nuxt UI conventions. It should support, according to Plan 050:

- chronological timeline;
- timestamp/relative time;
- truthful actor/source;
- operation/tool/category;
- relative target summary;
- running/ok/error/denied/cancelled/interrupted state;
- duration;
- affected-file count;
- `+N/-N` where evidence supports it;
- exact/summary/unavailable evidence indicator;
- bounded filters;
- route-query filter persistence where useful;
- cursor-based loading of older history;
- running -> terminal updates without duplicate/regressed rows;
- reconnect behavior;
- lazy detail view;
- safe historical diff viewer;
- retention/clear-history UX;
- loading/error/empty/degraded states;
- dark/light theme support;
- responsive layout;
- keyboard/accessibility behavior.

Use semantic theme classes and existing components. Reserve primary/cyan emphasis for active/running state instead of decoration. Render source/diffs as inert text; never use `v-html` for untrusted source content.

Avoid unnecessary expansion of public composable/file counts or speculative abstractions. Split components/modules only by real responsibility.

## Live update model

Durable database history/cursor is the source of truth.

Use same-origin resumable SSE only if the installed Nitro/H3/Postgres stack supports it cleanly. Otherwise use bounded near-live cursor polling while the page is visible. Do not make correctness depend on process-local in-memory pub/sub.

Reconnect must resume from durable cursor. Never send full diff payload through the live list/update channel.

## Security requirements

Treat workspace activity history as sensitive product data.

Do not store arbitrary raw MCP argument/result blobs.

Use fixed, bounded per-tool presentation schemas. Particular care:

- file reads: metadata/path/range/count, never returned source content;
- search: bounded/redacted query representation and result counts, not raw results;
- terminal: executable and reviewed bounded/redacted argv facts, never environment or arbitrary stdout/stderr;
- Git: structured operation/ref/path/status facts, never credential-helper output;
- LSP/code: bounded operation/path/symbol/result metadata, not arbitrary source text;
- background/task execution: task lifecycle/execution-mode/timeout/effect/change-evidence facts, never hidden reasoning/private transcript scraping.

Existing protected-path, symlink/no-follow, OAuth/scope/owner, approval/hook, terminal network, Git policy, task cancellation, and telemetry boundaries remain authoritative. Activity instrumentation must never introduce a weaker secondary read or authorization path.

## Database/migration discipline

Plan 049 may have changed current schema/migration numbering. Inspect current migration state at implementation time.

Use repository-standard Drizzle generation. Review generated SQL. Test migration on a disposable representative DB. Do not overwrite migration history or assume plan-era migration numbers remain valid.

Keep Nuxt layering intact:

`server/api -> server/application <- server/infrastructure`

Application code must not import H3/Drizzle/provider implementation details.

Keep Rust crate/layer ownership intact. SQLite/HTTP/exporter/encryption implementation details belong in infrastructure behind narrow application-facing contracts.

## Work phase-by-phase

Use the six phases already inside Plan 050. Do not create child plans.

For each phase:

1. inspect relevant current source before coding;
2. reconcile any stale plan assumptions;
3. implement the smallest coherent architecture-compliant change;
4. use sub-agents for bounded parallel work/review where useful;
5. review all changes before integration;
6. run focused deterministic verification;
7. fix confirmed findings before moving on;
8. update the Plan 050 checklist/status/evidence truthfully.

Do not mark a task complete just because a file exists.

## Verification model

The repository intentionally has no CI and no unit-test suite. Do not introduce one just for Plan 050. Follow existing deterministic acceptance/security script patterns.

Build focused Plan 050 verification covering the real behavior where practical, including:

- event contract/version/resource limits;
- workspace scope/root fingerprint rules;
- clientInfo/source attribution independence from authorization;
- required-mode durable pre-execution start;
- off-mode compatibility;
- journal crash/restart recovery;
- sink outage/retry/backoff;
- 401/403 behavior;
- journal quota/full/unwritable/corrupt failure;
- duplicate/out-of-order/concurrent ingestion;
- terminal-state non-regression;
- task/job completion/cancellation/restart;
- current catalogs/runtime contain no `agent_delegate` or provider-only coding-CLI execution surface;
- Primary `terminal_exec` accepts caller-selected timeout above 30 seconds when within operator/tool policy;
- operator/tool timeout maxima reject out-of-policy requests before execution;
- explicit sync direct-result behavior;
- explicit async task creation/poll/cancel/reconnect behavior;
- async request from a client without Tasks capability fails clearly without silent sync downgrade;
- deterministic auto-mode behavior;
- lost async response/retry converges on stable task identity where required and does not duplicate accepted mutating work;
- local-terminal capture;
- background/task capture;
- exact file_edit/file_write/apply_patch diffs;
- Git and opaque-writer truthfulness;
- source token hash/revoke behavior;
- cross-user/cross-workspace/root-binding attacks;
- encryption tampering/wrong-key behavior;
- plaintext canary sweeps;
- metadata-only list/live responses;
- detail/diff ownership;
- cursor correctness under inserts;
- retention;
- clear-history delayed-outbox race;
- live reconnect;
- resource/body/string/payload caps;
- existing relay/app security regressions.

Re-run all existing deterministic guards affected by the implementation.

## Plaintext/telemetry canary sweep

Before closure, place controlled canaries in representative diff, command/search, path, and fake-token inputs and inspect relevant surfaces.

Forbidden plaintext must not appear in:

- relay SQLite main/WAL where payload should be encrypted;
- PostgreSQL metadata/plaintext columns where payload should be encrypted;
- relay stdout/stderr;
- Nuxt stdout/stderr;
- Loki;
- Jaeger/OTel;
- list/live API responses;
- unrelated API errors;
- browser console/network traffic other than an explicitly authorized diff fetch.

Do not use real secrets for testing.

## Performance and boundedness

Measure rather than guess.

Ensure:

- exporter network latency is not on normal tool critical path after local durability;
- local journal work remains bounded;
- payload/diff generation respects existing file/tool size ceilings and explicit Plan 050 caps;
- UI does not load/render unbounded history;
- retention/pruning is bounded and safe across retries/multi-instance operation.

Do not silently weaken durability/fsync/security guarantees merely to hit a benchmark. If measured overhead is problematic, profile and fix the design.

## Live acceptance

Where current infrastructure and authorization permit, prove the composed path rather than only source-level behavior.

### Paired/local terminal

Use the real first-party `local-tool-controller -> useRelayAgent -> loopback relay` path. Execute harmless workspace-scoped operations and confirm durable relay journal, Nuxt ingestion, workspace Logs row, truthful local actor/channel facts, and persistence across browser refresh/reopen. Include one caller-selected timeout above the former Primary 30-second ceiling when policy permits and one explicit async operation that is polled to completion instead of being restarted.

### First-party Nuxt MCP

Execute representative first-party MCP activity against the target workspace, including a safe structured mutation if normal policy permits. Confirm one activity per real tool operation and correct historical diff for the structured mutation.

### Direct remote MCP / external MCP client

When current authenticated connector/deployment access and operator authorization permit, execute a harmless direct external MCP read/search and optionally a controlled mutation. Confirm the activity reaches the same workspace history without Nuxt being the execution origin.

Inspect actual delivered client metadata. Display a specific client label only if evidence supports it; otherwise keep generic external-client labeling and mark exact actor identity `UNPROVEN`.

If deployment/restart/credential/operator action is required and not authorized, do not fabricate the result. Finish all local work and mark only that live case `UNPROVEN`.

### Generic MCP compatibility

Verify a standards-compliant fixture client both with and without optional clientInfo. Plan 050 must remain vendor-neutral.

## Independent final falsification

Near completion, assign a fresh sub-agent an independent review of the composed implementation. It must try to break the result rather than merely repeat the implementation report.

Review at least:

- data-loss/crash windows;
- journaling before execution;
- duplicate/idempotency lifecycle races;
- async task duplication after a lost create response/reconnect;
- timeout policy bypass or unintended unbounded execution;
- sync/async/auto mode confusion or capability downgrade;
- residual `agent_delegate`/provider-CLI launch paths after removal;
- terminal-state regression;
- incorrect workspace mapping/IDOR;
- source-token scope/revocation/replay;
- actor spoofing/overclaim;
- payload/key handling;
- plaintext/telemetry leakage;
- clear-history resurrection;
- retention races;
- multi-instance live behavior;
- TOCTOU/protected-path regressions introduced by diff capture;
- local-terminal bypass;
- excessive complexity/maintainability;
- architecture layer violations;
- misleading Plan 050 closure claims.

Fix all confirmed P0/P1 findings before closure. Do not defer a Plan 050 P0/P1 to a new plan solely to close this one.

## Required final gates

At minimum on the final candidate, run the repository-authoritative gates applicable to the implementation, including:

- `pnpm verify:commit`
- `pnpm build`
- `pnpm build:tools`
- `git diff --check`

Run every Plan-050 verifier introduced by the implementation. Run `pnpm audit` and `cargo audit` when their dependency/security surfaces changed.

For UI changes, build fresh and verify the production preview in a browser; do not treat a stale dev watcher as acceptance.

For Rust relay changes, verify actual transport/journal behavior with production-shaped deterministic fixtures rather than source grep alone.

## Git delivery discipline

Review task-owned diffs before staging. Stage only Plan-050-owned files. Use logical Conventional Commits and repository-local hooks. Never bypass hooks or signing/branch protections.

Push the dedicated branch and use the repository PR/review workflow when source is ready.

Do not merge, deploy, restart production services, publish a release, or perform irreversible remote operations unless separately authorized by the user/operator.

## Documentation and durable knowledge

At closure reconcile only what changed in durable truth, including as applicable:

- `.agents/plans/050-workspace-activity-ledger-roadmap.md`;
- `.agents/memories/README.md`;
- relevant `.agents/knowledge/**`;
- `docs/architecture.md`;
- `docs/configuration.md`;
- `docs/remote-mcp.md`;
- `packages/relay-agent/SKILL.md`;
- `.env.example`.

Do not recreate child Plan-050 files. Do not create duplicate memory files. Persist reusable lessons through the repository self-improvement process only when genuinely durable.

## Definition of Done

Plan 050 is ready for closure only when current evidence proves all applicable plan acceptance criteria, including:

- one ordered persistent activity timeline per workspace;
- consistent activity contract across first-party/local/remote/sync-async task callers;
- `agent_delegate` and provider-specific coding-CLI delegation removed from current catalogs/runtime/config/docs without breaking unrelated native/sub-agent capabilities;
- caller-selected bounded timeouts work independently of Full/Primary profile, including >30s Primary terminal requests when policy permits;
- reviewed long-running tools support deterministic `sync`/`async`/`auto`, with resumable/cancellable async task identity and no blind restart after client/RPC timeout;
- required-mode durable pre-execution journaling;
- sink outage/restart without silent accepted-operation loss;
- duplicate/reordered delivery convergence;
- canonical-root-based ownership-safe workspace attribution;
- exact historical diffs for supported structured file mutations;
- truthful non-exact evidence for opaque writers;
- encrypted source-bearing activity payloads;
- activity/OTel-Loki separation;
- scoped hash-at-rest revocable activity sources;
- per-workspace Logs UI with filtering/pagination/live/detail/diff behavior;
- deterministic retention/clear-history semantics;
- local-terminal and first-party MCP end-to-end evidence;
- external MCP activity evidence when access permits, otherwise explicit `UNPROVEN` for that exact live case;
- generic MCP compatibility;
- relevant existing security regressions green;
- fresh independent review with zero unresolved P0/P1 findings;
- repository verification/build/audit gates green;
- source/docs/memory/Plan 050 status reconciled truthfully.

Do not mark Plan 050 `CLOSED / VERIFIED` merely because implementation exists. Closure status must match the evidence actually obtained.

## Final report

Report concisely but completely:

1. starting branch/HEAD/worktree isolation;
2. architecture implemented;
3. Plan 050 phase completion;
4. sub-agent usage — assignment, scope, result, findings, and parent review/integration decision;
5. security/data-integrity findings and remediation;
6. exact verification commands and results;
7. live acceptance results, explicitly marking unavailable cases `UNPROVEN`;
8. commits/branch/push/PR state;
9. real remaining blockers/limitations;
10. final verdict: `PLAN 050 READY FOR CLOSURE: YES / NO`.

If NO, list exact remaining work. If YES, update Plan 050 to `CLOSED / VERIFIED` only when its own evidence supports that state.
