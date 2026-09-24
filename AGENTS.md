# Repository Agent Rules

These rules apply to AI-assisted changes in this repository.

## Scope discipline

- Follow the user's requested scope exactly.
- Do not introduce unrelated features, refactors, dependencies, or architecture changes without explicit approval.
- Prefer small, reviewable changes.

## Mandatory engineering principles

- SOLID principles are required.
- Follow Clean Architecture boundaries defined by each service.
- Prefer DRY, reusable code when there is a concrete repeated concern.
- Split folders and files by responsibility. Do not accumulate unrelated implementation in one directory.
- Rust `mod.rs` files are module manifests only: module declarations and re-exports, no implementation logic.
- Frontend `index.ts` files are barrel files only: imports/re-exports, no implementation logic.

## Validation before completion

Repository checks are defined by:

```sh
./scripts/check.sh
```

A change is not considered complete until the relevant repository checks pass.

## Automatic CI failure recovery

When a GitHub Actions run triggered by an AI-authored change fails:

1. Inspect the failed workflow run and job logs using the GitHub integration.
2. Identify the root cause from the actual failure output.
3. Fix the failure directly on the same working branch when the fix remains within the user's current requested scope.
4. Commit the fix.
5. Inspect the next CI run.
6. Repeat until the CI run passes.

Do not wait for an additional user instruction merely to fix a CI/build/guardrail failure caused by the current change.

Stop and ask the user before proceeding when a fix requires:

- new secrets or credentials
- external server access not already available
- a significant architecture or product decision
- destructive data or infrastructure changes
- a change outside the user's requested scope

Never claim a change is complete or verified while its required CI run is failing or has not been checked.
