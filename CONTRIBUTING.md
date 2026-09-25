# Repository Rules

## Commit guardrail

Every commit must pass the repository checks.

Enable the tracked Git hooks once after cloning:

```sh
./scripts/setup-git-hooks.sh
```

Do not bypass the hook with `--no-verify`.

## Remote CI model

GitHub Actions uses two separate validation levels on the repository self-hosted runner.

### Fast CI

Fast CI is the default validation after ordinary AI-assisted edits.

It is triggered only when `.ci/fast-trigger` changes, or when its workflow is started manually.

Order:

1. lint/typecheck/Rust fmt/Clippy
2. structural guardrail
3. `linux/amd64` production build
4. publish AMD64 validation image

Fast CI publishes:

```text
ghcr.io/farismnrr/agentic-ai-code-sso-auth:fast
```

and an immutable commit-specific AMD64 validation tag.

Fast CI does not publish or modify the production `:latest` tag.

### Full deployment CI

Full CI is only for explicit deployment/full-validation requests.

It is triggered when `.ci/trigger` changes, or when its workflow is started manually.

Order:

1. lint/typecheck/Rust fmt/Clippy
2. structural guardrail
3. tests
4. serial `linux/amd64` and `linux/arm64` builds
5. Playwright E2E
6. publish and verify the multi-platform production image

Normal implementation commits do not start either CI pipeline.

The workflows intentionally do not run on `pull_request` because this repository is public and untrusted fork code must not execute on the self-hosted runner.

## Engineering rules

All production code must follow the architecture rules documented by each service.

For `sso-auth`, see `sso-auth/ARCHITECTURE.md`.

Current mandatory principles:

- SOLID
- Clean Architecture
- DRY and reusable code where a real repeated concern exists
- split folders and files by responsibility; do not accumulate unrelated code in one directory

## Automatic CI failure recovery

For AI-authored changes, a failed GitHub Actions run must be treated as part of the active task.

The agent should inspect the failed run, fix failures that remain within the current task scope, commit the correction, and re-check the same validation level until it passes.

Additional user approval is only required when the correction needs secrets, unavailable infrastructure access, a significant architecture/product decision, destructive changes, or work outside the requested scope.

See `AGENTS.md` for the repository-level agent rules.

## Deployment image

Only successful full deployment CI publishes the production image:

```text
ghcr.io/farismnrr/agentic-ai-code-sso-auth:latest
```

The production image is multi-platform and must support at least `linux/amd64` and `linux/arm64`.

Full CI also publishes an immutable tag using the validated Git commit SHA.

Normal deployment should pull this image and recreate the service instead of rebuilding the application locally:

```sh
git pull origin refactor/full-fe-be-relay
docker compose pull sso-auth
docker compose up -d --force-recreate sso-auth
```

## Visual UI inspection

The SSO frontend includes `sv-agentation` for visual UI feedback. The integration uses the Svelte-native package and is controlled by one environment flag:

```text
AGENTATION_ENABLED=true
```

Set it to `false` to keep the inspector hidden.

CI reads the GitHub Actions repository variable `AGENTATION_ENABLED` and passes it into the frontend production build. If the variable is absent, the build defaults to `false`.

The inspector is a development utility and must not become part of the authentication product flow.


## CI warning hygiene

CI success is not sufficient when repository-controlled GitHub Actions deprecation annotations are still present.

Docker actions should use supported Node 24-compatible major versions. Validation-only Buildx commands should explicitly use the `cacheonly` output exporter so they do not emit the default no-output warning.

Agents must inspect CI annotations/logs after validation and fix repository-controlled warnings before reporting a clean result.
