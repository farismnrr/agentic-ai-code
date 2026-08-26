# Plan 049B — Email Verification and Secure Email Change

**Status:** CLOSED / VERIFIED (2026-08-26)
**Parent:** Plan 049
**Depends on:** Plan 049A

**Closure evidence:** Verification consumption is now atomic and expiry-aware;
resend is bounded and revokes older verification tokens; primary-email changes
require current-password proof, create pending state, confirm the new address,
notify the old address, enforce uniqueness at commit, and revoke stale browser
sessions. Ordinary settings updates no longer write the primary email.

## Goal

Prove ownership of primary email addresses and make email changes resistant to stale-session takeover through fresh authentication, confirmation of the new address, notification of the old address, and safe identity/session refresh.

## Scope

### In scope
- registration email verification;
- verification-token lifecycle using the shared hashed-token primitive;
- fresh-auth requirement before primary-email change;
- pending email-change state and confirmation to the new address;
- notification to the old address;
- identity/session refresh or revocation after successful change;
- duplicate-address/concurrency safety and anti-enumeration-safe public behavior.

### Out of scope
- MFA implementation itself;
- organization-domain ownership policy;
- arbitrary profile-field verification.

## Key decisions

1. Reuse the verified hashed, short-lived, single-use token primitive from 049A.
2. Do not change `users.email` until new-address ownership is confirmed.
3. Email change requires fresh auth even when the current session is otherwise valid.
4. Successful change must invalidate or refresh stale identity claims so old sessions cannot continue asserting the previous principal state.
5. Notify the previous address after completion without giving that message a bearer link that can silently reverse ownership unless a separately reviewed recovery design exists.
6. Database uniqueness remains authoritative under concurrent attempts.

## Phases and tasks

### PHASE-01 — Registration verification
- [x] Review `register.post.ts`, `verify.post.ts`, token persistence, and login policy.
- [x] Ensure registration creates/sends a hashed-at-rest `email_verify` token with bounded TTL.
- [x] Make verification single-use and atomic when setting `emailVerifiedAt`.
- [x] Decide and enforce what unverified users may access; do not leave enforcement implicit.
- [x] Add resend behavior with bounded throttling and no secret leakage.

**Validation:** valid, expired, replayed, wrong-user, resend, and already-verified cases are deterministic.

### PHASE-02 — Fresh-auth email change
- [x] Add or verify a reusable recent-auth proof primitive.
- [x] Require recent proof before accepting an email-change request.
- [x] Store only bounded pending-change state; do not immediately replace the primary email.
- [x] Send confirmation to the new address using a hashed, short-lived, single-use token.
- [x] On consume, atomically enforce uniqueness and update primary email.
- [x] Notify the old address after successful change.
- [x] Advance the appropriate identity/session epoch or otherwise force safe identity refresh.

**Validation:** stale session, missing fresh auth, duplicate new address, replayed confirmation, concurrent confirmation, and old-session access all fail safely.

### PHASE-03 — Regression coverage
- [x] Add API/integration tests for register verification and email change.
- [x] Assert public errors do not disclose whether arbitrary addresses are registered beyond product-required authenticated context.
- [x] Assert logs/telemetry never contain raw verification tokens.
- [x] Run repository verification gates.

## Risks and rollback

- **Identity drift:** stale session claims can preserve old email after DB mutation. Mitigate by explicit session/identity epoch refresh.
- **Address squatting race:** two users may target one address. Let a database uniqueness constraint be final authority and map conflict to a safe bounded error.
- **Fresh-auth bypass:** do not infer freshness from session existence; require explicit recent proof.

## Final acceptance criteria

- [x] New registrations have a tested email-ownership verification path.
- [x] Verification tokens are hashed, short-lived, single-use, and replay-safe.
- [x] Email change requires fresh auth.
- [x] New address is confirmed before becoming primary.
- [x] Old address receives a security notification after change.
- [x] Stale sessions/identity claims are revoked or safely refreshed.
- [x] Duplicate/concurrent email changes fail safely.
- [x] Secret-bearing links/tokens are absent from logs and telemetry.

## Handoff

Continue to [Plan 049C](049c-session-and-device-management.md), reusing the session revocation semantics already proven by 049A/049B.
