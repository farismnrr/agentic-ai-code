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

allowed_scripts='scripts/check-agent-docs.sh
scripts/check-architecture.sh
scripts/check-maintainability.mjs
scripts/check-repo-policy.sh
scripts/check-test-layout.mjs
scripts/guardrail.sh
scripts/install-git-hooks.sh
scripts/guardrail-nuxt.sh
scripts/guardrail-rust.sh'
while IFS= read -r file; do
  if ! grep -Fxq "$file" <<<"$allowed_scripts"; then
    fail "scripts/ is guardrails-only; move feature tests to test/ or Rust tests/, and operational helpers to ops/: $file"
  fi
done < <(find scripts -type f -print | sort)

printf 'repo-policy: OK — no CI workflow tracked; scripts/ contains guardrails only.\n'
