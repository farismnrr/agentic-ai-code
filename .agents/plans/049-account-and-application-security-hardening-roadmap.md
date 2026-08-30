# Plan 049 — Account and Application Security Hardening Roadmap

**Status:** CLOSED — ENGINEERING SECURITY CONTRACT VERIFIED; PRODUCTION DB ROLE ROTATION REMAINS AN OPERATOR DEPLOYMENT PREREQUISITE (2026-08-30)
**Created:** 2026-08-22
**Plan family:** 049A–049G
**Primary next plan:** Plan 049A — Account Recovery Foundation

**Closure boundary:** The application/security implementation, additive
migrations, deterministic guards, and local review are complete for 049A–049E
and the HTTP portion of 049F/G. The repository now defines and proves a least-privilege runtime/migration role
contract against disposable PostgreSQL, and production startup can fail closed
before Nitro listens when an unsafe runtime role is configured. The currently
deployed production credential was not rotated or re-granted in Plan 060; that
external operator/database action remains the only DB-role closure blocker. See
`docs/security.md` and the 049F/G evidence below.

## Goal

Raise the application's account, authentication, session, administration, abuse-resistance, auditability, HTTP/API, database, and adversarial-test security posture toward an industry-standard baseline without weakening the existing authentication boundary or introducing high-risk account lockout behavior.

## Success criteria

Plan 049 is complete only when:

1. users can recover forgotten passwords through short-lived, hashed, single-use reset tokens with anti-enumeration responses and global session revocation after reset;
2. email ownership is verified, email changes require fresh authentication and confirmation of the new address, and identity/session state is safely refreshed after changes;
3. users can inspect and revoke active sessions/devices without exposing session tokens;
4. sensitive administrator actions require step-up/fresh authentication, are durably audited, and administrators have a stronger authentication policy with MFA/passkey support;
5. password/account abuse defenses cover breached-password checks and credential-stuffing/suspicious-login signals without permanent lockout primitives that can be weaponized for denial of service;
6. high-value security events are persisted with bounded metadata and retention rules;
7. HTTP/API and database production boundaries follow least privilege, strict input/content/cache policies, bounded resource usage, and safe error handling;
8. deterministic security tests cover IDOR, CSRF, XSS, authorization, session replay/revocation, recovery-token misuse, and relevant abuse paths;
9. no sensitive token, password, credential, reset secret, session secret, or provider secret is persisted or emitted in plaintext telemetry/logs.

## Verified current state

Verified in the working tree on 2026-08-22:

- Authentication endpoints exist under `server/api/auth/`, including login, register, logout, verify, forgot-password, and reset-password routes.
- `server/database/schema.ts` defines `users.sessionVersion`, `users.emailVerifiedAt`, `users.authVersion`, and a `verification_tokens` table with hashed token storage, token type, expiry, and `consumedAt`.
- `server/application/auth.ts` already exposes verification-token and reset/verification use cases.
- The current branch is `fix/account-recovery` and contains uncommitted authentication changes. Those changes are pre-existing implementation work and are not modified by this planning task.
- The current password-recovery worktree includes anti-enumeration behavior, IP-based rate limiting, generated reset tokens, hashed-token persistence, a 30-minute expiry, session clearing, and structured security telemetry. These are implementation candidates, not accepted completion evidence until reviewed and tested against Plan 049A.
- No matching `*auth*.test.*` test file was found by the bounded repository search; deterministic security acceptance must therefore be made explicit rather than assumed.

## Scope

### In scope

- account recovery;
- email verification and email-change security;
- user-visible session/device management;
- administrator fresh-auth and MFA/passkey foundation;
- breached-password and credential-stuffing/suspicious-login defenses;
- persistent high-value security audit trail;
- HTTP/API hardening;
- database least privilege and production connection/backup security;
- deterministic adversarial security test coverage.

### Out of scope

- passwordless-only migration for all users;
- permanent account lockout after failed authentication attempts;
- arbitrary risk-scoring/ML infrastructure;
- enterprise SIEM integration;
- full organization/tenant RBAC redesign unless required by a verified authorization defect;
- production deployment or irreversible infrastructure changes without explicit approval;
- storing raw reset, verification, recovery-code, session, or MFA secrets.

## Architecture and security decisions

### AD-001 — Token material is one-way at rest

