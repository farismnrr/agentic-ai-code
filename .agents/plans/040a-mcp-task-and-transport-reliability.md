# Plan 040A — MCP Task and Transport Reliability Foundation

**Status:** CLOSED / VERIFIED / MERGED
**Parent:** [Plan 040](040-git-github-delivery-roadmap.md)
**Depends on:** Plan 039 CLOSED / VERIFIED / MERGED
**Operator runtime rule:** Follow [Plans 040–042 execution guide](../roadmap-execution-guide-040-042.md). Do not restart/redeploy the relay or resync/recreate the external MCP connector/action. Batch source/build/deterministic work first; when a genuine live-runtime proof requires a new runtime, stop and give the user the exact reviewed restart/deploy command(s) and external MCP client resync steps. Only continue live proof after the user performs that checkpoint.

## Goal

Make MCP calls resilient enough for real coding delivery work before adding new Git/GitHub mutation surfaces. Preserve the low-latency synchronous path for fast operations, while moving operations that can legitimately outlive an HTTP/client/proxy request window onto a standards-aligned durable task lifecycle with bounded polling/progress, cancellation, retained results, retry/resume behavior, and explicit deadlines.

This plan extends the CLOSED Plan 037 job/task foundation; it must not create a second process runner or duplicate terminal job state.

## Confirmed baseline gaps

Current repository inspection shows:

- Plan 037 already provides one authoritative long-running terminal job manager plus MCP Tasks/fallback job tools;
- first-party `server/infrastructure/mcp/client.ts` recognizes task results for `terminal_exec`, but other potentially slow future tools are not yet governed by a generic slow-operation contract;
- the first-party task waiter polls `tasks/get` on a fixed 50 ms loop, which is unnecessarily aggressive for long operations;
- individual MCP `fetch()` requests do not currently apply an explicit per-request AbortSignal/deadline in the first-party client;
- the request advertises `Accept: application/json, text/event-stream`, while the current response path consumes JSON directly rather than implementing a complete SSE/resumption path;
- a historical positive outbound `http_fetch` path has been documented as environmentally unproven/pre-existing and should be revalidated while network reliability is being hardened;
- external MCP client/third-party clients may impose request/proxy/UI time limits outside this repository, so server correctness cannot depend on holding one request open indefinitely.

## Industry-aligned target model

Do **not** make every MCP call asynchronous. Use a hybrid contract:

1. **Fast synchronous path** for bounded, normally sub-second/short operations.
2. **Durable task path** for work whose latency is external, unbounded, or likely to exceed a conservative request budget: long commands, remote Git/forge mutation, waiting for checks, slow network fetches, and selected multi-agent work.
3. **Explicit request deadlines** on each HTTP round trip, distinct from the lifetime of the underlying task.
4. **Cancellation** that targets the durable task/job rather than merely abandoning the HTTP request.
5. **Bounded progress/status polling** with server-provided or adaptive backoff; no hot 50 ms indefinite polling loop.
6. **Retained terminal result** for a bounded TTL so a transient disconnect does not lose successful work.
7. **Idempotency/retry discipline** for external mutations: retries must not duplicate pushes, PRs, merges, comments, or branch deletion.
8. **Transport resumption where supported** using current MCP Streamable HTTP semantics rather than inventing a proprietary streaming channel.
9. **Progress is activity evidence, not permission for infinite execution**: request/task/operator maximums remain enforceable.
10. **Truthful fallback** for clients that do not negotiate Tasks or resumable streaming.

## Scope

### A. Generic taskability contract

- define which tool classes are always sync, task-eligible, or task-required;
- remove `terminal_exec`-specific assumptions from first-party task awaiting where a generic task result can be handled safely;
- preserve one authoritative task/job registry and lifecycle;
- support `working`, terminal states, cancellation, and any negotiated input-required state without hidden client hangs;
- expose bounded task TTL/result retention semantics.

### B. HTTP request deadline discipline

- add explicit configurable per-round-trip timeout/AbortSignal to the first-party MCP HTTP client;
- distinguish HTTP request timeout from task execution timeout;
- cancellation after a request timeout must not falsely imply the underlying durable task was cancelled;
- classify timeout, transport disconnect, task failure, task cancellation, and malformed response separately in internal telemetry while keeping public errors sanitized.

### C. Polling/progress/backoff

- replace fixed hot polling with bounded adaptive/server-hinted cadence;
- honor task poll interval / retry hints when available;
- define minimum/maximum poll cadence and jitter where appropriate;
- avoid thundering-herd behavior with multiple tasks/agents;
- progress notifications may refresh liveness UX but never disable absolute task/operator deadlines.

### D. Streamable HTTP / reconnection behavior

