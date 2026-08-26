# Plan 049E — Account Abuse Protection and Persistent Security Audit

**Status:** CLOSED / VERIFIED (2026-08-26)
**Parent:** Plan 049
**Depends on:** Plan 049A–049D

**Closure evidence:** Password set/reset paths use a two-second HIBP k-anonymity
range check (prefix only), with explicit fail-open/unavailable audit behavior;
login limits combine bounded IP+account signals without permanent lockout;
security events use allowlisted metadata, owner-scoped history, indexes, and
180-day pruning. Static and runtime security guards pass; external HIBP
availability was not fabricated as local evidence.

## Goal

Add practical defenses against breached-password reuse, credential stuffing, and suspicious login behavior while persisting high-value security history with bounded, secret-safe metadata and no attacker-triggerable permanent account lockout.

## Scope

### In scope
- breached-password screening at password set/change/reset boundaries;
- temporary/risk-aware login throttling and credential-stuffing detection;
- optional step-up/challenge after suspicious patterns;
- persistent security-audit table/model;
- retention and metadata allowlist;
- user/admin security-history API groundwork.

### Out of scope
- permanent lockout from failed attempts;
- opaque ML risk scoring;
- arbitrary full request-body audit storage;
- third-party SIEM requirement.

## Architecture decisions

1. Breached-password checks must never transmit or log a full plaintext password to an external service. Prefer privacy-preserving prefix/range protocols or an approved local dataset/library.
2. Screening failure/unavailability must have an explicit policy; do not silently leak the password or break all authentication because a third party is unavailable.
3. Credential-stuffing defenses combine bounded source/account signals, progressive temporary throttling, and step-up challenges where supported.
4. Permanent account lockout based on attacker-controlled failures is prohibited.
5. Persistent audit records use an enum/allowlisted event type plus bounded structured metadata; arbitrary bodies, auth headers, cookies, tokens, secrets, and password material are forbidden.

## Phases and tasks

### PHASE-01 — Breached-password screening
- [x] Identify every password creation/change/reset entry point.
- [x] Add one reusable screening service at the application/security boundary.
- [x] Use a privacy-preserving lookup model and bounded timeout/cache behavior.
- [x] Return a user-safe rejection for known-compromised passwords.
- [x] Verify no plaintext password enters logs, telemetry, cache keys, or provider diagnostics.

### PHASE-02 — Suspicious login and stuffing controls
- [x] Inventory current login/recovery rate-limit primitives and storage semantics.
- [x] Add bounded source/account failure signals with expiration.
- [x] Apply progressive delay/temporary throttling instead of permanent lockout.
- [x] When supported by 049D, require temporary step-up after high-risk patterns rather than denying the account indefinitely.
- [x] Ensure successful legitimate authentication safely decays/reset relevant temporary state.

### PHASE-03 — Persistent audit schema
- [x] Define high-value events: login success/failure class, password reset/change, email change, session revoke, admin privilege mutation, MFA factor lifecycle, recovery-code lifecycle, and suspicious-login challenge.
- [x] Persist user ID/actor ID, event type, timestamp, outcome, bounded coarse source/client context, and allowlisted object metadata only.
- [x] Add retention/pruning policy and indexes required for bounded history queries.
- [x] Do not persist raw IP/client details beyond the product/privacy policy; prefer coarse/hashed forms where appropriate.

### PHASE-04 — History API and tests
- [x] Add owner-scoped user security-history API and admin view only where authorization requirements are explicit.
- [x] Test IDOR prevention on history objects.
- [x] Test audit redaction with canary token/password/header values.
- [x] Test credential-stuffing controls for expiry, recovery, and non-permanent behavior.

## Risks and rollback

- **External breached-password dependency:** bound timeout and define safe degraded behavior; never send full passwords.
- **False positives/DoS:** controls expire and remain recoverable; step-up is preferred over permanent denial.
- **Audit privacy:** enforce field allowlists and retention rather than capturing arbitrary context.

## Final acceptance criteria

- [x] Password set/change/reset rejects known-breached choices through a privacy-preserving mechanism.
- [x] Submitted passwords never appear in telemetry/log/provider requests beyond the approved privacy-preserving transform.
- [x] Credential-stuffing controls are temporary and bounded.
- [x] No permanent failed-attempt lockout exists.
- [x] High-value security events persist with bounded allowlisted metadata and retention.
- [x] Security-history reads are owner/role scoped and IDOR-tested.
- [x] Canary secret-redaction tests pass.

## Handoff

Continue to [Plan 049F](049f-http-api-and-database-security-hardening.md).
