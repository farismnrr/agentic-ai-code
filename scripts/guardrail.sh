#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
git_args=(git)
if [[ "$(git rev-parse --is-bare-repository 2>/dev/null || true)" == true ]]; then
  git_args=(git -c core.bare=false --work-tree="$ROOT")
fi
scope="${1:-auto}"; mode="${2:-fast}"
[[ $# -le 2 ]] || { echo 'usage: guardrail.sh [auto|nuxt|rust|all] [fast|full|release]' >&2; exit 2; }
case "$scope" in auto|nuxt|web|rust|all) ;; *) exit 2;; esac; case "$mode" in fast|full|release) ;; *) exit 2;; esac
if [[ -n "${AI_CODE_GUARD_BASE_SHA:-}" ]]; then mapfile -t changed < <("${git_args[@]}" diff --name-only --diff-filter=ACMRD "$AI_CODE_GUARD_BASE_SHA" "$AI_CODE_GUARD_HEAD_SHA"); else mapfile -t changed < <({ "${git_args[@]}" diff --name-only --diff-filter=ACMRD; "${git_args[@]}" diff --cached --name-only --diff-filter=ACMRD; "${git_args[@]}" ls-files --others --exclude-standard; } | sort -u); fi
for check in check-repo-policy.sh check-agent-docs.sh check-architecture.sh; do bash "scripts/$check"; done
if [[ "$mode" != fast ]]; then pnpm guardrail:maintainability; fi
web=0; rust=0; for p in "${changed[@]}"; do case "$p" in app/*|server/*|shared/*|packages/{curl-tool,searxng-search-tool,terminal-tool,relay-agent}/*|test/*|package.json|pnpm-lock.yaml|pnpm-workspace.yaml|nuxt.config.ts|tsconfig.json|eslint.config.mjs|drizzle.config.ts) web=1;; packages/rust-tools/*|Cargo.toml|Cargo.lock) rust=1;; esac; done
case "$scope" in
  nuxt|web) web=1; rust=0 ;;
  rust) web=0; rust=1 ;;
  all) web=1; rust=1 ;;
esac
layout=all; ((web&&!rust))&&layout=nuxt; ((!web&&rust))&&layout=rust; node scripts/check-test-layout.mjs "$layout"
if ((web)); then bash scripts/guardrail-nuxt.sh "$mode"; fi
if ((rust)); then bash scripts/guardrail-rust.sh "$mode"; fi
printf 'AI_CODE_GUARD_PASS scope=%s mode=%s\n' "$scope" "$mode"
