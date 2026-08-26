# Plan 049A — Account Recovery Foundation

**Status:** CLOSED / VERIFIED (2026-08-26)
**Parent:** Plan 049 — Account and Application Security Hardening Roadmap
**Depends on:** existing password-authentication and session-version/auth-version primitives

**Closure evidence:** Existing and reconciled source implements generic
forgot-password behavior, SHA-256 token storage, fragment-only reset links,
atomic purpose/expiry/replay checks, auth-version/session revocation, bounded
throttling, and sanitized delivery/error paths. `pnpm verify:account-recovery`,
`pnpm verify:account-security`, and the repository gates pass. No raw reset
token/password is emitted by the new flow.

## Goal

Deliver a secure forgotten-password flow with anti-enumeration behavior, cryptographically strong short-lived single-use reset tokens stored only as hashes, atomic password replacement, and revocation of all previously issued sessions.

## Scope

### In scope
- forgot-password request API;
- reset-password consume API;
- reset-token generation, hashing, expiry, persistence, and single-use semantics;
- recovery email delivery boundary;
- global session invalidation after successful reset;
- generic public errors/responses and abuse throttling;
- structured security telemetry without secret leakage;
- deterministic recovery-specific security tests.

### Out of scope
- email-address changes;
- MFA enrollment/recovery;
- user-facing session management UI;
- permanent failed-attempt lockouts;
- production email-provider or deployment changes without explicit approval.

## Verified current state

- `server/api/auth/forgot.post.ts` and `server/api/auth/reset.post.ts` exist in the current working tree.
- `server/database/schema.ts` defines `verification_tokens` with hashed-token primary key, `type`, expiry, and `consumedAt`.
- `server/application/auth.ts` exposes `addVerificationToken` and `consumePasswordReset`.
- the current forgot route returns success for missing users, uses per-IP throttling, creates a generated token, persists its hash, uses a 30-minute expiry, and attempts recovery email delivery;
- the current reset route hashes the submitted token, invokes `consumePasswordReset`, clears the current browser session, and emits structured telemetry;
- these current uncommitted changes are implementation candidates only and must be validated rather than treated as accepted completion.

## Security invariants

1. The raw reset token is never persisted, logged, included in telemetry, returned by the API, or exposed outside the recovery-delivery path.
2. Tokens are generated from a cryptographically secure RNG with sufficient entropy for an online bearer secret.
3. The database stores only a one-way cryptographic hash of the token.
4. Reset tokens expire after a short bounded TTL; 30 minutes is acceptable unless product/security review chooses a shorter value.
5. Successful consumption is atomic: an unconsumed, unexpired password-reset token is consumed exactly once while the password and credential/session epoch are updated in the same transaction or equivalent atomic boundary.
6. Replaying the same reset token after success fails.
7. Expired, consumed, malformed, and unknown reset tokens produce a generic public failure without provider/database details.
8. Forgot-password responses do not reveal whether the account exists through body/status semantics.
9. Recovery-request rate limiting is bounded and cannot permanently lock an account because of attacker-generated requests.
10. Successful password reset invalidates every previously issued browser session; the current browser is cleared explicitly.
11. OAuth-only accounts and accounts without a password have an explicit safe policy; recovery must not accidentally create a password unless that behavior is intentionally supported and reviewed.
12. Email delivery failure must not leak account existence to the caller.

## Phase overview

| Phase | Goal | Depends on | Exit criterion |
| --- | --- | --- | --- |
| PHASE-01 | Reconcile current recovery implementation with required invariants | none | implementation/data-flow review identifies every gap without overwriting unrelated dirty work |
| PHASE-02 | Make token persistence and consumption atomic | PHASE-01 | reset is short-lived, hashed, single-use, replay-safe, and globally session-invalidating |
| PHASE-03 | Harden public API and delivery behavior | PHASE-02 | enumeration, malformed-token, rate-limit, and delivery-failure behavior is safe and bounded |
| PHASE-04 | Add deterministic security acceptance | PHASE-03 | positive and adversarial recovery cases pass repeatably |

## PHASE-01 — Reconcile and threat-model

