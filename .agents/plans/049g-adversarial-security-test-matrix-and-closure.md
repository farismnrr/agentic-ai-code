# Plan 049G — Adversarial Security Test Matrix and Closure

**Status:** LOCALLY + DISPOSABLE-DB VERIFIED / PRODUCTION CREDENTIAL ROTATION BLOCKED (2026-08-30)
**Parent:** Plan 049
**Depends on:** Plan 049A–049F

**Closure evidence:** `pnpm verify:account-recovery`,
`pnpm verify:account-security`, and `pnpm verify:account-security-runtime`
cover token/session/MFA/recovery replay primitives, secret-safe source
contracts, CSRF/content/cache/header policy, and TOTP/recovery runtime
vectors. Plan 060 supplements the source-contract checks with behavioral HTTP/security
and application authorization tests plus disposable PostgreSQL role acceptance.
A safe runtime role starts successfully, while a superuser runtime exits before
Nitro listens. The remaining acceptance item is production credential rotation
and deployed verification; no production DB privilege mutation, destructive
penetration test, or GitHub mutation was performed.

## Goal

Convert the account/application security requirements into deterministic adversarial regression coverage and close the Plan 049 family only after composed authorization, browser, session, recovery, abuse, API, and database boundaries have been independently reviewed.

## Scope

### In scope
- IDOR suite;
- CSRF matrix;
- XSS payload corpus;
- authorization matrix;
- session replay/revocation suite;
- account-recovery expiry/replay/concurrency suite;
- email-change and MFA replay/cross-account cases;
- abuse-control and audit-redaction tests;
- header/CSP regression integration;
- final security architecture review and documentation reconciliation.

### Out of scope
- destructive production penetration tests without explicit approval;
- unbounded fuzzing against external services;
- claiming third-party certification/compliance not actually performed.

## Test principles

1. Prefer deterministic local/unit/integration/API tests for security invariants so they are cheap enough for routine verification.
2. Every object-level authorization test must include a second-user/second-object negative case.
3. Every bearer or one-time credential test must include wrong, expired, consumed, replayed, and concurrent-use cases where relevant.
4. Canary secrets must be injected into controlled fixtures and asserted absent from logs, telemetry, audit metadata, and public errors.
5. Browser-security tests cover both policy headers and representative application behavior; header presence alone is insufficient.
6. Production/external mutation is never required to prove local security invariants unless a separately approved environment-specific acceptance step is explicitly necessary.

## Security matrix

| Domain | Required adversarial cases |
| --- | --- |
| IDOR | cross-user session, audit-history, MFA/factor, workspace/resource identifiers; owner derived from auth principal |
| CSRF | state-changing cookie-auth routes reject cross-site requests according to the chosen framework/token/origin strategy; safe methods remain non-mutating |
| XSS | stored/reflected payload corpus across profile, workspace, chat/rendered content, error surfaces, and any admin/security-history UI; CSP remains defense-in-depth |
| Authorization | anonymous/user/admin × read/write/destructive operations, including role downgrade and stale privileged session |
| Sessions | replay after password reset, email change, per-session revoke, logout-all-other, role change, MFA/security changes |
| Recovery | unknown/expired/consumed/replayed token, concurrent double consume, enumeration parity, delivery failure, OAuth-only behavior |
| Email identity | unverified access policy, resend abuse, duplicate address race, stale fresh-auth proof, confirmation replay |
| MFA/passkey | challenge replay, cross-account credential substitution, factor removal without fresh auth, recovery-code replay/regeneration |
| Abuse protection | throttling expiry, false-positive recovery, no permanent lockout, privacy-preserving breached-password lookup |
| Audit | IDOR, retention, bounded metadata, canary secret redaction |
| HTTP/API | unsupported content type, private-cache policy, timeout/concurrency, generic error leakage |
| Database | runtime-role allowed operations succeed; privileged/unrelated schema operations fail |

## Phases and tasks

