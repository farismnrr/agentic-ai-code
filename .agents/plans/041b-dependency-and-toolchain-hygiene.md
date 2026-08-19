# Plan 041B — Dependency and Toolchain Hygiene

**Status:** PLANNED
**Parent:** [Plan 041](041-code-intelligence-and-platform-polish-roadmap.md)
**Depends on:** 041A CLOSED / VERIFIED

## Goal

Reduce actionable dependency/toolchain debt without destabilizing the platform or forcing unsafe transitive overrides.

## Baseline to re-verify

Current audit observed a deprecated `glob@10.5.0` transitively through Nuxt/Nitro → archiver while `glob@13` also exists elsewhere. Treat this as evidence to inspect owner-level upgrade paths, not as permission to pin/override a transitive package blindly.

## Scope

- run current JS and Rust dependency audits using repository-authoritative tooling;
- map deprecated/vulnerable packages to direct owners;
- upgrade direct/root dependencies only when supported and regression-safe;
- review pnpm/corepack, Nuxt/Nitro, language-server and Rust toolchain pins;
- remove obsolete compatibility packages/configuration only when proven unused;
- keep lockfiles deterministic and intentional.

## Rules

- no blanket `pnpm overrides` merely to silence a warning unless compatibility and ownership are proven;
- no major framework upgrade bundled with unrelated cleanup;
- one logical dependency family per change where practical;
- compare behavior/build artifacts before and after meaningful framework/toolchain upgrades;
- preserve release reproducibility and canonical lockfile behavior.

## Verification

For any dependency change run focused package tests plus `pnpm verify:commit`, `pnpm build`, Rust tests/builds where applicable, dependency audits, and relevant Plan-039/040 regression gates.

## Exit criteria

- actionable owner-level dependency debt is reduced or explicitly documented as upstream-only;
- no forced unsafe transitive substitution is introduced;
- build/release/runtime behavior remains green;
- independent review finds zero unresolved P0/P1;
- 041B is merged before 041C begins.
