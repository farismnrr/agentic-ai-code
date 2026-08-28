# Plan 056 — Task-Level Telegram Completion Notifications

**Status:** IMPLEMENTED — DEPLOYED; LIVE TELEGRAM DELIVERY PENDING OPERATOR CONFIGURATION
**Created:** 2026-08-28

## Problem

The requested notification is a task/plan-level event: after one complete
implementation task finishes, send one Telegram notification. It must not be
coupled to every `tools/call`, terminal job, activity row, assistant stream
chunk, or UI progress update.

The current repository has the following relevant boundaries:

- Relay activity is intentionally tool-level and records each tool's start and
  outcome. It is not a task-completion source.
- `server/application/task-context-output.ts` describes its task ledger as
  ephemeral UI state and explicitly says it does not prove validation, hooks,
  Git delivery, or repository completion.
- `server/application/orchestration/task-graph.ts` owns an in-memory graph
  whose aggregate status becomes `completed` only after all graph nodes settle.
  This is the closest existing Nuxt-side representation of “one task = one
  plan”, but it currently has no notification observer or durable completion
  event.
- The Rust relay has no Telegram capability today. The installed Hermes setup
  already has a Telegram gateway, but its Telegram profile is the polling
  owner; Hermes is not currently a configured HTTP notification bridge.

## Goal

Deliver a one-way, server-side Telegram notification for one logical task or
implementation plan when it reaches an explicit successful terminal state.

The first version will:

- represent one plan with one stable `taskId`;
- notify only after the whole task/plan reaches `completed`;
- produce at most one Telegram message for a `taskId`, including across retry,
  duplicate requests, and relay restarts;
- support Nuxt orchestration completion and an explicit ChatGPT/relay
  `task_completed` finalization call through the same relay notifier;
- import the bot token and fixed recipient from Hermes into the relay's
  owner-only encrypted state;
- send a bounded, redacted summary rather than prompts, tool arguments, raw
  command output, credentials, environment values, or full transcripts;
- leave Hermes gateway code, profiles, session stores, and polling ownership
  unchanged.

The initial outbound policy is success-only. `failed`, `blocked`, `cancelled`,
and `budget_exhausted` states remain visible to the task owner but do not send
Telegram messages in this plan.

## Non-goals

- Sending a Telegram message for every tool call, activity event, stream chunk,
  child task, or terminal job.
- Receiving Telegram commands or using Telegram to control the relay.
- Turning the relay into a general-purpose `sendMessage` proxy with arbitrary
  chat IDs, bot tokens, URLs, or message bodies.
- Using the UI task ledger, activity journal, last tool result, conversation
  close, or assistant response completion as a substitute for an explicit
  task/plan completion event.
- Reconfiguring or upgrading Hermes, changing its Telegram gateway, or reading
  its session databases/logs. The relay reads only the two outbound Telegram
  keys from Hermes' owner-only `.env`.
- Copying secrets into the repository, browser runtime, Nuxt public runtime
  config, logs, telemetry attributes, or test fixtures.

## Architecture decision

The Rust relay will own the Telegram outbound adapter and its durable
deduplication ledger. Nuxt will publish a durable task-completion outbox entry
to the relay through an authenticated private MCP method. ChatGPT-facing relay
clients will use a narrowly scoped `task_completed` MCP tool that submits the
same event shape. Both paths converge on one relay-side queue and one fixed
Telegram recipient:

```text
Nuxt graph active -> completed ─┐
                                ├─> task completion event
ChatGPT finalization tool ──────┘       -> relay outbox/dedupe
                                             -> Telegram Bot API sendMessage
                                             -> fixed configured chat
```

This keeps the Telegram bot token on the laptop-side relay, avoids adding a
Telegram secret to the Nuxt deployment, and gives both callers identical
retry/idempotency behavior. Direct Bot API sending is appropriate for this
outbound-only use case; the existing Hermes gateway can continue owning
Telegram polling independently.

## Event contract

Add a versioned task-completion contract shared by the Rust interface, Nuxt
application, and operator documentation. The wire contract should use bounded
MCP JSON fields equivalent to:

- `taskId`: required stable logical task/plan identifier, max 128 characters;
- `title`: required human-readable task title, max 160 characters;
- `summary`: required concise completion summary, max 2,000 characters after
  normalization and credential redaction;
- `completedAt`: server-normalized RFC 3339 timestamp;
- `resultUrl`: optional HTTPS URL to the relevant Nuxt task/result view, max
  2,048 characters, with no credentials, query tokens, or fragments.

