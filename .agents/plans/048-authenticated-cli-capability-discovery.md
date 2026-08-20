# Plan 048 — Authenticated CLI capability discovery

Status: CLOSED/VERIFIED

Date: 2026-08-20

Verified: 2026-08-20

## Objective

Make delegated coding-agent providers discoverable only when the local CLI
session is usable, while preserving explicit operator-configured process/auth
root plumbing and the existing quota/auth fallback guard.

## Scope

1. Detect locally authenticated CLI sessions with bounded, noninteractive
   status probes; use the logged-in session by default and never use an
   unverified environment mapping to advertise a provider.
2. Run every provider through its documented headless interface and retain
   edit approval modes without adding permission-bypass flags.
3. Filter the live `tools/list` provider schema and reject direct calls for
   providers that are not currently available.
4. Auto-mount only known local session directories needed by the selected
   provider, while retaining explicit auth-root overrides and sandbox guards.
5. Add deterministic acceptance coverage and update the general MCP/client
   documentation and repository guidance.

## Acceptance

- CLIs with documented login-status commands use their local session checks;
  a CLI without one is advertised only when a supported local/explicit
  authentication source can be proven without spending a model request.
- A provider absent from the capability snapshot is absent from the live
  delegated-provider schema and cannot be selected by a direct call.
- Provider execution uses headless/structured output modes and no
  permission-bypass or API-key flags by default.
- Quota exhaustion, authentication failure, and unavailable-provider fallback
  remain bounded; ordinary task failures still stop the chain.
- `pnpm verify:commit` passes, including the new deterministic acceptance
  checks and synchronized `.agents/`/`docs/` guidance.

## Closeout

Before closure, record the final invariant in `.agents/memories/README.md`,
update this file to `CLOSED/VERIFIED` with the verification timestamp, and
run the repository self-improvement closeout checklist.

## Verification

- `pnpm verify:commit` — PASS.
- Plan-048 capability example and live filtered-catalog fixture — PASS.
- Static v11 contract remains unchanged; the runtime catalog is filtered from
  that superset by the startup capability snapshot.