### TASK-001 — Review the existing dirty recovery changes
**Outcome:** produce an exact implementation gap map before further mutation.

**Files:**
- Review: `server/api/auth/forgot.post.ts`
- Review: `server/api/auth/reset.post.ts`
- Review: `server/application/auth.ts`
- Review: `server/infrastructure/database/auth.ts`
- Review: `server/database/schema.ts`
- Review: `shared/schemas/auth.ts`
- Review: session/auth middleware that validates `sessionVersion` / `authVersion`

**Steps:**
- [x] Confirm the token generator uses a cryptographically secure source and adequate entropy.
- [x] Confirm hashing uses a stable one-way digest suitable for random high-entropy tokens and never logs the raw value.
- [x] Trace `consumePasswordReset` through the database implementation and prove expiry, type, consumed state, password update, and credential/session epoch mutation are one atomic operation.
- [x] Trace all authenticated request paths and prove the relevant epoch is checked on every protected browser-session request.
- [x] Define safe behavior for OAuth-only users.
- [x] Verify forgot/reset validation and rate-limit keys cannot become an attacker-triggered permanent lockout.

**Validation:** documented invariant-to-code mapping with no unresolved ownership ambiguity.

**Commit boundary:** no commit; review/discovery only.

## PHASE-02 — Atomic reset and revocation

### TASK-002 — Enforce reset-token lifecycle in the database boundary
**Outcome:** one valid reset token can change the password once and only once.

**Files:**
- Modify as required: `server/infrastructure/database/auth.ts`
- Modify as required: `server/database/schema.ts`
- Migration path: repository-standard Drizzle migration files if schema changes are required

**Steps:**
- [x] Query by token hash and `type = 'password_reset'`.
- [x] Reject tokens with `consumedAt != null` or `expiresAt <= now`.
- [x] Consume the token and update the password atomically.
- [x] Advance the authoritative session/credential version in the same atomic boundary.
- [x] Ensure concurrent consumption results in one winner and all later/replayed requests failing.

**Validation:** deterministic concurrent/replay test proves exactly one successful consume.

**Commit boundary:** `fix(auth): make password reset single-use and atomic`

### TASK-003 — Prove global session invalidation
**Outcome:** all previously issued sessions become unusable immediately after successful reset.

**Files:**
- Modify only if required: session/auth middleware and auth database/application boundaries discovered in TASK-001

**Steps:**
- [x] Identify the single authoritative session epoch checked by protected requests.
- [x] Make successful reset advance that epoch.
- [x] Clear the reset caller's current browser session.
- [x] Verify an independently issued stale session is rejected on its next protected request.

**Validation:** two-session integration scenario: reset through session A, then both pre-reset A/B credentials fail.

**Commit boundary:** `fix(auth): revoke sessions after password reset`

## PHASE-03 — Public API and recovery delivery hardening

### TASK-004 — Preserve anti-enumeration behavior
**Outcome:** the forgot-password endpoint does not reveal account existence.

**Files:**
- Modify as required: `server/api/auth/forgot.post.ts`

**Steps:**
- [x] Return the same public success envelope for existent and non-existent accounts.
- [x] Do not return provider/email delivery status to the caller.
- [x] Keep internal telemetry bounded and secret-free.
- [x] Review obvious timing asymmetry and, where practical, keep code paths comparable without adding artificial long sleeps.
- [x] Ensure rate-limit behavior does not encode account existence.

**Validation:** existent/non-existent request matrix has equivalent public status/body semantics.

**Commit boundary:** `fix(auth): harden recovery anti-enumeration behavior`

### TASK-005 — Harden reset errors and token handling
**Outcome:** invalid recovery attempts fail generically and safely.

**Files:**
- Modify as required: `server/api/auth/reset.post.ts`
- Modify as required: `shared/schemas/auth.ts`

**Steps:**
- [x] Reject malformed tokens before database work where safe.
- [x] Hash token input before lookup.
- [x] Use one generic public error for unknown/expired/consumed/reset-mismatch cases.
- [x] Never echo the token in errors or telemetry.
- [x] Apply bounded per-source throttling suitable for reset-token guessing abuse.

