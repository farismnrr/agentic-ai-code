# Plans 040–042 — Sequential Execution and Known-Gap Ownership Guide

**Status:** PLANNED / GUIDE
**Created:** 2026-08-19

## Purpose

Keep the next three initiatives deliberately sequential and make every currently known relevant gap owned by exactly one upcoming plan. This file is a guide, not a fourth initiative.

## Mandatory order

```text
Plan 040
  040A -> 040B -> 040C -> 040D -> 040E -> 040F
  CLOSE / VERIFY / MERGE
        ↓
Plan 041
  041A -> 041B -> 041C
  CLOSE / VERIFY / MERGE
        ↓
Plan 042
  042A -> 042B -> 042C -> 042D
  CLOSE / VERIFY / MERGE
```

Rules:

1. Implement one child plan only.
2. Run focused acceptance and repository gates for that child.
3. Run independent review when the child changes security, authorization, remote mutation, task lifecycle, concurrency or confidentiality boundaries.
4. Merge/close the child before starting the next sibling.
5. Re-read current `main` at every child-plan handoff; never execute a later child from stale assumptions.
6. Do not parallelize sibling implementations in separate worktrees merely to finish the roadmap faster.
7. Read-only exploration/review subagents are allowed; implementation ownership remains one child at a time.
8. If a later child uncovers an earlier-foundation defect, remediate/reverify the foundation before continuing.
9. **Do not restart or redeploy the relay merely because a phase/child plan finished.** Batch implementation and deterministic verification against source/test binaries first. Preserve the currently running relay unless a live-runtime proof truly requires loading a new binary/configuration.
10. **Relay restart/redeployment is operator-owned.** Agents/assistants must not run `systemctl --user restart`, kill/relaunch the relay, replace the installed live binary, mutate the user service, or otherwise disrupt the active connector on their own. When a restart becomes necessary, stop at that boundary and give the user the exact reviewed commands to run.
11. **external MCP connector/action resync is operator-owned too.** Do not attempt to force/recreate/resync the user's external MCP connection. If the MCP catalog/schema/runtime needs a fresh external MCP client snapshot, provide the exact resync/reconnect steps only when the new runtime is ready for live proof.
12. Prefer a **single coordinated restart/resync checkpoint** after the largest safe batch of implementation is complete, rather than one restart per phase. Multiple restarts require a concrete reason (for example incompatible runtime state or a live regression that cannot be tested otherwise).

## Operator-owned restart/resync checkpoint

The normal execution pattern is:

```text
implement child/phase work
  -> deterministic/source-level verification
  -> build reviewed release candidate when needed
  -> continue with additional work that does not require the running relay to change
  -> reach a genuine live-runtime proof boundary
  -> STOP and hand the operator exact restart/deploy commands
  -> operator restarts/redeploys relay
  -> STOP and hand the operator external MCP connector/action resync steps if catalog/schema changed
  -> operator resyncs/reconnects external MCP client
  -> resume live proof against the exact installed/runtime artifact
```

A plan must never mark a live-runtime criterion passed using stale runtime state. Conversely, lack of a restart is not a blocker while source/build/deterministic work can still proceed truthfully.

## Current known-gap ownership