- verify the exact MCP protocol revision implemented by this repository and current external MCP client-compatible behavior;
- either implement the advertised SSE response path correctly, including resumability semantics where negotiated, or narrow `Accept`/capabilities so the client does not claim behavior it does not consume;
- disconnect is not equivalent to task cancellation;
- repeated delivery/replay must not duplicate side effects.

### E. Network-tool reliability

- revalidate `http_fetch` and `web_search` timeout/cancellation/retry behavior against the current relay/network policy;
- distinguish DNS failure, connect timeout, upstream timeout, policy denial, and caller cancellation internally without leaking raw sensitive diagnostics;
- do not add blind automatic retries for non-idempotent requests;
- prove positive outbound canaries from the actual deployed execution path when operator policy permits.

### F. Observability and SLO evidence

Reuse the existing Plan-035/039 action-level telemetry and sanitizer boundaries rather than creating per-poll telemetry noise. This phase must preserve bounded lifecycle timing/result events, classify first-party round-trip expiry as the existing safe `TimeoutError`/`timeout` class, and keep network failure classes static/sanitized. Detailed cross-runtime metrics or log-bridge expansion remains owned by Plan 041C unless a concrete 040A debugging gap proves it necessary.

Define practical acceptance budgets instead of one universal timeout; never emit task arguments/results, raw URLs, credentials, private paths, or high-cardinality task IDs merely to diagnose polling.

## Security and correctness requirements

- no global "no timeout" default for HTTP round trips;
- no broad terminal network/credential exposure as a reliability workaround;
- no task retry may duplicate an external mutation;
- task IDs/handles are unguessable or scoped so another user/session cannot control unrelated work;
- cancellation remains cooperative at protocol level but authoritative process-tree cleanup remains enforced for local jobs;
- retained task results are bounded by count, bytes and TTL;
- malformed task/SSE/resume data fails closed;
- transport reconnect never upgrades capability or approval scope.

## Acceptance matrix

At minimum prove:

1. short native read remains synchronous and low-latency;
2. long terminal task survives multiple HTTP polling requests and completes;
3. a deliberately timed-out HTTP poll does not kill the underlying durable task;
4. explicit task cancellation settles and reaps the process tree;
5. client disconnect followed by supported resume/result retrieval returns exactly one final result;
6. polling cadence is bounded and substantially lower than the current fixed 50 ms loop for long work;
7. task TTL expiry is deterministic and bounded;
8. DNS/connect/upstream timeout classes are distinguishable internally and sanitized externally;
9. positive `http_fetch` and `web_search` deployed canaries work when policy/network permit;
10. external-mutation calls remain synchronous until a later delivery layer provides request-level idempotency/deduplication; deterministic acceptance proves a Tasks-capable client cannot accidentally convert a mutating `http_fetch` call into a durable task;
11. external MCP client live invocation of a task-eligible operation does not remain indefinitely stuck when the client supports the negotiated lifecycle; otherwise fallback behavior is documented truthfully;
12. all Plan 037 long-running/cancellation/security regressions remain green.

## Non-goals

- rewriting Plan 037's process/job manager;
- making every tool asynchronous;
- WebSockets or a proprietary background protocol when MCP Tasks/Streamable HTTP are sufficient;
- infinite task/result persistence;
- retrying arbitrary non-idempotent operations;
- weakening OAuth, approval, sandbox, protected-path or capability policy.

## Exit criteria

- generic sync-vs-task policy is implemented and documented;
- first-party HTTP calls have explicit round-trip deadline/cancellation semantics;
- polling/backoff and result retention are bounded;
- advertised transport behavior matches what the client/server actually implement;
- deployed external MCP client and first-party acceptance cover timeout/disconnect/cancel/retrieve paths;
- historical network-tool reliability gap is revalidated;
- independent review reports zero unresolved P0/P1;
- Plan 037 behavior remains regression-clean;
- 040A is CLOSED / VERIFIED / merged before 040B begins.

## Closure evidence (2026-08-19)

- Generic task eligibility, bounded task polling, per-round-trip HTTP deadlines, task routing headers, fail-safe task results, lifecycle completion observation, catalog v5, and Bubblewrap runtime HOME/toolchain isolation are implemented.
- `pnpm verify:040a` passed three consecutive times on the final source; workspace v1 integration, Rust formatting/checks/tests, Nuxt build, release-tools build, and the named hooks/lifecycle/effect/capability/path/LSP/Git/confidentiality/039H/039I/Plan-037 gates passed.
- The final release binary was deployed and live-verified against the restarted relay: health, authenticated MCP catalog/tool calls, terminal/cargo/Node behavior, protected-path denial, read-like Tasks, mutating HTTP synchronous behavior, polling routing, cancellation/reaping, and explicit network policy all passed.
- Final internal adversarial review found no unresolved P0/P1. Task ownership remains single-owner relay scope with opaque task IDs; multi-user/session binding remains future hardening for a later multi-user deployment plan.
