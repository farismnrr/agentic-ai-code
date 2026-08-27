# Development

## Branch model

`main` is the canonical long-lived branch. Implementation work uses a short-lived branch, a focused commit, and a pull request into `main`. Do not commit implementation directly to `main`.

## Local quality model

This repository intentionally has no hosted CI workflow. Local quality is split into two layers:

1. **Guardrails** protect repository-wide structural invariants.
2. **Tests** prove feature behavior in the stack that owns it.

`scripts/` is reserved for guardrails and hook installation. Do not add plan-numbered `verify-*`, `phase-*`, acceptance, or one-off feature validation scripts there.

Web/Node tests live under top-level `test/`. Rust tests use Cargo's package-local `tests/` directories. Production files do not contain inline test modules.

## Mandatory local guardrail

`pnpm install` configures the tracked `.githooks/pre-commit` hook. Run the same check manually with:

```bash
pnpm guardrail
```

The guardrail always checks repository policy, agent guidance, architecture boundaries, maintainability budgets, and test layout. It then looks at changed paths and runs only the applicable stack:

- web changes: `pnpm lint:web`, `pnpm typecheck:web`, `pnpm test:web`;
- Rust changes: `pnpm lint:rust`, `pnpm typecheck:rust`, `pnpm test:rust`;
- cross-stack changes: both sets.

A Nuxt-only change must not compile, lint, or test Rust merely because both languages live in the same repository. Rust-only changes follow the same rule in the other direction. Validate both only when both stacks or a real shared contract changed.

Never bypass the hook with `git commit --no-verify` or by changing `core.hooksPath`.

## Tests

Use feature/behavior names, not plan numbers. Examples:

```text
test/unit/capability-policy.test.ts
test/unit/mcp-task-reliability.test.ts
packages/rust-tools/infrastructure/tests/activity.rs
```

Use Node's native test runner for repository web tests and Cargo for Rust. No new third-party test framework is required for ordinary coverage.

Run explicitly:

```bash
pnpm test:web
pnpm test:rust
```

`pnpm test` is an explicit full-repository convenience command and runs both; it is not the default pre-commit behavior.

## Maintainability guardrails

`node scripts/check-maintainability.mjs` covers maintained TypeScript/JavaScript/Vue/Rust/CSS source under `app/`, `server/`, `shared/`, and `packages/` while excluding generated/build/vendor/migration/evidence output.

Current guardrails:

- >400 source lines is a responsibility-review signal; >500 fails unless an exact-path exception has a concrete cohesion reason;
- 13–15 direct maintained files in one cohesive implementation folder is a review signal; >15 fails unless an exact-path exception has a concrete cohesion reason;
- do not split code solely to satisfy counts; prefer clear ownership, DRY policy, pragmatic SOLID/layering, YAGNI, and KISS.

`node scripts/check-test-layout.mjs` enforces the test locations and rejects inline test modules in production source.

## Common commands

| Task | Command |
| --- | --- |
| Development server | `pnpm dev` |
| Production build | `pnpm build` |
| Production preview | `pnpm preview` |
| Build native tools | `pnpm build:tools` |
| Web lint | `pnpm lint:web` |
| Rust lint | `pnpm lint:rust` |
| Web typecheck | `pnpm typecheck:web` |
| Rust typecheck | `pnpm typecheck:rust` |
| Web tests | `pnpm test:web` |
| Rust tests | `pnpm test:rust` |
| Stack-aware commit guardrail | `pnpm guardrail` |
| Dependency audit | `pnpm audit` |
| Generate DB migration | `pnpm db:generate` |
| Apply DB migrations | `pnpm db:migrate` |

The combined `pnpm lint`, `pnpm typecheck`, and `pnpm test` commands intentionally run both stacks when a full-repository pass is actually desired.

For final UI/runtime verification, prefer `pnpm build && pnpm preview` over a long-lived dev watcher after branch/file changes. For Rust security-sensitive work, run `cargo audit` when applicable. For dependency changes run at least the applicable tests, `pnpm guardrail`, the relevant audit, and the affected build before merge.

Operational/deployment/release helpers live under `ops/`, not `scripts/`.

## Source-of-truth hierarchy

For current behavior use:

1. current source/config;
2. `docs/` for human/operator usage;
3. `.agents/knowledge/` + `.agents/memories/README.md` for agent implementation constraints;
4. numbered plans/contracts/evidence for historical design and proof context.

Historical references to removed verification scripts record what was executed at that time; they are not instructions to recreate those scripts.
