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
if git ls-files '.github/workflows/*' | grep -q .; then
  fail 'tracked CI workflow found under .github/workflows/; this repository is no-CI'
fi

# Unit-test suites are intentionally forbidden. Deterministic acceptance and
# security scripts under scripts/ are allowed; these patterns target normal
# unit-test locations/names in application and package source trees.
if git ls-files | grep -Eq '(^|/)(test|tests|__tests__)/'; then
  fail 'tracked unit-test directory found (test/, tests/, or __tests__/); this repository has no unit-test suite'
fi

if git ls-files app server shared packages | grep -Eq '(^|/)[^/]+\.(test|spec)\.[^/]+$'; then
  fail 'tracked *.test.* or *.spec.* source file found; this repository has no unit-test suite'
fi

if rg -n '#\[cfg\(test\)\]' packages/rust-tools/src >/dev/null 2>&1; then
  fail 'Rust #[cfg(test)] unit-test module found; this repository has no unit-test suite'
fi

if git ls-files 'package.json' 'packages/*/package.json' | while IFS= read -r file; do
  grep -Eq '^[[:space:]]*"test"[[:space:]]*:' "$file" && printf '%s\n' "$file"
done | grep -q .; then
  fail 'package.json test script found; this repository has no unit-test suite'
fi

printf 'repo-policy: OK — no CI workflow and no unit-test suite tracked.\n'
