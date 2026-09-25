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

## Validation model

This repository has three CI pipelines.

### Fast AMD64 pipeline — default after every completed edit/task

Normal implementation commits must not trigger CI individually.

When an AI-authored edit/task is ready to report back to the user, update `.ci/fast-trigger` in a dedicated commit. This starts the fast pipeline.

The Fast AMD64 pipeline must run, in order:

1. lint/typecheck/Rust fmt/Clippy with warnings denied
2. structural guardrail
3. production build for `linux/amd64`
4. publish the non-deployment AMD64 validation image

The Fast AMD64 pipeline is the default completion gate for ordinary edits that are not already in an explicit deployment/full-validation flow. Inspect the resulting GitHub Actions run and do not report the edit/task as complete until it passes.

Fast AMD64 CI publishes validation-only tags:

- `ghcr.io/farismnrr/agentic-ai-code-sso-auth:fast`
- an immutable AMD64 validation tag for the validated commit

Fast AMD64 CI must never overwrite the production `:latest` tag.

### Fast ARM64 pipeline — explicit ARM64 debugging only

Do not trigger this pipeline for ordinary edits.

Only use Fast ARM64 when the user explicitly asks for ARM64 validation/debugging, or when the active task is specifically fixing an ARM64 build/image/runtime issue.

Trigger it by updating `.ci/fast-arm64-trigger` in a dedicated commit.

The Fast ARM64 pipeline must run, in order:

1. lint/typecheck/Rust fmt/Clippy with warnings denied
2. structural guardrail
3. production build for `linux/arm64`
4. publish an ARM64-only validation image and verify the published binary is AArch64

Fast ARM64 publishes validation-only tags:

- `ghcr.io/farismnrr/agentic-ai-code-sso-auth:fast-arm64`
- an immutable ARM64 validation tag for the validated commit

Fast ARM64 must never overwrite `:fast`, `:latest`, or the production commit SHA tag.

Do not run both Fast AMD64 and Fast ARM64 for the same task unless the user explicitly requests both architectures. For ARM64-specific debugging, use Fast ARM64 only.

### Full pipeline — deployment gate only

Only run the full pipeline when the user explicitly asks to deploy, prepare for deployment, run the full pipeline, or otherwise requests deployment validation. When the user has explicitly requested full/deployment validation, skip Fast CI entirely for fixes made within that deployment flow and validate only with the Full pipeline. Do not run Fast CI before Full CI for the same deployment task.

Trigger it by updating `.ci/trigger` in a dedicated commit.

The full pipeline must run, in order:

1. lint/typecheck/Rust fmt/Clippy with warnings denied
2. structural guardrail
3. tests
4. production builds for `linux/amd64` and `linux/arm64`, serially
5. Playwright E2E
6. publish the production multi-platform image and verify its manifest

A deployment is not considered ready until the full pipeline passes.

## Automatic CI failure recovery

When a GitHub Actions run triggered by an AI-authored change fails:

1. Inspect the failed workflow run and job logs using the GitHub integration.
2. Identify the root cause from the actual failure output.
3. Fix the failure directly on the same working branch when the fix remains within the user's current requested scope.
4. Commit the fix without triggering CI for intermediate repair commits.
5. Retrigger the same pipeline that failed:
   - Fast AMD64 validation: update `.ci/fast-trigger`
   - Fast ARM64 validation: update `.ci/fast-arm64-trigger`
   - deployment/full validation: update `.ci/trigger`
6. Inspect the resulting CI run.
7. Repeat until the required pipeline passes.

Do not wait for an additional user instruction merely to fix a CI/build/guardrail failure caused by the current change.

Stop and ask the user before proceeding when a fix requires:

- new secrets or credentials
- external server access not already available
- a significant architecture or product decision
- destructive data or infrastructure changes
- a change outside the user's requested scope

Never claim a change is complete or verified while its required CI run is failing or has not been checked.

## CI-built deployment image

The self-hosted CI runner is the source of deployable `sso-auth` container images.

Only the successful full/deployment pipeline publishes:

- `ghcr.io/farismnrr/agentic-ai-code-sso-auth:latest`
- an immutable image tagged with the validated Git commit SHA

The production `:latest` manifest must include at least:

- `linux/amd64`
- `linux/arm64`

Full deployment validation must verify the published ARM64 image contains an AArch64 `/sso-auth` binary. Do not require ARM64 runtime execution under QEMU on the AMD64 self-hosted runner because host binfmt/QEMU support is not a reliable deployment gate.

Do not ask the user to rebuild `sso-auth` locally for normal deployment after a successful full CI run. The normal local update flow is to pull the repository configuration, pull the already validated container image, and recreate the service.


## CI warning hygiene

A successful workflow is not considered clean if GitHub Actions reports deprecation annotations from actions used by this repository.

After each required Fast AMD64, Fast ARM64, or Full CI run:

- inspect job annotations/logs for action runtime deprecations and workflow warnings
- do not report the task as clean while repository-controlled deprecation warnings remain
- keep Docker GitHub Actions on Node 24-compatible supported major versions
- use explicit `--output type=cacheonly` for validation-only Buildx builds that intentionally do not export an image
- fix repository-controlled warnings in the same branch and rerun the same validation level

Warnings caused solely by host kernel/daemon capabilities may be reported separately when they cannot be fixed in repository code, but they must not be confused with repository/action deprecation warnings.
