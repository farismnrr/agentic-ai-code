# Plan 046 — Sandboxed CLI delegation and quota-aware fallback

**Status:** COMPLETE

## Objective

Expose one bounded `agent_delegate` MCP tool in the Full relay profile so an
authorized client can delegate a coding task to an operator-configured order
of three operator-installed coding CLIs with bounded provider-specific adapters.

## Safety contract

- Delegation runs through the existing Bubblewrap process boundary, authorized
  workspace roots, output limits, timeout, cancellation, and job capacity.
- Provider argv is constructed by the relay; the MCP caller supplies only the
  prompt, provider order, workspace, and bounded execution limits.
- Provider permission flags enable non-interactive work only inside the relay
  sandbox. Host-level permission bypass flags are never accepted or generated.
- Attempts run serially. Fallback is permitted only for an explicit provider
  quota/rate-limit/capacity/auth/unavailable classification. A failed provider
  that may have changed the workspace prevents automatic continuation.
- Provider credentials/network access require explicit operator configuration;
  they are not inherited from arbitrary relay request data.

## Acceptance

- Full catalog v10 contains `agent_delegate`; historical v9 remains immutable.
- Primary does not advertise the delegation tool.
- Deterministic tests cover provider argv construction, failure
  classification, fallback ordering, no-fallback-after-mutation, and rejection
  of bypass permission flags.
- `pnpm verify:commit` passes.

## Open operational evidence

Live provider authentication, quota exhaustion, and ChatGPT rediscovery require
an operator-configured CLI installation and are not inferred from source-only
verification.
