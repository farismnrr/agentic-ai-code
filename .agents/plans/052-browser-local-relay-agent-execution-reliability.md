# Plan 052 — Browser-Local Relay Agent Execution Reliability

**Status:** OPEN — deferred from closed Plan 051
**Created:** 2026-08-27

## Context

Plan 051 delivered and deployed the unified Settings → MCP experience and aligned the laptop's systemd user service with the browser-local relay contract. The current runtime baseline is healthy:

- `ai-tools-relay.service` is active under the user systemd manager;
- the browser-local relay uses local mode, loopback binding, port `47821`, and the exact page Origin `http://100.99.88.53:3333`;
- `/health` and a correctly shaped direct MCP `tools/call` succeed;
- the configured Origin is reflected exactly and an unrelated Origin is rejected;
- the Nuxt app container is healthy after the targeted recreate.

During a browser Agent Mode attempt, the `local_terminal` card showed the generic **Tool execution failed** state. Relay evidence for that attempt reached the relay but was rejected by strict MCP routing-header validation. A direct request with the intended browser contract subsequently completed `git status` successfully, so the failure is not currently attributed to service downtime, listener binding, or CORS.

The browser Network panel also showed frequent `/api/conversations/<id>/tasks` and `/context` requests. The former is the Nuxt Agent Mode's ephemeral progress ledger, refreshed while the ledger is visible; it is separate from MCP `tasks/get` polling used when a relay call is actually task-backed. The UI should make these two concepts understandable without treating a successful ledger poll as command-execution proof.

## Goal

Make browser-local Agent Mode tool execution reliable and diagnosable after the Plan 051 connection flow, while preserving the strict MCP transport contract, exact browser Origin allowlist, authentication, and local relay execution boundary.

## Non-goals

- Do not weaken MCP routing-header/body validation.
- Do not add wildcard CORS, disable Origin checks, or disable authentication.
- Do not change the local relay into a remote-mode service or use `0.0.0.0` as a browser/client URL.
- Do not move terminal execution into Nitro/server-side shell execution.
- Do not redesign the Agent Mode task ledger unless the investigation proves a user-facing change is required.

## Investigation and implementation tasks

### TASK-001 — Capture the browser-to-relay contract safely

- Reproduce a fresh browser Agent Mode `local_terminal` call after a hard refresh.
- Compare the browser's `/mcp` Request Headers/Payload with the current `useRelayAgent()` contract and the relay's strict validation requirements.
- Add or improve bounded, non-secret failure classification if the existing generic error hides the actionable reason. Never log cookies, authorization, tool arguments, command output, or private paths.
- Distinguish stale frontend assets, approval/controller state, malformed routing headers, and actual relay/tool failures before changing behavior.

### TASK-002 — Fix the owning browser or relay boundary

- Correct only the layer proven to be at fault.
- Keep `mcp-protocol-version`, `mcp-method`, `mcp-name`, body method/name, and `_meta` protocol/capability consistency aligned with the supported MCP revision.
- Preserve the browser-local loopback URL, exact configured Origin, and relay local-mode security checks.
- Add focused feature tests for the regression and generic error confidentiality when a source change is required.

### TASK-003 — Verify execution and explain task polling

- Run the relevant web/Rust gates only for the changed stack or shared contract.
- Recreate the browser-local flow in Agent Mode and prove a short `git status` call completes through the actual UI.
- Verify a genuinely task-backed call, if used by the client, polls MCP task state to completion or bounded failure without an indefinite loop.
- Verify the Agent Mode progress ledger remains bounded/ephemeral and that its polling does not masquerade as relay execution evidence.
- Re-run `/health`, configured-Origin preflight/MCP calls, and unrelated-Origin rejection after any deployment change.

## Acceptance criteria

- [ ] A fresh browser Agent Mode `local_terminal` call completes successfully through the live local relay.
- [ ] The browser request and relay validation use the same supported MCP routing contract.
- [ ] Failure feedback identifies the safe actionable class without exposing secrets, raw commands, output, credentials, or private paths.
- [ ] Exact browser Origin CORS remains allowlisted; unrelated Origin remains rejected; no wildcard CORS is introduced.
- [ ] Local relay remains a systemd user service in local mode with loopback binding for the browser-local flow.
- [ ] MCP task polling, where applicable, is bounded and separate from the Nuxt progress-ledger polling.
- [ ] `pnpm guardrail` and all other applicable stack-scoped gates pass.
- [ ] Any repository change is delivered from a short-lived branch through a PR into `main`; host-only adjustments are not fabricated into repository changes.

## Delivery boundary

Start from the current `main` after re-checking the live service and browser evidence. Preserve unrelated worktrees, stashes, unit files, wrappers, and environment configuration. If the fix is browser/source-owned, use the repository branch → commit → push → PR → merge workflow. If the remaining issue is only host-local state, record the verified operator action without changing repository source.
