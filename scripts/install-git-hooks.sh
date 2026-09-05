#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! git -c core.bare=false --work-tree=. rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  printf 'git-hooks: not inside a Git worktree; skipping hook installation.\n'
  exit 0
fi

chmod +x .githooks/pre-commit .githooks/pre-push
git -c core.bare=false --work-tree=. config --local core.hooksPath .githooks

printf 'git-hooks: installed fast pre-commit and main-only full pre-push gates.\n'