The caller must not provide a bot token, chat ID, parse mode, arbitrary Bot API
method, or free-form destination. The source (`nuxt` or `chatgpt`) should be
assigned by the authenticated entry point rather than trusted from user
input. A missing stable `taskId` is rejected; it must never be synthesized
from a conversation ID or the last tool call.

Normalize text before persistence and delivery: strip terminal control/ANSI
sequences, collapse unsafe whitespace, apply the repository's credential
redaction rules, enforce Telegram's message-size bound, and use plain text
formatting rather than user-controlled Markdown/HTML parsing. Persist only the
bounded sanitized notification payload and operational delivery state.

## Implementation

### 1. Freeze the task completion boundary and shared contract

**Outcome:** There is one explicit definition of “task complete” and one
versioned event shape. No existing tool/activity path can accidentally emit a
notification.

**Likely files:**

- `shared/types/` — add the Nuxt task-notification contract and delivery
  result types;
- `server/application/orchestration/task-graph.ts`;
- `server/application/orchestration/scheduler.ts`;
- `server/application/orchestration/reconciliation.ts`;
- `server/application/task-context-output.ts`;
- `packages/rust-tools/interfaces/src/mcp.rs` and/or a focused new interface
  module for the relay event schema;
- `docs/remote-mcp.md` or a focused new operator-facing notification section.

**Steps:**

1. Map every Nuxt graph transition path that can change the aggregate graph
   status, including normal completion, child settlement, cancellation, and
   reconciliation. Define one transition observer for `active -> completed`.
2. Use the graph's stable `graph_id` as the Nuxt `taskId` for orchestrated
   plans. Do not use `TaskLedger` updates or activity rows as completion
   evidence.
3. Define the versioned event and result contracts, field limits, redaction
   rules, and the success-only notification policy in both TypeScript and Rust
   boundaries. Keep the Rust protocol layer transport-independent.
4. Make the completion observer return a single “newly completed” decision;
   repeated recomputation or duplicate node settlement must return no-op.
5. Keep notification publication after the task's own final result/evidence
   has been committed. A notification failure must not change a completed task
   back to running or make the task itself fail.

**Validation:**

- Pure lifecycle tests prove that only the first aggregate transition to
  `completed` emits a completion event.
- Tests prove that `tasks.put`, `updateTaskLedger`, activity ingestion, and
  individual tool completion do not emit one.
- Contract tests reject missing/oversized/unsafe fields and prove that raw
  prompt, tool argument, credential, and environment-shaped content is not
  retained in the notification payload.

### 2. Add the relay-side Telegram outbox and Bot API adapter

**Outcome:** The relay can enqueue and deliver one bounded completion message
  to one fixed Telegram recipient without exposing Telegram as a generic
  network capability.

**Likely files:**

- `packages/rust-tools/core/src/config/cli.rs` and related config validation;
- `packages/rust-tools/infrastructure/src/notifications/` — new module for
  the Telegram client, durable task outbox, redaction/formatting, and worker;
- `packages/rust-tools/infrastructure/src/transport.rs` for shared state and
  worker lifecycle;
- `packages/rust-tools/infrastructure/src/lib.rs` and module exports;
- `packages/rust-tools/infrastructure/src/transport/mcp_http.rs`;
- `packages/rust-tools/infrastructure/src/transport/tools.rs`;
- `packages/rust-tools/application/src/dispatcher.rs`;
- `packages/rust-tools/interfaces/src/mcp/catalog.rs`;
- `packages/rust-tools/interfaces/src/mcp.rs`.

**Steps:**

1. Add server-only configuration loaded from the relay environment:
   `RELAY_TELEGRAM_ENABLED` and an optional
   `RELAY_TELEGRAM_HERMES_ENV` path. On startup, read only
   `TELEGRAM_BOT_TOKEN` and `TELEGRAM_HOME_CHANNEL` from that owner-only
   Hermes file. Never accept the token or target as a CLI argument or MCP
   argument.
2. Add an owner-only, permissioned local SQLite outbox/ledger under the
   existing relay state directory. Keep it separate from the activity schema
   so task notification retention and activity delivery cannot alter each
   other's state. Use WAL/full-sync behavior, owner-only directory/file modes,
   a unique `taskId` constraint, and a separate owner-only AES-GCM key file
   for the imported bot token.
3. Store only the sanitized bounded message, task ID, timestamps, attempt
   count, next-attempt time, and a sanitized delivery error category. Recover
   an in-flight row to retryable state after a process crash.
