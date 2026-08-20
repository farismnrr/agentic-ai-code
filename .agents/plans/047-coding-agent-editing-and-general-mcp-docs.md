# Plan 047 — Coding-agent edits and vendor-neutral MCP documentation

Status: CLOSED / VERIFIED (2026-08-20)

## Objective

Make native file mutation practical for coding-agent workflows without asking an
agent to rewrite an entire large file. Extend `file_edit` with bounded,
atomic multi-anchor edits while keeping `apply_patch` as the multi-file unified
diff primitive. Generalize the operator documentation around MCP deployment,
OAuth/OIDC resource-server configuration, and compatible client connections so
it does not depend on a named client, provider, tunnel vendor, or identity
vendor.

## Scope

- add a backwards-compatible batch form to `file_edit`;
- preflight every anchor against the original file and commit one atomic result;
- preserve workspace containment, protected-path, symlink, identity, size, and
  UTF-8 safeguards;
- publish a new immutable MCP catalog snapshot for the schema change;
- add deterministic acceptance for batch edits and documentation neutrality;
- rename/rewrite client and OAuth deployment docs around generic MCP/OAuth/OIDC
  contracts, including HTTPS edge and token configuration guidance.

## Verification

- focused Plan 047 acceptance;
- current MCP catalog contract and historic snapshot integrity;
- `pnpm verify:commit`.

Acceptance completed:

- `scripts/verify-047-editing-and-docs.sh` passed, including legacy and batch
  `file_edit`, all-or-nothing preflight, overlap rejection, catalog v10/v11
  immutability, and the vendor-neutral documentation scan;
- current MCP catalog v11 is 101 tools with SHA256
  `7a95d5d4344e50bd8be9266ebad95ea9a8d5bc6b2fb7934654ad053924c70ceb`;
- `pnpm verify:commit` passed, including lint/Clippy, Nuxt typecheck, and Rust
  warnings-denied checks.

## Closeout

Closeout is complete: canonical memory and affected guidance were synchronized.
The branch remains uncommitted because no commit was requested.
