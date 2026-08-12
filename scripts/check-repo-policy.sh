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

# Unit-test suites are intentionally forbidden. Deterministic acceptance and
# security scripts under scripts/ are allowed; these patterns target normal
# unit-test locations/names in application and package source trees.
tracked_files="$(git ls-files)"
if grep -Eq '(^|/)(test|tests|__tests__)/' <<<"$tracked_files"; then
  fail 'tracked unit-test directory found (test/, tests/, or __tests__/); this repository has no unit-test suite'
fi

source_files="$(git ls-files app server shared packages)"
if grep -Eq '(^|/)[^/]+\.(test|spec)\.[^/]+$' <<<"$source_files"; then
  fail 'tracked *.test.* or *.spec.* source file found; this repository has no unit-test suite'
fi

if grep -R -n -F '#[cfg(test)]' packages/rust-tools/src >/dev/null 2>&1; then
  fail 'Rust #[cfg(test)] unit-test module found; this repository has no unit-test suite'
fi

while IFS= read -r file; do
  if grep -Eq '^[[:space:]]*"test"[[:space:]]*:' "$file"; then
    fail "package.json test script found in $file; this repository has no unit-test suite"
  fi
done < <(git ls-files 'package.json' 'packages/*/package.json')

printf 'repo-policy: OK — no CI workflow and no unit-test suite tracked.\n'
