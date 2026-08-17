#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

printf 'commit-gate: checking repository policy...\n'
bash scripts/check-repo-policy.sh

printf 'commit-gate: checking agent docs integrity...\n'
bash scripts/check-agent-docs.sh

printf 'commit-gate: checking architecture boundaries...\n'
bash scripts/check-architecture.sh

printf 'commit-gate: checking maintainability budgets...\n'
node scripts/check-maintainability.mjs

printf 'commit-gate: running all linters...\n'
pnpm lint

printf 'commit-gate: running all type checks...\n'
pnpm typecheck

printf 'commit-gate: OK — repository policy, architecture, lint, and typecheck passed; commit may proceed.\n'
