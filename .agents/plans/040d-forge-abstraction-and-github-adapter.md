# Plan 040D — Forge Abstraction and GitHub Adapter

**Status:** IMPLEMENTED — LIVE GITHUB VERIFICATION / FINAL CLOSURE PENDING
**Parent:** [Plan 040](040-git-github-delivery-roadmap.md)
**Depends on:** Plan 040C source boundary implemented; final closure remains batched

## Goal

Introduce a small forge-neutral application contract for change requests and implement GitHub as the first adapter using a narrow `gh`/GitHub API boundary without exposing GitHub credentials to normal terminal execution.

## Core model

Use generic domain language where practical:

- repository/forge identity;
- change request (GitHub pull request / GitLab merge request);
- checks/status;
- review state;
- mergeability;
- remote branch.

GitHub-specific fields belong in the GitHub infrastructure adapter, not the core contract unless truly unavoidable.

## Initial operations

Read-first operations before mutation:

- forge/repository detection from validated origin;
- change-request get/list;
- checks/status summary;
- review/approval summary;
- mergeability/base/head metadata;
- bounded comments/review findings only if needed for remediation workflows.

Then narrowly scoped mutation:

- create change request;
- update title/body/base when policy permits;
- no arbitrary GraphQL/REST/`gh api` model-facing passthrough.

## GitHub adapter

The adapter may use `gh` or direct GitHub API based on which yields the safer maintainable implementation. If using `gh`:

- execute only allowlisted argument templates;
- repository target must be derived/validated, not accepted as arbitrary free-form target;
- credential files remain hidden from the model and ordinary terminal;
- prohibit `gh api` arbitrary endpoints as a generic escape hatch;
- bound stdout/stderr and sanitize all public errors;
- classify network read vs external mutation accurately.

## Acceptance scenarios

1. GitHub origin is detected correctly;
2. non-GitHub/unknown forge returns explicit unsupported result rather than pretending GitHub;
3. PR get/list/check/review reads are bounded;
4. create PR from validated pushed branch succeeds in authorized live fixture;
5. wrong repository/owner target cannot be substituted through arguments;
6. arbitrary `gh`/API command injection is impossible;
7. credentials/auth headers/private paths do not leak;
8. ordinary terminal remains unable to read `~/.config/gh`;
9. capability policy distinguishes read from external mutation.

## Extensibility constraint

Do not implement GitLab in 040D unless trivial and independently justified. Prove instead that adding another forge adapter would not require changing model-facing core semantics for ordinary change-request operations.

## Exit criteria

- forge-neutral contracts exist at the proper application/core boundary;
- GitHub adapter is safe and useful;
- live GitHub read/create proof succeeds where authorized;
- zero unresolved P0/P1 in independent review;
- 040D merged before 040E begins.