4. Implement a fixed HTTPS Bot API client that calls only Telegram
   `sendMessage` for the configured recipient. Never accept a destination,
   token, API method, or arbitrary URL from the event. Do not use
   user-controlled Markdown/HTML parse modes.
5. Add a bounded asynchronous worker with exponential backoff. Retry network
   failures, rate limits, and server-side 5xx responses; classify ordinary
   4xx validation/auth failures without leaking Telegram response bodies or
   the token. Return `queued`, `already_sent`, `disabled`, or a sanitized
   failure category to callers.
6. Make enqueue idempotent under concurrent duplicate requests: one logical
   `taskId` may create one pending row and one successful Telegram send. A
   retry after an acknowledged send must not send a second message.
7. Emit only redacted operational telemetry: event type, bounded task ID,
   queue state, attempt number, and outcome. Never log the token, chat ID,
   full message, HTTP authorization header, or Telegram response body.

**Validation:**

- Rust unit tests cover configuration fail-closed behavior, message bounds,
  credential redaction, fixed destination enforcement, and safe error mapping.
- SQLite tests cover concurrent duplicate enqueue, restart recovery, unique
  task identity, acknowledgement, retry scheduling, and retention.
- HTTP-client tests use an injected fake endpoint and prove that production
  configuration cannot redirect requests to an arbitrary host.
- A fake Telegram server test proves one request for repeated completion calls,
  retry on transient errors, and no retry storm on permanent errors.

### 3. Expose one explicit completion entry point for ChatGPT/relay tasks

**Outcome:** A ChatGPT agent can signal “this whole plan is complete” once,
  while the relay still owns authorization, bounds, dedupe, and delivery.

**Likely files:**

- `packages/rust-tools/interfaces/src/mcp/catalog.rs`;
- `packages/rust-tools/application/src/dispatcher.rs`;
- `packages/rust-tools/infrastructure/src/transport/mcp_http.rs`;
- `packages/rust-tools/infrastructure/src/transport/tools.rs`;
- `packages/rust-tools/infrastructure/src/transport.rs`;
- `server/infrastructure/mcp/modern-http-client.ts`;
- `server/infrastructure/mcp/client.ts` and relevant MCP capability typing;
- `test/unit/` and Rust MCP transport tests.

**Steps:**

1. Add a narrowly scoped `task_completed` tool to the relay catalog. Its
   description must explicitly say that it is called once after the entire
   implementation task and validation finish; it is not a progress/activity
   tool and must not be called after individual tools.
2. Use the existing relay authorization and capability policy. Classify the
   tool as an external network side effect, keep it unavailable in read-only
   or plan-only execution modes, and require the normal authenticated coding
   scope. It must be idempotent but must not be treated as read-only.
3. Dispatch the tool to the shared notifier service rather than through
   `terminal_exec`, `http_fetch`, activity hooks, or a generic HTTP proxy.
4. Add a private first-party `server/task_completed` MCP method for Nuxt. It
   must not appear in `tools/list`; it must use the existing authenticated
   first-party relay path and the same outbox service. Its source is fixed to
   `nuxt`.
5. Advertise a versioned discovery extension, for example
   `io.masihawam/task-completion-notifications: { version: "1", method:
   "server/task_completed" }`, and make the Nuxt client capability-detect it.
   An older relay must fail with a clear unsupported-capability result rather
   than silently claiming notification success.
6. Add `ModernHttpMcpClient.taskCompleted(...)` with strict response
   validation and sanitized errors. The method must be usable by the Nuxt
   outbox worker and must never be exposed to browser code.

**Validation:**

- Rust dispatcher/transport tests prove the private method is not listed as a
  tool, requires authentication, and reaches the same idempotent notifier.
- Catalog tests prove `task_completed` has bounded input, no destination/token
  fields, external-side-effect annotations, and correct profile/policy
  filtering.
- Nuxt client tests cover discovery extension negotiation, accepted queue
  results, already-sent results, unsupported old relays, malformed responses,
  and sanitized authorization/network errors.

### 4. Publish Nuxt orchestration completion through a durable outbox

**Outcome:** Nuxt tasks reliably hand off one completion event even when the
  relay is temporarily unavailable, while task execution remains independent
  of Telegram availability.

**Likely files:**

- `server/database/schema.ts`;
- a new Drizzle migration under the repository's existing migration path;
- `server/application/task-notifications.ts` or an equivalent narrow use-case
  contract;
- `server/infrastructure/database/` task-notification persistence adapter;
- `server/infrastructure/mcp/modern-http-client.ts`;
- `server/infrastructure/composition/application.ts`;
- a server-only delivery plugin/worker alongside existing server plugins;
- `server/application/orchestration/task-graph.ts`;
- `server/application/orchestration/scheduler.ts` and completion call sites;
- `.env.example` only for non-secret enablement/endpoint flags if required.

