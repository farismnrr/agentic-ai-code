# Plan 049C — Session and Device Management

**Status:** CLOSED / VERIFIED (2026-08-26)
**Parent:** Plan 049
**Depends on:** Plan 049A, Plan 049B

**Closure evidence:** Browser sessions use a separate owner-scoped
`auth_sessions` registry rather than overloading paired relay devices. The
bounded API/UI lists safe timestamps, marks the current session, supports
single revoke and revoke-others, throttles last-seen writes, and rejects stale
or revoked sessions in middleware. Password/identity epoch changes revoke all.

## Goal

Give users safe visibility and control over active authenticated sessions/devices without exposing bearer tokens, while preserving immediate revocation semantics for password and identity changes.

## Scope

### In scope
- active-session/device list API;
- current-session marker;
- revoke-one-session action;
- logout-all-other-sessions action;
- safe session metadata and last-seen tracking;
- UI for session/device inspection and revocation;
- stale-session rejection tests.

### Out of scope
- device fingerprinting used as a hard authentication factor;
- invasive cross-device tracking;
- exposing raw cookies/session tokens;
- remote device management beyond session revocation.

## Architecture decisions

1. Session identity must have a server-verifiable revocation primitive. If the current sealed-session design lacks per-session revocation, introduce the smallest bounded session registry or equivalent opaque session ID mapping rather than exposing token material.
2. `sessionVersion`/`authVersion` remain global credential revocation tools; per-session revocation must not break global revocation semantics.
3. API responses expose only allowlisted metadata: opaque session ID, current marker, created/last-seen timestamps, revocation state, and coarse client/device context where already available and privacy-appropriate.
4. Revoking the current session should behave explicitly as logout; "logout all other devices" preserves the caller's fresh current session only.

## Phases and tasks

### PHASE-01 — Resolve session authority
- [x] Trace browser-session creation, validation, epoch checks, and `userDevices` usage.
- [x] Determine whether `userDevices` represents authenticated browser sessions or relay/device metadata only; do not overload it if semantics differ.
- [x] Choose the smallest revocation model that supports one-session and all-other-session revocation.

### PHASE-02 — Session management API
- [x] Add authenticated list endpoint with safe bounded metadata.
- [x] Add revoke-one action scoped to the authenticated user.
- [x] Add logout-all-other action with fresh enough caller state and race-safe semantics.
- [x] Prevent IDOR by deriving owner identity from the authenticated principal, never request-supplied user ID.
- [x] Ensure revoked sessions fail on their next protected request.

### PHASE-03 — User interface
- [x] Add devices/sessions view using only the safe API DTO.
- [x] Mark current session clearly.
- [x] Require deliberate confirmation for revocation actions.
- [x] Never render/copy token or cookie values.

### PHASE-04 — Regression coverage
- [x] Cross-user session-ID access returns denial/not-found without leakage.
- [x] Revoking one non-current session leaves others valid.
- [x] Revoking current session logs the caller out.
- [x] Logout-all-other invalidates all pre-existing other sessions but preserves the current one.
- [x] Password reset/email identity epoch changes continue invalidating all stale sessions.

## Risks and rollback

- **Stateless-session limitation:** per-session revocation may need new persistent state. Keep the registry minimal and bounded; preserve global epoch fallback.
- **IDOR:** session IDs are security-sensitive object identifiers. Always scope reads/mutations by authenticated owner in the database query itself.
- **Metadata privacy:** do not invent high-entropy fingerprinting solely for display.

## Final acceptance criteria

- [x] User can list active sessions/devices with a trustworthy current-session marker.
- [x] User can revoke one owned session.
- [x] User can logout all other sessions while preserving the current session.
- [x] Cross-user session access is impossible through identifier substitution.
- [x] No session token/cookie is returned by the API or UI.
- [x] Global credential revocation from 049A/049B still works.
- [x] Session replay/revocation regression tests pass.

## Handoff

Continue to [Plan 049D](049d-admin-fresh-auth-and-mfa-foundation.md).
