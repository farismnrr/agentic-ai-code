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

GitHub Actions runs the same `./scripts/check.sh` on the repository self-hosted runner for every push.

The workflow intentionally does not run on `pull_request` because this repository is public and untrusted fork code must not execute on the self-hosted runner.

## Engineering rules

All production code must follow the architecture rules documented by each service.

For `sso-auth`, see `sso-auth/ARCHITECTURE.md`.

Current mandatory principles:

- SOLID
- Clean Architecture
- DRY and reusable code where a real repeated concern exists
- split folders and files by responsibility; do not accumulate unrelated code in one directory
