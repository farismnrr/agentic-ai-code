# Plan 066 — Sensio-Style Engineering Guard Lifecycle

**Status:** CLOSED / VERIFIED
**Goal:** Adopt Sensio's fast-development and closure-validation policy in ai-code without importing Sensio's monorepo-specific structure: permanent isolated unit tests are forbidden, unrelated legacy unit tests are removed after classification, and guard execution distinguishes fast, full, and release lifecycle stages.

## Verified current state

- ai-code has one Nuxt/TypeScript application and one Rust native-tool package, while Sensio has a registered multi-codebase monorepo. ai-code must use its existing Nuxt/Rust scope model rather than copy Sensio's codebase registry.
- Sensio's current `AGENTS.md` forbids new permanent isolated unit tests, permits temporary disposable unit tests, and permits permanent integration, E2E, contract, smoke, and regression tests.
- Sensio's codebase policy checks only newly-added diff paths, preserving historical tests while blocking new `test/unit/**`, `*.unit.test.*`, `*.unit.spec.*`, source-co-located test files, and newly-added Rust inline `#[cfg(test)]` modules.
- Sensio's fast mode runs policy plus lint/typecheck only. Full mode performs boundary-level validation/build/audit by component; browser E2E is explicit/opt-in. Its pre-push hook runs full validation only for a push to the integration branch and only for affected components.
- ai-code now runs a fast `pnpm guardrail:fast` pre-commit hook, explicit full/release/package gates, and a main-only full pre-push hook. Fast mode omits broad tests/build/audit; full mode runs the affected stack's declared closure checks. `check-test-layout.mjs` preserves historical tests while rejecting newly-added isolated unit tests.

## Target policy

1. **No permanent unit tests:** New isolated/mocked unit tests are temporary debugging aids only and must be removed before staging/commit. Existing unit tests are audited: tests that do not exercise a real ai-code boundary are removed; security, transport, catalog, sandbox, integration, contract, smoke, and regression tests remain when they prove relevant behavior.
2. **Boundary tests remain valid:** New integration, E2E, contract, smoke, security, and regression tests that exercise a real boundary are allowed in approved test directories.
3. **Fast implementation loop:** Fast validation runs repository policy, agent guidance, architecture, and test-policy checks, then only lint/typecheck for changed Nuxt/Rust stacks. It does not run broad test suites, dependency audits, maintainability, or builds.
4. **Explicit full/release validation:** Full validation adds closure-only maintainability, builds, and declared stack tests for affected stacks. Dependency audits are opt-in with `AI_CODE_GUARD_RUN_AUDIT=1`; release adds release artifacts. Existing legacy TypeScript unit corpus is not an automatic guard stage.
5. **Integration push enforcement:** The tracked pre-push hook determines changed stack scope for pushes to `main`, runs the affected full guard serially, and checks the pushed diff. Branch checkpoint commits and non-main pushes do not replay full validation automatically.
6. **No weakened non-test safeguards:** Repository policy, architecture, maintainability, agent-document integrity, immutable catalog tests where applicable, and source security boundaries remain enforced.

## Implementation phases

### Phase 1 — Sensio-style guard model and changed-range scope

- [x] Extend `scripts/guardrail.sh` to accept an explicit `fast`, `full`, or `release` mode while retaining concise Nuxt/Rust scope selection.
- [x] Use a bounded changed-file source that supports working/staged edits and pre-push base/head ranges.
- [x] Keep structural checks in every mode; run only changed-stack lint/typecheck in fast mode.
- [x] Make full/release work explicit and avoid invoking unrelated stack validation.
- [x] Add ai-code equivalents of Sensio's component policy and closure-only maintainability budget without importing its multi-codebase registry, Docker environment guard, or product-specific OpenAPI checks.

### Phase 2 — Permanent test policy

- [x] Extend the existing test-layout guard with Sensio-equivalent added-path policy: reject new permanent `test/unit/**`, `*.unit.test.*`, `*.unit.spec.*`, source-co-located JS/TS tests, and Rust inline `#[cfg(test)]` additions.
- [x] Preserve current approved test locations for valid boundary-level TypeScript and Rust tests.
- [x] Classify the existing `test/unit/` corpus. All 27 surviving files exercise current security, MCP, activity, orchestration, or contract behavior; no isolated/non-boundary deletion was justified. They remain available through the documented manual command.
- [x] Document that temporary unit tests must be removed before staging and that existing tests are historical rather than a template for new additions.

### Phase 3 — Hook and package interface

- [x] Replace the pre-commit full guard with Sensio-style fast-development behavior.
- [x] Add a tracked pre-push hook that applies full validation only to pushes targeting `main`, with exact changed-range scope.
- [x] Add package commands for fast, full, release, Nuxt, Rust, maintainability closure, and explicit legacy/manual test access without adding dependencies.
- [x] Ensure hook installation enables every tracked hook.

### Phase 4 — Guidance and validation

- [x] Update `AGENTS.md`, `.agents` guidance, and canonical memory so the test policy and lifecycle are unambiguous.
- [x] Add behavior-named guard fixtures within the existing guard architecture, not plan-numbered scripts; the added-unit rejection was exercised with a temporary intent-to-add fixture and the fixture was removed.
- [x] Validate fast Nuxt, fast Rust, full Nuxt, full Rust, changed-range pre-push behavior, test-policy rejection, historical test preservation, hook syntax, and normal branch commit behavior.
- [x] Run final cross-stack guardrail, review the diff for accidental policy bypasses, update this plan truthfully, commit, push, merge the PR, and delete the task branch/worktree.

## Risks and guardrails

- Classify every deletion against the current source and boundary. Remove only task-owned legacy tests that are isolated/non-boundary or stale; do not delete security, sandbox, MCP, transport, catalog, integration, contract, smoke, or regression coverage merely because the filename contains `unit`.
- Do not classify tests only from extension. A test that crosses a real application/security/transport boundary is permitted even if it uses a generic test framework.
- Do not copy Sensio's codebase registry, Docker environment guard, or multi-product serial dispatcher into ai-code; they solve monorepo problems ai-code does not have.
- Do not make pre-push run broad validation for ordinary feature branches. `main` integration pushes are the explicit full-gate boundary.
- Do not remove existing architecture, maintainability, agent-doc, catalog, or security checks to improve speed.

## Acceptance criteria

- [x] New permanent isolated unit tests are rejected before commit/closure, and unrelated legacy unit tests are removed only after documented classification.
- [x] New boundary-level integration/E2E/contract/smoke/security/regression tests remain allowed in approved locations.
- [x] Fast Nuxt and Rust modes run only their relevant lint/typecheck plus structural guards.
- [x] Full Nuxt and Rust modes run their declared closure validations without running unrelated stacks.
- [x] Surviving legacy TypeScript checks are available through an explicit manual command and are not an automatic fast/full guard stage.
- [x] `main` pre-push runs full validation only for affected scope and rejects a failed pushed diff.
- [x] Ordinary branch checkpoint commits do not run a full test suite through the hook.
- [x] Documentation, canonical memory, and package commands describe the final behavior accurately.
- [x] No production, release, deployment, or force-push occurs; merge and branch cleanup remain the final delivery actions authorized by the user.
