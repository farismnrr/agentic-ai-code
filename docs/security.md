# Account and application security

This document records the security invariants implemented by the Plan 049
closure work. It is deliberately operational: bearer values and provider
credentials never belong in logs, audit metadata, database rows, or support
captures.

## Authentication and recovery

- Password-reset and email-verification links are random bearer values whose
  SHA-256 hashes are stored in `verification_tokens`. Consumption is an
  atomic, purpose-scoped, unexpired, single-use update.
- Password reset increments `users.auth_version` and revokes every active
  browser session for that user.
- Browser sessions contain an opaque secret in the sealed httpOnly cookie;
  only its hash is stored in `auth_sessions`. Middleware validates the session
  id, user id, secret hash, and revocation state on every authenticated request.
- Email changes remain pending until the new address confirms a short-lived
  link. The primary `users.email` value is not writable through ordinary
  settings updates.
- Sensitive account actions require recent authentication. TOTP enrollment,
  removal, recovery-code regeneration, and email change also require current
  password proof or an equivalent fresh session proof.

## MFA and audit

TOTP seeds are encrypted with the existing AES-256-GCM application secret.
Recovery codes are random, shown only in the enrollment/regeneration response,
and stored only as SHA-256 hashes. A guarded database update makes each code
single-use. Security events use bounded allowlisted metadata, owner-scoped
history reads, and a 180-day pruning policy.

The current application has no administrator mutation API or role-management
route. The `users.role` field defaults to `user`; middleware invalidates a
sealed session when the database role changes, so a future admin route must
still require both database-backed role authorization and fresh proof.

## HTTP policy

State-changing API requests enforce same-origin when a browser sends an
`Origin` header. Mutation requests with a content type must use JSON. API and
security-history responses are `no-store`; all responses receive the central
anti-sniffing, framing, referrer, permissions, and cross-origin headers.
The public landing page's custom animation timing is CSS-defined rather than
an inline style attribute. API errors use the repository's generic problem
handler and do not return raw provider, SQL, filesystem, or credential data.

## Database operating boundary

The additive account-security migrations are `0018_mean_lethal_legion.sql`
and `0019_even_stingray.sql`. They were generated, reviewed, and applied to
the configured database without destructive statements.

The current configured database connection was inspected read-only on
2026-08-26 and resolves to a PostgreSQL superuser. This is not an acceptable
production runtime boundary. Creating/rotating a dedicated non-superuser
runtime credential, granting only `CONNECT`, schema `USAGE`, table DML, and
required sequence privileges, separating migration ownership, and updating
the deployment secret requires an authorized operator/database change. It was
not performed during this closure pass because it changes external credentials
and can affect the running application. Plan 049F/G remain externally blocked
on that exact acceptance item until the authorized role migration is executed
and verified in a safe environment.

Backups and restores must use a separate privileged operational identity;
`NUXT_DATABASE_URL` and backup credentials are deployment secrets and must not
be placed in repository files, telemetry, or support output.

## Deterministic checks

Run the local security guards with:

```sh
pnpm verify:account-recovery
pnpm verify:account-security
pnpm verify:account-security-runtime
```

These checks are source/runtime contract guards, not a substitute for the
authorized database-role and deployed-browser acceptance described above.