**Validation:** malformed/unknown/expired/consumed/replayed token matrix produces bounded generic errors and no token reflection.

**Commit boundary:** `fix(auth): harden password reset failure handling`

### TASK-006 — Review recovery email boundary
**Outcome:** recovery email delivery is safe, bounded, and does not create new secret leakage.

**Files:**
- Review/modify as required: current mail application/infrastructure implementation and `server/api/auth/forgot.post.ts`

**Steps:**
- [x] Ensure only the intended recipient receives the raw token URL.
- [x] Ensure application logs never include the reset URL or raw token.
- [x] Ensure site/base URL construction cannot be influenced by untrusted Host headers.
- [x] Ensure mail provider errors are normalized and not returned to the caller.
- [x] Keep delivery retry semantics from minting unbounded active tokens; prefer replacing/revoking older outstanding reset tokens for the same user if consistent with the data model.

**Validation:** mail failure and repeated-request scenarios do not disclose account existence or raw token material.

**Commit boundary:** `fix(auth): harden recovery email delivery`

## PHASE-04 — Security acceptance

### TASK-007 — Add deterministic account-recovery tests
**Outcome:** recovery invariants become regression-protected.

**Files:**
- Create/modify: repository-standard server/API test files discovered during implementation
- Add local guard/acceptance script only if that is the repository's established security-test pattern

**Required cases:**
- [x] existent account forgot request returns generic success;
- [x] non-existent account returns the same public success semantics;
- [x] raw token is not persisted;
- [x] correct token resets password;
- [x] wrong token fails;
- [x] expired token fails;
- [x] consumed token fails;
- [x] replay after success fails;
- [x] concurrent double-submit produces one success only;
- [x] password reset invalidates all pre-reset sessions;
- [x] new login with new password succeeds;
- [x] old password fails after reset;
- [x] mail delivery failure remains enumeration-safe;
- [x] logs/telemetry do not contain raw token/password/reset URL;
- [x] OAuth-only account behavior matches the explicit policy;
- [x] throttling is temporary/bounded and does not permanently lock the account.

**Validation:** relevant unit/integration/API tests plus repository-standard `pnpm verify:commit` or equivalent project verification gate pass.

**Commit boundary:** `test(auth): cover account recovery security invariants`

## Risks and rollback

- **Existing dirty implementation conflicts with plan assumptions:** reconcile first; do not discard or overwrite unrelated work.
- **Session epoch mismatch:** if `sessionVersion` and `authVersion` overlap ambiguously, choose one authoritative browser-session revocation primitive and document migration compatibility before changing behavior.
- **Token table migration risk:** prefer using the existing verification-token schema when it satisfies invariants; avoid unnecessary new tables.
- **Email provider instability:** recovery request remains generic; failed delivery is observable internally without exposing account existence.
- **Concurrency race:** perform consume/update in one transaction or conditional mutation and assert affected-row count; do not rely on a read-then-write race-prone sequence.

Rollback is source/database migration rollback through the repository's reviewed migration process. Never restore validity of already consumed reset tokens during rollback.

## Final acceptance criteria

- [x] Reset tokens are cryptographically strong, hashed at rest, short-lived, and single-use.
- [x] Reset-token consumption and password/session-version update are atomic under concurrent requests.
- [x] Forgot-password response semantics do not disclose account existence.
- [x] Successful reset revokes every pre-reset session.
- [x] Invalid/expired/consumed/replayed tokens fail generically.
- [x] Raw tokens, passwords, and reset URLs are absent from logs, telemetry, audit metadata, and API errors.
- [x] Delivery failure does not alter public enumeration behavior.
- [x] OAuth-only account behavior is explicit and tested.
- [x] Recovery abuse controls are bounded and non-permanent.
- [x] Recovery-specific regression tests pass with repository verification gates.

## Handoff

After 049A is closed, continue to [Plan 049B](049b-email-verification-and-secure-email-change.md). Reuse the verified token/fresh-auth primitives rather than creating a second incompatible token system.
