#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

requested_scope="${1:-auto}"
if (($# > 1)); then
  printf 'usage: %s [auto|nuxt|rust|all]\n' "$0" >&2
  exit 2
fi

case "$requested_scope" in
  auto|nuxt|web|rust|all) ;;
  *)
    printf 'guardrail: invalid scope %q; expected auto, nuxt, rust, or all\n' "$requested_scope" >&2
    exit 2
    ;;
esac

mapfile -t staged < <(git diff --cached --name-only --diff-filter=ACMRD)
if ((${#staged[@]})); then
  changed=("${staged[@]}")
else
  mapfile -t changed < <({ git diff --name-only --diff-filter=ACMRD HEAD; git ls-files --others --exclude-standard; } | sort -u)
fi

printf 'guardrail: repository policy\n'
bash scripts/check-repo-policy.sh
printf 'guardrail: agent guidance integrity\n'
bash scripts/check-agent-docs.sh
printf 'guardrail: architecture boundaries\n'
bash scripts/check-architecture.sh
printf 'guardrail: maintainability budgets\n'
node scripts/check-maintainability.mjs

web=0
rust=0
if [[ "$requested_scope" == "auto" ]]; then
  for path in "${changed[@]}"; do
    case "$path" in
      app/*|server/*|shared/*|packages/curl-tool/*|packages/searxng-search-tool/*|packages/terminal-tool/*|packages/relay-agent/*|test/*|package.json|pnpm-lock.yaml|pnpm-workspace.yaml|nuxt.config.ts|tsconfig.json|eslint.config.mjs|drizzle.config.ts)
        web=1
        ;;
    esac
    case "$path" in
      packages/rust-tools/*|Cargo.toml|Cargo.lock)
        rust=1
        ;;
    esac
  done
else
  case "$requested_scope" in
    nuxt|web) web=1 ;;
    rust) rust=1 ;;
    all) web=1; rust=1 ;;
  esac
fi

case "$requested_scope" in
  auto)
    if ((web && rust)); then
      layout_scope=all
    elif ((web)); then
      layout_scope=nuxt
    elif ((rust)); then
      layout_scope=rust
    else
      layout_scope=all
    fi
    ;;
  all) layout_scope=all ;;
  nuxt|web) layout_scope=nuxt ;;
  rust) layout_scope=rust ;;
esac

printf 'guardrail: selected stacks (nuxt=%s rust=%s scope=%s)\n' "$web" "$rust" "$requested_scope"
printf 'guardrail: test layout (scope=%s)\n' "$layout_scope"
node scripts/check-test-layout.mjs "$layout_scope"

if ((web)); then
  printf 'guardrail: web lint\n'
  pnpm lint:web
  printf 'guardrail: web typecheck\n'
  pnpm typecheck:web
  printf 'guardrail: web unit tests\n'
  pnpm test:web
else
  printf 'guardrail: web checks skipped (no web changes)\n'
fi

if ((rust)); then
  printf 'guardrail: rust lint\n'
  pnpm lint:rust
  printf 'guardrail: rust typecheck\n'
  pnpm typecheck:rust
  printf 'guardrail: rust tests\n'
  pnpm test:rust
else
  printf 'guardrail: rust checks skipped (no Rust changes)\n'
fi

printf 'guardrail: OK\n'
