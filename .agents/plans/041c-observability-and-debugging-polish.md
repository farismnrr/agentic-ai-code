# Plan 041C — Observability and Debugging Polish

**Status:** CLOSED / VERIFIED / MERGED (2026-08-19)
**Parent:** [Plan 041](041-code-intelligence-and-platform-polish-roadmap.md)
**Depends on:** 041B CLOSED / VERIFIED

## Goal

Close only measured observability/debugging gaps using the existing telemetry/logging architecture. Do not build a second event system or emit sensitive payloads merely to make debugging easier.

## Scope

- review agent/tool/action telemetry coverage against current Plan-039/040 flows;
- ensure policy-denied actions remain observable even when tool execution never starts;
- preserve bounded failure classifications so timeout/cancellation/runtime/provider classes do not collapse into generic errors;
- ensure direct OTel span attributes use the same sanitizer/allowlist contract as structured logs;
- provide a practical request/trace-first operator debugging flow;
- re-evaluate whether Rust needs a separate log-to-OTel bridge rather than assuming it does.

## Non-goals

- raw prompt/tool/source/patch/provider payload logging;
- arbitrary provider error text in telemetry;
- new telemetry storage/event databases;
- a second tracing architecture;
- product analytics expansion unrelated to debugging.

## Debugging scenarios

Demonstrate a small set of real debugging scenarios, such as:

- trace one tool action from policy decision to bounded execution result;
- distinguish timeout/cancellation/policy-deny/provider failure classes;
- verify sanitizer behavior for all new fields.

## Verified implementation outcome

- direct request/application spans now pass attributes through the same `sanitizeAttributes()` allowlist used by structured logs, closing a direct-span sanitizer bypass class;
- MCP approval evaluation emits bounded `chat.tool.policy` telemetry so a policy-denied action remains observable even when tool execution never starts;
- MCP tool failures classify results with the existing secret-safe cause classifier (`cancelled`, `timeout`, bounded runtime/provider code, or `unclassified`) rather than collapsing every failure to a generic `error` result class;
- operator documentation now gives a request/trace-first debugging flow without recommending raw arguments, provider responses, source, private paths, credentials, or exception text;
- Rust already uses the first-party `tracing` subscriber with `tracing_opentelemetry` plus W3C `traceparent` propagation on `relay.request`, so a separate log-to-OTel bridge was reviewed and deemed unnecessary;
- `pnpm verify:041c`, `pnpm verify:039j`, full typecheck, lint/fmt/clippy, architecture, maintainability, subagent/background/task-context gates, and the current Plan-039H contract pass in the isolated task worktree. The terminal-sandbox worktree metadata limitation prevented the Git-dependent tail of `verify:commit` / `phase-039i-contract.sh` from running there; native Git MCP status/diff/commit/push remained healthy for the branch;
- final internal adversarial review found zero unresolved P0/P1 in the 041C scope;
- implementation landed through PR #148; remote `main` advanced to merge commit `8a02ccb46e7587b6ff3d31bf4ec98e7af0125a82`.

## Exit criteria

- [x] measured observability gaps are closed or explicitly deemed unnecessary;
- [x] sanitizer/confidentiality tests pass;
- [x] no duplicate telemetry architecture is introduced;
- [x] final review reports zero unresolved P0/P1;
- [x] Plan 041 can be closed before Plan 042 begins.
