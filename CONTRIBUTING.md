# Repository Rules

## Commit guardrail

Every commit must pass the repository guardrail.

Enable the tracked Git hooks once after cloning:

```sh
./scripts/setup-git-hooks.sh
```

After that, `git commit` automatically runs:

```sh
node scripts/guardrail.mjs sso-auth
```

If the guardrail fails, the commit is blocked.

Do not bypass the hook with `--no-verify`.

## Engineering rules

All production code must follow the architecture rules documented by each service.

For `sso-auth`, see `sso-auth/ARCHITECTURE.md`.

Current mandatory principles:

- SOLID
- Clean Architecture
- DRY and reusable code where a real repeated concern exists
- split folders and files by responsibility; do not accumulate unrelated code in one directory
