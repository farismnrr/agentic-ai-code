# Plan 041C — Observability and Debugging Polish

**Status:** IMPLEMENTED / VERIFIED — MERGE PENDING
**Parent:** [Plan 041](041-code-intelligence-and-platform-polish-roadmap.md)
**Depends on:** 041B CLOSED / VERIFIED

## Goal

Close only measured observability/debugging gaps that remain after Plans 039–041B, using the existing OpenTelemetry/logging stack and sanitizer contracts.

## Scope

- identify real debugging questions that current traces/logs cannot answer;
- improve correlation between agent turn, tool call, Git/delivery action, background/subagent work and provider request where needed;
- evaluate whether the currently deferred Rust log-to-OTel bridge materially improves operations;
- add bounded health/diagnostic classifications only when they avoid raw error/content logging;
- improve operator documentation for tracing a failed action end-to-end.

## Rules

- no new event database or duplicate audit store;
- no prompts/source/patch/tool args/auth headers/private paths/raw provider errors in telemetry;
- do not add telemetry merely because it is possible;
- every new semantic field must have a concrete debugging use and sanitizer coverage;
- cardinality remains bounded.

## Acceptance

Demonstrate a small set of real debugging scenarios, such as:

- identify which bounded tool/action failed within a turn;
- correlate a remote delivery operation to its policy decision and result without exposing secrets;
- distinguish timeout/cancellation/policy-deny/provider failure classes;
- verify sanitizer behavior for all new fields.

## Verified implementation outcome

- direct request/application spans now pass attributes through the same `sanitizeAttributes()` allowlist used by structured logs, closing a direct-span sanitizer bypass class;
- MCP approval evaluation emits bounded `chat.tool.policy` telemetry so a policy-denied action remains observable even when tool execution never starts;
- MCP tool failures classify results with the existing secret-safe cause classifier (`cancelled`, `timeout`, bounded runtime/provider code, or `unclassified`) rather than collapsing every failure to a generic `error` result class;
- operator documentation now gives a request/trace-first debugging flow without recommending raw arguments, provider responses, source, private paths, credentials, or exception text;
- `pnpm verify:041c`, `pnpm verify:039j`, full typecheck, architecture, maintainability, subagent/background/task-context gates, and the current Plan-039H contract pass in the isolated task worktree. The terminal-sandbox worktree metadata limitation prevents the Git-dependent tail of `verify:commit` / `phase-039i-contract.sh` from running there; this is an environment limitation, not a source failure, and Git-native MCP status/diff remain healthy for the branch.

## Exit criteria

- measured observability gaps are closed or explicitly deemed unnecessary;
- sanitizer/confidentiality tests pass;
- no duplicate telemetry architecture is introduced;
- final review reports zero unresolved P0/P1;
- Plan 041 can be closed before Plan 042 begins.