| Known gap / deferred capability | Owner | Reason / disposition |
| --- | --- | --- |
| external MCP calls can appear stuck or time out around slow operations | **040A** | Separate short HTTP request lifetime from durable task lifetime; explicit deadlines, cancellation, retained result and reconnect/retrieval |
| First-party task polling currently uses a fixed 50 ms loop | **040A** | Replace with bounded adaptive/server-hinted polling and backoff |
| First-party MCP fetch round trips lack explicit per-request deadline/AbortSignal semantics | **040A** | Add HTTP deadline discipline distinct from task execution deadline |
| Client advertises JSON + SSE while current path primarily parses JSON | **040A** | Make advertised Streamable HTTP behavior match implemented consumption/resumption behavior |
| Historical positive outbound `http_fetch` reliability remained environmentally unproven | **040A** | Revalidate network timeout/DNS/connect/cancel behavior and deployed positive canaries |
| Plan 037 terminal task/job foundation | **040A dependency, not reimplementation** | Reuse one authoritative job manager and preserve its regressions |
| Native local Git mutations/conflicts are not first-class bounded MCP workflows | **040B** | Branch/commit/merge/rebase/continue/abort + structured conflict state |
| Remote Git fetch/push depends too much on general networked terminal behavior | **040C** | Narrow credential-isolated remote transport with ref/origin validation |
| GitHub/`gh` credentials should not be exposed to ordinary Bubblewrap terminal | **040D** | Narrow forge adapter/privileged bridge with forge-neutral model contract |
| PR/check/review/merge/remote branch lifecycle is not first-class | **040E** | Explicit change-request lifecycle, waiting/check status, merge policy and cleanup |
| Full delivery loop needs coherent approvals/reliability/UX/audit evidence | **040F** | Integrate 040A–E and close Plan 040 with live proof |
| Rust workspace symbol search currently unsupported in this build | **041A** | Investigate/complete real LSP capability or return truthful unsupported result |
| Vue definition/references/hover/diagnostics documented as empty with installed stack | **041A** | Re-investigate upstream/plugin configuration and improve only if real semantics are available |
| Deprecated/transitive dependency/toolchain warnings (for example old `glob` through Nuxt/Nitro/archiver) | **041B** | Upgrade through owning dependency where safe; do not force unsafe transitive overrides |
| Historical Workspace-v1 content-hash idea | **Not scheduled unless evidence justifies it** | Exact-match + operation-time identity protections already cover current edit contract; avoid speculative complexity |
| Historical semantic-search idea | **Not scheduled unless lexical/code intelligence proves insufficient** | Existing search/LSP should remain preferred until a measured retrieval gap exists |
| Rust logs-to-OTel bridge and remaining trace/debug correlation gaps | **041C** | Only implement measured observability needs through existing telemetry/sanitizer architecture |
| Parent-only subagents lack explicit multi-agent task graph/shared coordination | **042A–C** | Introduce bounded parent-owned task graph, routing, concurrency and evidence reconciliation rather than peer swarm |
| Background-agent stale task metadata after process restart was intentionally deferred | **042A** | Define bounded recovery or explicit invalidation/reconciliation; never silently reuse stale ownership |
| Peer-to-peer teams/shared task lists were intentionally deferred | **042** | Replace with parent-owned orchestrator semantics; peers do not grant authority to each other |
| Multi-agent UX/observability/security closure | **042D** | Final adversarial/live closure after task graph/concurrency/reconciliation are stable |

## Reliability architecture guidance

Do not interpret “async” as “everything returns a job ID.” The target is hybrid:

```text
bounded fast operation
    -> synchronous MCP response

potentially slow/external operation
    -> durable MCP task
       -> bounded polling/progress
       -> cancel/retrieve/resume
       -> bounded terminal result retention
```

Prefer standards-aligned MCP Tasks + Streamable HTTP behavior over custom background protocols. Third-party client request/proxy timeouts are outside the relay's control, so successful long work must not depend on a single HTTP connection surviving for the entire operation.

External mutations require idempotency/deduplication before automatic retry/resume. A timeout or disconnect means “result currently unknown” until task/remote state is re-observed; it must never be translated directly into “operation failed” or “safe to repeat.”

## Markdown audit snapshot

A repository markdown audit performed while preparing this guide considered tracked repository markdown plus the new Plan 040–042 files, excluding generated caches/toolchains. At that point:

- relevant markdown files checked: 155 before this guide was added;
- broken relative/local markdown links found: 0;
- historical `deferred`, `unproven`, `limitation`, and `unsupported` mentions were reviewed for relevance;
- historical evidence/closed-plan limitations remain historical unless they correspond to a still-relevant capability in the ownership table above;
- no placeholder-only TODO/TBD is intentionally introduced in Plans 040–042.

This audit is not a claim that every historical limitation should be implemented. Gaps without current evidence of user impact remain intentionally unscheduled, as recorded above.

## Completion discipline

At the end of each child plan, update:

- its own status/evidence;
- its parent roadmap checklist;
- this guide only if gap ownership or sequencing changed;
- durable memory/docs only for facts that remain useful after the implementation branch disappears.

Do not mark a parent plan CLOSED merely because its implementation exists. Closure requires the child-by-child verification and live evidence specified by that parent.
