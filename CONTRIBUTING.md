# Repository Rules

## Commit guardrail

Every commit must pass the repository checks.

Enable the tracked Git hooks once after cloning:

```sh
./scripts/setup-git-hooks.sh
```

After that, `git commit` automatically runs:

```sh
./scripts/check.sh
```

The check script runs:

- structural architecture guardrails
- the production frontend build
- the Rust backend release build through the Docker build target

If any check fails, the local commit is blocked.

Do not bypass the hook with `--no-verify`.

## Remote enforcement

GitHub Actions runs the same `./scripts/check.sh` on the repository self-hosted runner only when `.ci/trigger` is changed, or when the workflow is started manually.

Normal implementation commits do not start CI. At the end of an AI-assisted task, the agent updates `.ci/trigger` in a dedicated commit, inspects the resulting run, and fixes/retriggers until CI passes.

The workflow intentionally does not run on `pull_request` because this repository is public and untrusted fork code must not execute on the self-hosted runner.

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

The agent should inspect the failed run, fix failures that remain within the current task scope, commit the correction, and re-check CI until it passes.

Additional user approval is only required when the correction needs secrets, unavailable infrastructure access, a significant architecture/product decision, destructive changes, or work outside the requested scope.

See `AGENTS.md` for the repository-level agent rules.


## Deployment image

Final CI validation publishes the production `sso-auth` image to GitHub Container Registry only after repository checks pass.

Stable branch image:

```text
ghcr.io/farismnrr/agentic-ai-code-sso-auth:refactor-full-fe-be-relay
```

CI also publishes an immutable tag using the validated Git commit SHA.

Normal local deployment should pull this image and recreate the service instead of rebuilding the application locally:

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

Useful shortcuts include `i` for inspect mode, `c` to copy annotations, `r` to reset toolbar position, `o` to open source context when available, and `esc` to cancel the current interaction.

The inspector is a development utility and must not become part of the authentication product flow.
