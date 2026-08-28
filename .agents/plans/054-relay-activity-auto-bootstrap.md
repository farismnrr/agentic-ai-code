# Plan 054 — Relay Activity Auto-Bootstrap

**Status:** CLOSED / VERIFIED
**Created:** 2026-08-28

## Problem

Workspace Logs is backed by the Plan 050 activity ledger, not by OpenTelemetry. The live relay currently runs with no `RELAY_ACTIVITY_*` configuration, and the application database has no enrolled relay activity source, no workspace bindings, and no activity rows. Therefore both Nuxt-originated and external ChatGPT-originated relay calls execute successfully but leave the workspace activity UI empty.

The existing configuration requires manual source enrollment, one-time secret transfer, workspace binding, and relay process environment changes. That is deployment-oriented plumbing rather than acceptable product UX for a relay already connected through authenticated MCP.

## Goal

Make activity capture self-configuring for the first-party relay while preserving the relay as the authoritative execution boundary:

- Nuxt enrolls an owned activity source and binds owned workspaces.
- Nuxt sends the activity sink + one-time source credential to the already-authenticated first-party relay over a private MCP protocol extension.
- The relay stores the bootstrap config owner-only, activates the activity recorder/exporter without requiring a process restart, and reloads the persisted bootstrap on future restarts.
- Activity remains recorded at relay execution time, so calls from both Nuxt and external MCP clients such as ChatGPT appear in the same workspace Logs view.
- Arbitrary third-party MCP servers are never offered or sent the activity bootstrap credential.

## Security constraints

- Bootstrap is a non-model-facing MCP protocol extension, not a tool in `tools/list`.
- Only a relay that explicitly advertises the activity-bootstrap capability may receive the credential.
- The activity source token is never returned to browser code, logs, telemetry, or model context.
- Persisted relay bootstrap data uses an owner-only local file and validates HTTPS sink URLs before activation.
- Existing activity ingest token hashing, workspace-root fingerprint binding, payload encryption, retention, and ownership checks remain authoritative.
- Unknown/external MCP servers remain ineligible even if they expose similarly named tools.

## Implementation

1. Add a relay activity-bootstrap capability to `server/discover` and a private `server/activity_configure` protocol method.
2. Introduce a reloadable activity recorder that can atomically swap from Noop/startup config to a newly bootstrapped runtime.
3. Persist validated bootstrap configuration under the existing relay activity state directory and load it at startup.
4. Extend the Nuxt modern MCP client with capability discovery + activity configuration support.
5. Add a server-side activity bootstrap use case that enrolls/reuses one source, binds all owned workspaces, and configures only a compatible first-party relay.
6. Run bootstrap automatically after successful OAuth MCP verification and expose an idempotent repair endpoint for already-connected relays.
7. Add cross-stack tests covering capability advertisement, secret non-exposure, persisted restart behavior, workspace binding, and non-first-party fail-closed behavior.
8. Deploy Nuxt + relay, repair the currently connected relay, execute a read-only relay action from Nuxt/connector, and verify the workspace Logs API/UI receives activity.

## Definition of done

- Web and Rust tests for the changed cross-stack contract pass.
- `pnpm guardrail` passes.
- Production web build and release relay build pass.
- Existing connected relay can be repaired without editing systemd environment files.
- Relay restart preserves activity delivery.
- Workspace Logs shows new activity produced through the shared relay execution boundary.

## Closure evidence

- The first-party OAuth relay was bootstrapped without adding `RELAY_ACTIVITY_*` systemd environment configuration; source enrollment and workspace bindings are application-managed.
- Relay bootstrap state persisted owner-only and remained configured after reconnect/restart testing performed during implementation.
- A production routing defect was found during E2E: the global browser auth middleware also guarded `/api/**`, so the relay exporter received `302 /login?redirect=/api/activity/ingest` before endpoint-owned bearer authentication could run. `app/middleware/auth.global.ts` now leaves API authentication to server handlers.
- Production Nuxt was rebuilt and redeployed after the routing fix. `GET /api/activity/ingest` now reaches Nitro routing (`404` for unsupported GET) instead of browser-auth redirecting.
- The activity source is claimed and has been seen by ingestion. Workspace `917c2181-63ff-412e-ba47-7baa71251c1b` contained 82 activity rows in the production read model immediately after the fix, including fresh relay events with `channel=relay` and actor `External MCP client`.
- No systemd relay changes were required for the final fix or closure.
