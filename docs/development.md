# Development

## Branch model

Long-lived branches:

```text
dev   integration
main  release
```

Repository workflow is intentionally asymmetric:

- documentation/planning-only changes may be committed directly to `dev`;
- implementation/runtime/config/dependency/script changes use a short-lived branch targeting `dev`;
- `dev` -> `main` release promotion requires an explicit PR/release decision.

Do not commit implementation directly to `main` or `dev`.

## Mandatory local gate

This repository intentionally has no GitHub Actions CI workflow and no unit-test suite. Normal local commits are protected by the tracked pre-commit hook installed during `pnpm install`.

For capability-boundary changes, run `sh scripts/verify-capability-policy.sh`. It checks the canonical protected-path semantics, the fail-closed terminal network default, and the first-party policy module's lint contract.

Run manually with:

```bash
pnpm verify:commit
```

The gate includes repository policy, agent-doc integrity, architecture checks, deterministic maintainability budgets, JS/Vue + Rust linting, generated-project Vue typing, and warnings-denied Rust compile checks.

Never bypass it with `git commit --no-verify` or by changing `core.hooksPath`.


## Maintainability guardrails

`pnpm verify:commit` runs `node scripts/check-maintainability.mjs` before lint/type checks. The checker uses one in-repository policy source and covers maintained TypeScript/JavaScript/Vue/Rust/CSS source under `app/`, `server/`, `shared/`, and `packages/`, while excluding generated/build/vendor/migration/evidence-style output.

Current guardrails:

- target source files should stay around 300 lines where cohesion permits; >400 lines is reported for responsibility review; >500 lines fails unless an exact-path exception has a concrete cohesion reason;
- cohesive implementation folders target roughly 10–12 direct source files; 13–15 is reported for review; >15 fails unless an exact-path exception has a concrete cohesion reason;
- function-size decomposition remains a review concern rather than a custom parser rule;
- never split code solely to satisfy counts. Prefer one clear owner, DRY policy, pragmatic SOLID/layering, YAGNI, and KISS.

Run the checker directly with `node scripts/check-maintainability.mjs`; run its deterministic negative probes with `node scripts/check-maintainability.mjs --self-test`. Architecture/folder/module changes are not complete until relevant operator docs and agent guidance describe the final ownership.

## Common commands

| Task | Command |
| --- | --- |
| Development server | `pnpm dev` |
| Production build | `pnpm build` |
| Production preview | `pnpm preview` |
| Build native tools | `pnpm build:tools` |
| Lint | `pnpm lint` |
| Type/compile check | `pnpm typecheck` |
| Mandatory commit gate | `pnpm verify:commit` |
| Dependency audit | `pnpm audit` |
| Generate DB migration | `pnpm db:generate` |
| Apply DB migrations | `pnpm db:migrate` |

For final runtime verification, prefer `pnpm build && pnpm preview` over a long-lived dev watcher after branch/file changes.

## Security/protocol acceptance scripts

Targeted deterministic scripts exist for areas where static lint/type checks are insufficient. They are acceptance/security checks, not a unit-test suite.

Examples:

```bash
bash scripts/phase4-black-box.sh
bash scripts/phase7-external-mcp-contract.sh
bash scripts/phase-039c-contract.sh
bash scripts/verify-git-patch-tools.sh
bash scripts/phase8-zero-bypass.sh
bash scripts/verify-no-secret-leakage.sh
bash scripts/verify-telemetry-endpoint-security.sh
```

For Rust security-sensitive work also run `cargo audit` when applicable. For dependency changes run at least `pnpm audit`, `pnpm verify:commit`, and `pnpm build` before merge.

## Source-of-truth hierarchy

For current behavior use:

1. current source/config;
2. `docs/` for human/operator usage;
3. `.agents/knowledge/` + `.agents/memories/README.md` for agent implementation constraints;
4. numbered plans/contracts/evidence for historical design and proof context.

Do not reconstruct current architecture from an old plan snapshot when source has moved on.
