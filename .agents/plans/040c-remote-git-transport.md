# Plan 040C — Remote Git Transport

**Status:** IMPLEMENTED — LIVE GITHUB VERIFICATION / FINAL CLOSURE PENDING
**Parent:** [Plan 040](040-git-github-delivery-roadmap.md)
**Depends on:** Plan 040B source boundary implemented; final closure remains batched

## Goal

Provide narrow, policy-aware remote Git transport for fetch/push/ref synchronization without exposing Git credentials or general network access to ordinary `terminal_exec`.

## Scope

- inspect configured remotes/origin safely;
- validate remote URL/forge/repository identity;
- fetch selected refs with bounded result;
- push a specific local branch/ref to a validated remote;
- optional upstream tracking update where explicit;
- remote branch existence/parity checks;
- safe remote branch deletion as an explicit destructive operation;
- no generic arbitrary refspec execution.

## Credential boundary

Remote credentials remain owned by a narrow transport bridge/provider, not the normal Bubblewrap terminal. The model must never receive credential values, auth headers, helper output, or protected credential paths.

Prefer existing Git credential/provider mechanisms only when they can be invoked through a controlled execution path with:

- allowlisted Git subcommands/options;
- known repository and remote;
- sanitized bounded output;
- capability/effect policy before mutation;
- no shell interpolation;
- no arbitrary credential-helper configuration supplied by the repository.

## Security requirements

- validate remote repository matches active workspace intent;
- deny cross-repository push by default;
- force push / force-with-lease remain denied unless a later explicit narrow policy is designed and reviewed;
- reject arbitrary refspecs, remote helper protocols and config-injected commands;
- preserve host-key/TLS verification;
- classify fetch as network read and push/delete as external mutation/destructive effect as appropriate;
- independently verify remote result after mutation;
- never infer success only from process exit text.

## Acceptance scenarios

Use disposable remote fixtures where possible and a live GitHub test path only when authorized:

1. remote discovery is bounded and secret-safe;
2. fetch updates expected remote-tracking ref;
3. push a feature branch succeeds and remote existence is observed;
4. wrong-repo/wrong-origin target is denied;
5. malformed ref/refspec rejected before mutation;
6. force-push attempts denied;
7. credential values never appear in result/UI/telemetry;
8. remote delete requires explicit policy and is independently confirmed;
9. terminal network can remain default-isolated while native remote Git succeeds.

## Exit criteria

- fetch/push/ref synchronization works through a narrow remote transport boundary;
- normal terminal remains credential/network-isolated by default;
- policy/effect integration and live remote proof pass;
- independent review returns zero unresolved P0/P1;
- 040C is CLOSED / VERIFIED / merged before 040D begins.
