# Plan 052 — Remote MCP Web Mode

**Status:** CLOSED — implementation complete; runtime credential blocker remains
**Created:** 2026-08-27

## Goal

Make the hosted Nuxt application use the remote, server-side MCP relay path
for Agent Mode. Remove the browser-local relay product surface and prevent
the historical `native.local_terminal` capability from re-enabling client-side
terminal execution.

## Constraints

- The relay is a separately managed systemd user service. This plan does not
  restart or reload it without explicit operator approval.
- The browser must never call the relay directly or receive its bearer token.
- Remote MCP credentials remain server-only and are attached only by the
  existing exact-URL/owner-bound first-party client.
- Existing legacy `native.local_terminal` conversation IDs remain recognized
  only so they are ignored safely; they are not a visible tool or executable
  server tool.
- Existing remote HTTP/SSE MCP rows remain account-scoped, server-verified,
  and approval-gated.

## Implementation

- Remove local relay setup, browser transport, local terminal controller, and
  client-side hook/session execution from the chat and Settings surfaces.
- Make Agent Mode available only when a connected remote MCP tool is selected.
- Make Agent Mode server execution depend on selected remote MCP IDs and use
  the existing server-side `buildMcpTools` path.
- Keep the legacy settings route redirect for old bookmarks.
- Update feature tests and durable agent guidance to describe remote-only web
  execution.

## Validation and delivery

- Proportional web lint, typecheck, unit tests, guardrail, and production build
  pass on the implementation branch.
- Feature tests confirm browser code no longer references the local relay or
  loopback endpoint and that the legacy native ID cannot re-enable execution.
- Repository identity is revalidated before commit/push.
- Delivery uses the required short-lived branch and pull request, then returns
  to a clean local `main`.
- Only the Nuxt app container is recreated after delivery; the relay is not
  restarted or reloaded as part of this plan.

## Deployment blocker to report honestly

The current app container did not expose `NUXT_REMOTE_MCP_URL`,
`NUXT_REMOTE_MCP_OWNER_USER_ID`, or `NUXT_REMOTE_MCP_ACCESS_TOKEN` in its
environment inspection. Source changes can route remote MCP calls correctly,
but the first-party relay connection cannot authenticate until those private
deployment values are configured. Their values must never be committed or
printed in logs.
