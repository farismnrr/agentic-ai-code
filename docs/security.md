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

The configured production database connection inspected on 2026-08-26 was a
PostgreSQL superuser and remains an external deployment blocker until an
operator rotates it. The repository now enforces the intended boundary:

- `NUXT_DATABASE_URL` is the application runtime identity and production
  startup can reject `SUPERUSER`, `CREATEROLE`, `CREATEDB`, `REPLICATION`, or
  `BYPASSRLS` capabilities;
- `NUXT_DATABASE_MIGRATION_URL` is the separate Drizzle/migration identity;
  production migration tooling does not fall back to the runtime credential;
- `ops/database/least-privilege.sql` grants only `CONNECT`, schema `USAGE`,
  table `SELECT/INSERT/UPDATE/DELETE`, and required sequence access to the
  runtime role, including migration-owner default privileges;
- Docker Compose has no built-in `postgres:postgres` fallback and requires an
  explicit runtime database URL.

A disposable PostgreSQL 17 acceptance on 2026-08-30 proved the grant contract:
the runtime role could insert/select application data, could not create a table,
and had all five forbidden cluster-level privilege flags disabled. This proves
the repository contract, not the production credential rotation. Production
Plan 049 closure still requires the authorized operator role/secret migration.

Backups and restores must use a separate privileged operational identity;
`NUXT_DATABASE_URL`, `NUXT_DATABASE_MIGRATION_URL`, and backup credentials are
deployment secrets and must not be placed in repository files, telemetry, or
support output.

## Terminal filesystem and credential boundary

Broad terminal authority means operator-user work within authorized roots.
`--dir "$HOME" --execution-root "$HOME"` permits ordinary files throughout
that home; a project grant stays project-scoped. Bubblewrap remains mandatory,
with read-only system runtime (`/usr`, `/lib`, `/etc`, `/bin`, `/sbin`) and isolated
`/dev`, `/proc`, `/tmp` (tmpfs) and PID namespace. System configuration files such
as `/etc/resolv.conf` or TLS CA bundles are readable by runtime toolchains, but writes
to `/etc` fail read-only (`Read-only file system`). The child has no effective
capabilities (`CapEff=0000000000000000`) and inherits Linux `PR_SET_NO_NEW_PRIVS`,
preventing setuid/file-capability elevation even for renamed or copied helpers.
`sudo`, `su`, `doas`, `pkexec`, `runas` and generic SSH clients are denied directly
and masked at every visible safe-PATH spelling, including separately mounted `/bin`
and `/usr/bin` aliases.

The canonical `core::protected_paths` policy masks SSH/GPG/cloud/Git/package
credentials, all `.env` / `.env.*` except `.env.example`, and known browser,
keyring, password-manager and CLI authentication stores. Relay state uses its
canonical configured location and is hidden even when placed inside HOME.
Protected stores are empty private mounts; file masks prevent host reads and
writes. Masking applies recursively to nested credential directories (e.g.
`deep/.ssh`, `deep/.aws`, `deep/.cargo/credentials`, `deep/.env.*`) and nested
Unix domain sockets across visible trees. Non-secret templates such as `.env.example`
remain explicitly readable. The child environment is cleared and rebuilt with
runtime-only values (`LANG`, `PATH`, `TMPDIR`, `HOME`); host secrets, SSH/GPG/keyring,
and session-bus variables are never forwarded. Existing output and job-retention
redaction remains mandatory.

Every visible user tree is scanned before spawn, without following directory
symlinks. Protected symlinks, traversal/metadata failures and the 500,000-entry
limit abort execution. Dependency/build/cache directories cannot be skipped
while still visible: they can contain credentials too. Filesystem Unix sockets
found in the tree are masked. This is a path-based credential policy, not a
content classifier for secrets copied into arbitrary ordinary files. Operators
must keep credential material in protected stores, and must not concurrently
replace the filesystem being prepared for sandbox execution.

Host `/run/user`, host `/tmp`, host processes, session/system D-Bus and journal
mounts remain absent. HOME scope never authorizes host-service control.
Docker/Tailscale sockets remain independent explicit operator opt-ins; Docker
commonly provides root-equivalent host authority. Network namespace sharing is
also an independent operator grant (`RELAY_ALLOW_TERMINAL_NETWORK=true`); by
default, Bubblewrap unshares the network namespace (`--unshare-net`), blocking all
outbound TCP/UDP connects and raw sockets at the kernel level. Dedicated HTTP and
search tools (`http_fetch`, `web_search`) enforce their own SSRF, private-network,
and allowlist policies independent of the terminal network flag.

Generic SSH remains separate from `ssh_readonly_exec`. Dedicated SSH restores
only its reviewed identity and known-host files read-only after masking the
credential store; it gets no writable workspace or optional local sockets.

MCP-first selection is advice derived from the final active tool keys in
primary and child prompts. It cannot grant missing tools, bypass read-only or
manual approvals, or replace the relay security checks. Covered file, remote
Git, HTTP/forge/SSH/messaging operations should use their active dedicated
capabilities; local Git, interpreters, package managers, and language-server work
remain legitimate terminal fallbacks. No blanket Git executable ban is introduced,
and removing dedicated MCP wrappers does not ban CLI developer tools.
Discovery and invocation are consistent: modern wire clients observe 52 tools in
Full and 15 in Primary; invoking tools disabled by capability returns structured
revocation errors (`CAPABILITY_REVOKED`), and unknown tools return 404 errors.

## Deterministic checks

Account recovery/session/MFA security coverage is part of the normal web test suite:

```sh
pnpm test:web
```

The relevant feature tests live under `test/unit/` and are selected by behavior rather than plan-specific verification commands. These tests are not a substitute for the authorized database-role and deployed-browser acceptance described above.
