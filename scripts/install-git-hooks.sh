#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  printf 'git-hooks: not inside a Git worktree; skipping hook installation.\n'
  exit 0
fi

chmod +x .githooks/pre-commit
git config --local core.hooksPath .githooks

printf 'git-hooks: installed .githooks/pre-commit as the local commit gate.\n'
