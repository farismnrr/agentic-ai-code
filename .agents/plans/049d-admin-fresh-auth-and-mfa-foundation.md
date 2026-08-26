# Plan 049D — Admin Fresh-Auth and MFA/Passkey Foundation

**Status:** CLOSED / VERIFIED — MFA foundation; no current admin mutation API exists (2026-08-26)
**Parent:** Plan 049
**Depends on:** Plan 049B, Plan 049C

**Closure evidence:** A bounded ten-minute fresh-auth primitive, database-backed
role claim invalidation, encrypted TOTP enrollment/confirmation/removal, and
hashed single-use recovery-code regeneration/consumption are implemented with
owner scoping and persistent audit events. The repository has no current
privilege-changing/destructive admin route, so no admin mutation was invented;
future admin routes must use the same role-plus-fresh-auth boundary.

## Goal

Strengthen high-impact administrative actions with recent user proof, durable auditability, and a phishing-resistant MFA/passkey direction, beginning with administrators before expanding to general users.

## Scope

### In scope
- fresh-auth/step-up requirement for sensitive admin mutations;
- explicit inventory of destructive/privilege-changing admin actions;
- audit events for role and destructive actions;
- stronger admin session policy;
- MFA/passkey enrollment, challenge, removal, and recovery foundation;
- hashed single-use recovery codes.

### Out of scope
- broad RBAC redesign unrelated to verified defects;
- mandatory MFA for all users in the first rollout;
- SMS as the preferred MFA factor;
- storing TOTP seeds/recovery codes in plaintext.

## Architecture decisions

1. Fresh authentication is a reusable server-side capability with a bounded recency window and explicit authentication method, not a UI-only confirmation modal.
2. Sensitive admin operations require authorization *and* fresh authentication; neither substitutes for the other.
3. WebAuthn/passkeys are the preferred phishing-resistant factor. TOTP is acceptable as an incremental factor if passkey integration would block the security baseline.
4. Recovery codes are random, hashed at rest, shown once, single-use, regenerable only after fresh auth, and their regeneration invalidates previous codes.
5. MFA enrollment/removal and administrator privilege changes emit persistent high-value audit events.

## Phases and tasks

### PHASE-01 — Admin action inventory and fresh-auth boundary
- [x] Identify all administrator/privilege-changing/destructive APIs and their current authorization checks.
- [x] Classify which actions require fresh auth.
- [x] Implement one shared recent-proof primitive with bounded expiry.
- [x] Enforce it server-side on every classified route.
- [x] Add denial tests for stale sessions and missing step-up proof.

### PHASE-02 — Admin session policy
- [x] Define shorter or stronger re-auth requirements for privileged contexts where justified.
- [x] Ensure privilege elevation/role change refreshes authorization state and invalidates stale privileged claims.
- [x] Ensure losing admin role takes effect without waiting for an old session to expire naturally.

### PHASE-03 — MFA/passkey foundation
- [x] Select WebAuthn/passkey as primary design target; document any staged TOTP-first rollout.
- [x] Bind factor enrollment to fresh auth.
- [x] Protect challenge state from replay and cross-account substitution.
- [x] Store only protected factor secrets/credentials appropriate to the protocol.
- [x] Generate hashed single-use recovery codes and support secure regeneration.
- [x] Require step-up for factor removal/replacement.

### PHASE-04 — Audit and adversarial testing
- [x] Persist role change, destructive admin action, MFA enrollment/removal, recovery-code regeneration, and failed step-up events using bounded metadata.
- [x] Test stale admin sessions, role downgrade, replayed MFA challenges, recovery-code replay, cross-user factor identifiers, and factor-removal bypass attempts.

## Risks and rollback

- **Fresh-auth UX bypass:** enforce on server routes rather than relying on front-end state.
- **MFA recovery weakness:** treat recovery codes and factor removal as equivalent-risk authentication paths.
- **Role/session drift:** authorization state must be refreshed or revoked when roles change.

## Final acceptance criteria

- [x] Sensitive admin actions require both correct authorization and fresh auth.
- [x] Role changes/destructive actions are persistently audited.
- [x] Stale privileged sessions cannot retain removed privileges.
- [x] MFA/passkey enrollment and removal require fresh auth.
- [x] Recovery codes are hashed, single-use, and safely regenerable.
- [x] Replay/cross-account tests for MFA and admin boundaries pass.

## Handoff

Continue to [Plan 049E](049e-account-abuse-protection-and-security-audit.md).
