# Plan 049F — HTTP/API and Database Security Hardening

**Status:** IMPLEMENTED / REPOSITORY + DISPOSABLE-DB VERIFIED / PRODUCTION CREDENTIAL ROTATION BLOCKED (2026-08-30)
**Parent:** Plan 049
**Depends on:** Plan 049A–049E

**Closure evidence:** Central headers, no-store policy, same-origin checks,
JSON mutation content-type enforcement, generic error handling, CSS-defined
landing animation timing, bounded existing model/relay execution, and additive
migrations are implemented and locally verified. Plan 060 added separate migration/runtime credential configuration, a reviewed
least-privilege grant contract, disposable PostgreSQL positive/negative proof,
and a pre-listen production runtime-role check. The deployed production
credential itself was not rotated or re-granted because that remains an
authorized deployment/database boundary change. See `docs/security.md`.

## Goal

Harden authenticated HTTP/API behavior and the production database boundary with stricter browser policy, bounded request execution, content-type enforcement, cache/error review, least-privilege database roles, transport security, and safe backup/restore secret handling.

## Scope

### In scope
- remove remaining `style-src-attr 'unsafe-inline'` where repository implementation permits;
- request timeout and concurrency limits;
- strict content-type checks on mutation APIs;
- authenticated-response cache policy review;
- generic API error leakage review;
- runtime DB role least privilege;
- schema ownership/migration-role separation;
- production DB TLS/connection policy;
- backup/restore access and secret-handling review.

### Out of scope
- unrelated performance tuning;
- adopting a new database engine;
- production credential rotation or deployment without explicit approval.

## Architecture decisions

1. Security middleware must fail closed for unsupported mutation content types where the route expects JSON/form data.
2. Request timeouts/concurrency caps must be placed at the narrowest reliable application/edge boundary and have bounded errors.
3. Authenticated/private responses must default to non-shared caching unless a route has an explicit safe cache design.
4. Generic public API errors must not expose stack traces, SQL/provider errors, filesystem paths, internal hostnames, or credentials.
5. Runtime application DB credentials must be non-superuser and must not own the whole database when a narrower schema/application role model is feasible.
6. Migration/schema-owner authority is separated from ordinary application DML authority.

## Phases and tasks

### PHASE-01 — Browser and API policy review
- [ ] Inventory CSP/header generation and identify the exact remaining dependency on `style-src-attr 'unsafe-inline'`.
- [ ] Refactor custom-property/inline styling to remove the directive when possible without functional regression.
- [ ] Inventory mutation APIs and enforce expected `Content-Type` before body parsing/business logic.
- [ ] Review authenticated/private response cache headers and normalize unsafe defaults.
- [ ] Run a representative API-error leakage corpus against validation, auth, database, provider, and not-found failures.

### PHASE-02 — Request resource bounds
- [ ] Identify current server/proxy timeout and concurrency boundaries.
- [ ] Add bounded request timeout/cancellation behavior for mutation and expensive authenticated endpoints.
- [ ] Add concurrency limits where resource exhaustion is plausible and application-owned.
- [ ] Ensure timeout/limit responses are generic and do not leave partial security-sensitive mutations committed outside intended transaction boundaries.

### PHASE-03 — Database least privilege
- [ ] Inspect deployment/database role model and current grants without exposing credentials.
- [ ] Define separate schema/migration owner and runtime application role where supported.
- [ ] Grant runtime role only required schema usage and DML/sequence privileges.
- [ ] Prove runtime role cannot create roles/databases, alter unrelated schemas, or perform superuser-only operations.
- [ ] Review production TLS/connection requirements and certificate verification policy appropriate to the deployment.

### PHASE-04 — Backup/restore security
- [ ] Document who/what can create/read backups and where credentials are sourced.
- [ ] Ensure backups do not embed application secrets outside database data itself and are protected at rest/in transit by deployment controls.
- [ ] Define restore procedure using dedicated privileged operational identity, not runtime app credentials.
- [ ] Add a safe restore verification/runbook that requires explicit approval before production use.

### PHASE-05 — Regression coverage
- [ ] CSP/header regression proves no unintended weakening.
- [ ] Mutation content-type matrix rejects unsupported media types.
- [ ] Cache tests prove private authenticated data is not shared-cacheable by default.
- [ ] Error-leak canaries remain absent from public responses.
- [ ] Least-privilege database acceptance runs under runtime role and proves forbidden privileged operations fail.

## Risks and rollback

- **CSP UI breakage:** remove unsafe-inline only after exact dependency refactor and browser regression coverage.
- **Timeout partial writes:** transaction-bound sensitive mutations and propagate cancellation deliberately.
- **DB grant regression:** test application under the intended runtime role before revoking old broad privileges; production grant changes require explicit approval.

## Final acceptance criteria

- [ ] Remaining `style-src-attr 'unsafe-inline'` is removed or a narrow documented blocker remains for a separately approved follow-up.
- [ ] Mutation APIs enforce expected content type.
- [ ] Relevant requests have bounded timeout/concurrency behavior.
- [ ] Authenticated response cache policy is explicitly reviewed and safe by default.
- [ ] Public API errors do not leak internal/provider/database details.
- [ ] Runtime DB role is non-superuser and least privileged.
- [ ] Migration/schema ownership is separated from runtime authority where deployment supports it.
- [ ] Production DB TLS/connection and backup/restore secret-handling policy is documented and verified without exposing credentials.

## Handoff

Continue to [Plan 049G](049g-adversarial-security-test-matrix-and-closure.md).