### PHASE-01 — Build the cheap local security guard
- [ ] Inventory existing repository verification scripts and header/CSP checks.
- [ ] Add security suites to the cheapest appropriate local guard rather than creating a second competing verification framework.
- [ ] Keep fixtures deterministic and independent of production credentials or network access wherever possible.
- [ ] Document any environment-bound acceptance that cannot be made deterministic locally.

### PHASE-02 — IDOR and authorization matrix
- [ ] Create two-user and admin fixtures.
- [ ] Exercise every security-sensitive object endpoint with owner and non-owner identifiers.
- [ ] Exercise role transitions and stale authorization claims.
- [ ] Assert safe denial semantics without object-existence leakage where appropriate.

### PHASE-03 — CSRF and XSS corpus
- [ ] Enumerate cookie-authenticated state-changing routes.
- [ ] Test the repository's selected CSRF/origin/same-site protections across allowed and disallowed origins/methods/content types.
- [ ] Build a bounded representative XSS corpus for stored and reflected sinks.
- [ ] Verify server/client escaping/sanitization plus CSP regression.

### PHASE-04 — Authentication/session abuse suites
- [ ] Run full recovery-token matrix from 049A.
- [ ] Run email verification/change matrix from 049B.
- [ ] Run session revocation/replay matrix from 049C.
- [ ] Run admin/MFA/recovery-code replay matrix from 049D.
- [ ] Run stuffing/throttling/breached-password/audit-redaction matrix from 049E.

### PHASE-05 — HTTP/database and integrated closure
- [ ] Run header/CSP/content-type/cache/error/time-limit tests from 049F.
- [ ] Run least-privilege DB role acceptance in an authorized non-production/test environment where role behavior can be proven safely.
- [ ] Perform a composed architecture/security review for bypasses created between child plans.
- [ ] Reconcile plan checklists, security docs/runbooks, migrations, and operator/deployment notes with actual behavior.
- [ ] Run repository-standard final verification gates.

## Review gates

Plan 049 must not close with any unresolved:
- P0 critical account-takeover/data-exposure issue;
- P1 high-impact authorization/session/recovery/MFA bypass;
- known plaintext credential/token leakage;
- attacker-triggerable permanent account lockout;
- runtime database superuser requirement without an explicitly reviewed blocker and follow-up plan.

P2/P3 findings may be deferred only when documented with bounded impact, owner, and follow-up plan; deferral must not contradict a Plan 049 acceptance criterion.

## Risks and rollback

- **Flaky security tests:** use deterministic clocks/fixtures and isolate external-provider acceptance from local guards.
- **False confidence from header-only checks:** pair policy tests with actual endpoint/render behavior.
- **Test corpus secret leakage:** use synthetic canaries only, never real credentials.
- **Unsafe DB testing:** privileged negative tests run only against an authorized non-production/test database or through introspection that cannot mutate production.

## Final acceptance criteria

- [ ] IDOR suite covers every Plan 049 security-sensitive object boundary with cross-user negative cases.
- [ ] CSRF matrix covers all cookie-authenticated mutation routes.
- [ ] XSS corpus covers representative stored/reflected sinks and CSP/header regression.
- [ ] Authorization matrix covers anonymous/user/admin and stale-role/session cases.
- [ ] Recovery/email/MFA one-time credential replay/concurrency suites pass.
- [ ] Session replay/revocation suite proves each planned revocation event.
- [ ] Abuse-control tests prove bounded temporary behavior and absence of permanent lockout.
- [ ] Canary secret-redaction tests cover logs, telemetry, audit records, and API errors.
- [ ] HTTP/API hardening regression passes.
- [ ] Least-privilege database acceptance passes in an authorized safe environment.
- [ ] Final composed security review has zero unresolved P0/P1 findings.
- [ ] Repository-standard verification gates pass and Plan 049 documentation matches the implemented state.

## Closure handoff

After 049G passes, update the parent Plan 049 status and master todo truthfully. Any deferred P2/P3 work becomes a separately numbered follow-up plan; do not silently expand or leave ambiguous unfinished security work inside a closed Plan 049.