Password-reset tokens, email-verification tokens, recovery codes, and comparable bearer secrets must be generated with cryptographically secure randomness and stored only as cryptographic hashes. Raw values may exist only transiently for delivery to the intended user.

### AD-002 — Recovery is single-use and session-invalidating

Successful password reset must consume the reset token atomically, update the password, and advance the server-side credential/session epoch so every previously issued session becomes invalid. The current browser session is cleared explicitly as defense in depth.

### AD-003 — Anti-enumeration responses are mandatory

Forgot-password and similar account-discovery endpoints must return materially indistinguishable public responses regardless of whether the account exists. Logs/telemetry may record bounded internal state but must not expose account existence to the caller.

### AD-004 — No permanent authentication lockout

Credential-stuffing and suspicious-login controls use bounded throttling, progressive delay, temporary challenge/step-up, or equivalent recoverable controls. Do not introduce permanent lockout based solely on attacker-controlled failed attempts.

### AD-005 — Fresh authentication for high-risk mutations

Changing primary email, sensitive administrator mutations, MFA enrollment/removal, recovery-code regeneration, and comparable actions require a recent authenticated proof. Session age alone is not sufficient if the session can be replayed without user re-proof.

### AD-006 — Audit high-value events, not secrets

Persist security events needed for investigation and user/admin history using allowlisted, bounded metadata. Never persist plaintext passwords, reset/verification tokens, session tokens, MFA seeds, recovery codes, provider credentials, or arbitrary request bodies.

### AD-007 — Session management exposes identity, never bearer material

Session/device APIs return safe metadata such as session identifier, current-session marker, creation/last-seen timestamps, coarse client/device context where available, and revocation state. They never return cookie values or bearer tokens.

### AD-008 — Passkeys/WebAuthn are preferred for long-term strong authentication

The MFA foundation may begin with TOTP if repository/product constraints make it materially simpler, but WebAuthn/passkeys are the preferred phishing-resistant direction. Recovery codes remain hashed and single-use.

### AD-009 — Least privilege extends through the database

Production application connections must use a non-superuser role with only required grants. Schema ownership, migration authority, application DML authority, transport encryption, backup access, and secret handling must be explicitly separated where the deployment permits it.

## Child plans

| Plan | Capability | Depends on | Status | Exit criterion |
| --- | --- | --- | --- | --- |
| 049A | Account Recovery Foundation | existing password authentication/session epoch | CLOSED / VERIFIED | secure forgot/reset flow is atomic, anti-enumerating, single-use, short-lived, session-invalidating, telemetry-safe, and adversarially tested |
| 049B | Email Verification and Secure Email Change | 049A token/step-up primitives where reusable | CLOSED / VERIFIED | registration verification and fresh-auth email change safely prove ownership and refresh identity/session state |
| 049C | Session and Device Management | 049A session revocation semantics | CLOSED / VERIFIED | users can list safe active-session metadata, revoke one session, and log out all other sessions without token exposure |
| 049D | Admin Fresh-Auth and MFA/Passkey Foundation | 049B, 049C | CLOSED / VERIFIED | sensitive admin actions require step-up auth, audit records exist, and stronger admin authentication/recovery is usable and tested |
| 049E | Account Abuse Protection and Persistent Security Audit | 049A–049D event/auth primitives | CLOSED / VERIFIED | breached-password checks, credential-stuffing/suspicious-login controls, and durable bounded high-value audit history are integrated without permanent lockout |
| 049F | HTTP/API and Database Security Hardening | 049A–049E interfaces stabilized | CLOSED / VERIFIED | mutation APIs, headers/CSP, resource limits, caches/errors, DB roles/TLS/grants/backups satisfy reviewed least-privilege policy |
| 049G | Adversarial Security Test Matrix and Closure | 049A–049F | CLOSED / VERIFIED | automated IDOR/CSRF/XSS/authz/session/recovery/abuse suites pass and final security review has no unresolved P0/P1 finding |

## Master todo

- [x] 049A — account recovery foundation
- [x] 049B — email verification and secure email change
- [x] 049C — session/device management API and UI
- [x] 049D — admin fresh-auth plus MFA/passkey foundation
- [x] 049E — account abuse protection and persistent security audit trail
- [x] 049F — HTTP/API and database hardening
- [x] 049G — adversarial security test matrix and integrated closure
- [x] security-sensitive migrations reviewed for rollback and data compatibility
- [x] no secret-bearing field appears in logs, telemetry, API error bodies, or audit metadata
- [x] composed security/architecture review has zero unresolved P0/P1 findings
- [x] repository-required lint/type/build/test/verification gates pass
- [x] documentation and security runbooks reflect the final behavior

