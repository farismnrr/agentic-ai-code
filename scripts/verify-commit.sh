#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if git diff --cached --name-only | grep -Eq '^\.agents/039h-.*\.md$'; then
  echo 'commit-gate: 039H prompt artifacts must remain untracked' >&2
  exit 1
fi

printf 'commit-gate: checking repository policy...\n'
bash scripts/check-repo-policy.sh

printf 'commit-gate: checking agent docs integrity...\n'
bash scripts/check-agent-docs.sh

printf 'commit-gate: checking architecture boundaries...\n'
bash scripts/check-architecture.sh

printf 'commit-gate: checking maintainability budgets...\n'
node scripts/check-maintainability.mjs

printf 'commit-gate: checking subagent policy and lifecycle behavior...\n'
pnpm verify:subagents
pnpm verify:background-agents
printf 'commit-gate: checking task/context/output bounds...\n'
pnpm verify:task-context-output
printf 'commit-gate: checking current MCP contract...\n'
bash scripts/phase-039h-contract.sh
bash scripts/phase-039i-contract.sh

printf 'commit-gate: running all linters...\n'
pnpm lint

printf 'commit-gate: running all type checks...\n'
pnpm typecheck

printf 'commit-gate: OK — repository policy, architecture, lint, and typecheck passed; commit may proceed.\n'