**Steps:**

1. Add a server-owned outbox table with a unique logical identity (source plus
   `taskId` or an equivalent stable key), bounded sanitized title/summary,
   completion timestamp, delivery state, attempt metadata, and sanitized last
   error. Do not store raw task transcripts or tool payloads.
2. Add an application port for recording a completed task and an infrastructure
   adapter for inserting/upserting the outbox row. Keep Drizzle and `$fetch`
   out of the application orchestration code.
3. Call the port only from the newly completed aggregate graph transition.
   Ensure all scheduler/reconciliation paths converge on that one call and
   that a graph recomputation cannot enqueue again.
4. Add a server-only bounded worker that drains the Nuxt outbox through
   `ModernHttpMcpClient.taskCompleted`. Use backoff and retry classification;
   task completion must remain successful if delivery is delayed or disabled.
5. Never trigger this path from `server/api/conversations/[id]/tasks.put.ts`,
   `buildTaskUpdateTool`, activity ingestion, message persistence, or the
   assistant stream's per-tool lifecycle.
6. Keep the relay source and Nuxt source convergent: a Nuxt completion handoff
   and a later duplicate relay `task_completed` call with the same logical
   task identity must still result in one Telegram message.

**Validation:**

- Web tests prove one outbox row for repeated graph settlement and no row for
  tool-level activity or UI ledger updates.
- Worker tests cover relay downtime, retry/backoff, disabled notifications,
  malformed relay responses, and eventual success.
- Tests prove a completed task response is not changed to failed when Telegram
  delivery is unavailable.
- A migration/schema test proves the uniqueness and bounded-column contract.

### 5. Configuration, documentation, and operator-safe rollout

**Outcome:** The feature can be enabled deliberately on the existing laptop
  deployment without changing Hermes ownership or exposing secrets.

**Likely files:**

- `.env.example` for safe variable names only;
- `docs/mcp-client.md`, `docs/remote-mcp.md`, or a focused Telegram
  notification operations section;
- relay systemd drop-in documentation/examples if the repository already
  documents those deployment files;
- `ai-self/` only if implementation produces a durable, reusable correction or
  skill improvement; do not record task-specific secrets or runtime state.

**Steps:**

1. Document that Hermes remains the source of the real bot token and channel;
   the relay imports them from the owner-only Hermes `.env`, then stores only
   the encrypted token and fixed channel in relay state. Neither is committed
   or printed.
2. Keep the existing Hermes `tele-agent` as the only Telegram polling owner.
   The relay uses outbound `sendMessage` and must not start a second polling
   gateway with the same bot token.
3. Add a safe operator check that reports only `enabled/configured/disabled`
   and never echoes a token, chat ID, request header, or full Telegram error.
   A non-channel Hermes target must report disabled and must not fall back to
   `TELEGRAM_ALLOWED_USERS` or an older private-chat configuration.
4. Document the user-visible message shape and the exact semantic boundary:
   one message after the whole plan is done, not one message per tool.
5. Treat enabling the relay notifier, restarting the relay, and sending a real
   Telegram message as explicit operator/production actions. The notifier may
   be enabled after the Hermes source has a valid channel target, but a real
   message still requires separate live verification.

**Validation:**

- Documentation/config tests prove secret names are present only in server
  configuration examples and no secret values are tracked.
- A controlled staging/runtime check verifies: relay health, discovery
  extension, authenticated Nuxt handoff, one visible Telegram message, and
  exactly one message after repeating the same completion event.
- The check also verifies that individual tool calls and activity events do
  not produce Telegram messages, and that Hermes Telegram polling remains
  owned by the existing profile without a duplicate-token polling process.

## Verification gates

Run only the gates relevant to the changed stacks, then the repository
guardrail:

```sh
pnpm guardrail
```

For implementation closure, use `pnpm guardrail:nuxt` when only Nuxt files
changed, `pnpm guardrail:rust` when only relay files changed, and
`pnpm guardrail:all` when the MCP wire contract is deliberately changed on
both sides. The default pre-commit `pnpm guardrail` auto-detects the changed
stack(s); it does not compile or test the other service for a service-local
change.

The implementation must also run the production-build checks appropriate to
the touched Nuxt and Rust packages. Runtime success must be reported in three
separate categories:

1. static/build and unit-test proof;
2. authenticated relay reachability and capability proof;
3. real authenticated task completion plus visible Telegram delivery proof.

