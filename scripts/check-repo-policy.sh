#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() {
  printf 'repo-policy: %s\n' "$1" >&2
  exit 1
}

# CI is intentionally forbidden. Check tracked paths rather than the working
# tree so an empty local directory does not matter.
if [[ -n "$(git ls-files '.github/workflows/*')" ]]; then
  fail 'tracked CI workflow found under .github/workflows/; this repository is no-CI'
fi

printf 'repo-policy: OK — no CI workflow tracked; test layout is enforced by check-test-layout.\n'