## Execution order

Primary dependency path:

```text
049A Account Recovery
  ↓
049B Email Verification / Change
  ↓
049C Session Management
  ↓
049D Admin Fresh-Auth + MFA / Passkey
  ↓
049E Abuse Protection + Audit Persistence
  ↓
049F HTTP/API + Database Hardening
  ↓
049G Security Test Closure
```

Read-only review, threat-model review, and test-case design may run in parallel. Implementation that changes shared auth/session/schema boundaries must remain serialized or use isolated worktrees with explicit reconciliation.

## Cross-plan risks

- **Account takeover through recovery:** weak reset token lifecycle or non-atomic consumption can bypass otherwise strong password controls. Mitigation: cryptographic tokens, hashed storage, short TTL, atomic consume/update, single-use enforcement, global session invalidation, and replay tests.
- **User enumeration:** distinct status, body, timing, or rate-limit behavior can reveal account existence. Mitigation: generic public responses and deterministic timing/response review.
- **Email-change takeover:** changing email from a stale/replayed session can redirect recovery ownership. Mitigation: fresh auth, new-address confirmation, old-address notification, and session/identity refresh.
- **Session-revocation gaps:** sealed/stateless sessions can survive unless a server-side epoch or revocation registry is checked on every authenticated request. Mitigation: make the revocation authority explicit and test stale-session rejection.
- **MFA recovery becoming the weakest link:** recovery codes or disable-MFA flows can negate MFA. Mitigation: hashed single-use codes, fresh auth, audited changes, and strong recovery policy.
- **Abuse controls causing DoS:** attacker-generated failures can lock out victims. Mitigation: temporary/risk-aware controls only; no permanent failed-attempt lockout.
- **Audit trail becoming sensitive data:** excessive metadata can create a new credential/privacy leak. Mitigation: allowlisted event schema, bounded fields, retention limits, and secret-redaction tests.
- **Database privilege drift:** migrations may accidentally require or normalize superuser access. Mitigation: separate migration role from runtime role and prove runtime behavior under least privilege.

## Final acceptance criteria

- [ ] All child plans 049A–049G meet their own exit criteria.
- [ ] Password reset is anti-enumerating, hashed at rest, short-lived, single-use, atomic, and revokes all prior sessions.
- [ ] Registration/email ownership verification and email-change confirmation cannot be bypassed through stale session state.
- [ ] Session management can revoke one or all-other sessions and stale sessions are rejected on the next protected request.
- [ ] Sensitive admin operations require fresh authentication and emit persistent audit records.
- [ ] MFA/passkey enrollment, authentication, recovery, and removal paths have explicit step-up and replay protections.
- [ ] Breached-password and suspicious-login protections fail safely if external screening is unavailable and never log submitted passwords.
- [ ] Permanent attacker-triggerable account lockout does not exist.
- [ ] Persistent audit records contain bounded allowlisted metadata and documented retention.
- [ ] Authenticated responses have reviewed cache policy; mutation APIs enforce expected content type; generic errors do not leak internals.
- [ ] Production DB runtime role is non-superuser and least-privileged; transport/backup/restore secret handling is documented and verified in the deployable environment.
- [ ] IDOR, CSRF, XSS, authorization matrix, session replay/revocation, recovery replay/expiry, and representative abuse tests are automated.
- [ ] Final security review reports zero unresolved P0/P1 defects.

## Handoff

Execution begins with [Plan 049A](049a-account-recovery-foundation.md). Because the current `fix/account-recovery` worktree already contains uncommitted authentication changes, the implementation owner must first review and reconcile those changes against 049A rather than blindly recreating or overwriting them.

## Engineering closure decision — 2026-08-30

Plan 060 completed the repository-owned least-privilege contract, disposable PostgreSQL grant proof, pre-listen privileged-role rejection, behavioral HTTP authorization/security tests, container hardening, and fresh dependency audits. Production credential rotation is intentionally an operator deployment prerequisite because it requires protected secret access and a production database mutation; it is no longer represented as unfinished application engineering. Do not interpret this closure as evidence that the currently running legacy Nuxt container has already rotated its credential.