A build, an unauthenticated `401`, or a successful MCP tool call alone is not
evidence that a Telegram message was delivered.

## Risks and mitigations

- **Duplicate notifications:** durable unique task identity, atomic enqueue,
  acknowledgement, and retry recovery prevent normal duplicate sends.
- **False completion:** only an explicit aggregate graph transition or the
  explicit finalization tool can emit; UI task state and activity are excluded.
- **Sensitive data leakage:** bounded semantic fields, server-only secrets,
  redaction before persistence, plain-text formatting, and sanitized logs.
- **Relay downtime:** Nuxt's durable outbox retries without failing the actual
  completed task.
- **Telegram rate limiting or API failure:** bounded worker retries transient
  failures and stops retrying permanent request/auth errors.
- **Hermes polling conflict:** Hermes remains unchanged; only the relay's
  outbound Bot API call is added, with no second `getUpdates`/webhook owner.
- **Old relay binary:** Nuxt capability-detects the versioned private method
  and records unsupported delivery rather than claiming success.
- **In-memory orchestration lifecycle:** the Nuxt outbox is written at the
  completion boundary so notification intent survives a later relay restart;
  the plan does not pretend that an unpersisted graph itself is durable.

## Rollback

Disable the relay notification flag and stop the notification worker through
the normal operator-controlled service configuration. Completed tasks remain
completed; undelivered outbox rows remain inspectable for later retry or
operator-directed cleanup. Restore the previous relay binary if necessary,
without deleting the notification database or Hermes state. Any schema rollback
must be additive/recoverable and must not drop task or activity data.

## Final acceptance

The plan is complete only when all of the following are true:

- one completed implementation plan creates one logical completion event;
- the same event retried concurrently or after restart produces no more than
  one Telegram message;
- Nuxt and ChatGPT/relay completion paths use the same fixed-recipient sender;
- no Telegram message is produced by an individual tool call, activity event,
  UI task update, or assistant stream chunk;
- no bot token, chat ID, prompt, raw tool payload, or credential appears in
  browser output, persisted raw payloads, logs, telemetry, or errors;
- Hermes remains unchanged and continues to be the only configured Telegram
  polling owner;
- build/test, authenticated relay capability, and real visible Telegram
  delivery are verified separately.

## Implementation closure

The implementation is merged into `main` and deployed. It adds the shared
completion contract, Nuxt durable outbox, first-party relay handoff,
relay-owned SQLite dedupe ledger, fixed-recipient Bot API sender, retry worker,
and explicit `task_completed` completion signal. Telegram remains an internal
relay detail; there is no generic `telegram_send` tool and no per-tool
notification path.

Local proof completed on 2026-08-29:

- `pnpm guardrail:all` passed, including repository policy, architecture,
  maintainability, test layout, web lint/typecheck/tests, Rust lint/typecheck,
  and Rust tests.
- Web unit suite passed 29/29, including the task-completion contract test.
- Rust workspace tests passed, including nine task-notification tests covering
  bounds/redaction, invalid fields, dedupe/restart recovery, Hermes import and
  encryption, channel-only validation, credential refresh, private-method /
  catalog boundaries, and disabled delivery.
- Hermes credential values were read only by the local importer, never printed,
  copied into the repository, or included in test output. No real external
  Telegram message was sent during local validation.

Deployment proof completed on 2026-08-29:

- PR 188 was merged into `main` at `c199bd1`, and the feature branch was
  deleted.
- Migration `0028` applied successfully; the Nuxt `app` container was rebuilt
  and recreated from `main`, with root HTTP status `200`.
- The installed relay binary matches the release build byte-for-byte;
  `ai-tools-relay.service` is active/running and its `/health` endpoint returns
  HTTP `200`.
- The unauthenticated relay MCP smoke check returns HTTP `401`, proving the
  auth boundary without claiming an authenticated task handoff.

The remaining external proof is deliberately separate: the configured Hermes
`TELEGRAM_HOME_CHANNEL` must identify a channel, then an operator must verify
authenticated Nuxt handoff plus one visible deduplicated Telegram message.
Until that is done, this plan must not claim live Telegram delivery.

## Delivery closure

The repository lifecycle is complete: the focused implementation passed the
normal guardrail, was pushed and merged through PR 188, the reviewed relay
binary was deployed, the operator-controlled service was restarted, the Nuxt
database migration and container deployment completed, and the final checkout
is clean on `main`. The Hermes source, relay encrypted credential database, and
key file remain outside Git; delivery is not claimed until the configured
Hermes target is a channel and a visible message is verified.
