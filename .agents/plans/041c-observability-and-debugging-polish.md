# Plan 041C — Observability and Debugging Polish

**Status:** PLANNED
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

## Exit criteria

- measured observability gaps are closed or explicitly deemed unnecessary;
- sanitizer/confidentiality tests pass;
- no duplicate telemetry architecture is introduced;
- independent review reports zero unresolved P0/P1;
- Plan 041 can be closed before Plan 042 begins.
