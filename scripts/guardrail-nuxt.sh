#!/usr/bin/env bash
set -euo pipefail
mode="${1:-fast}"
case "$mode" in fast|full|release) ;; *) echo 'guardrail-nuxt: expected fast, full, or release' >&2; exit 2 ;; esac
# Nuxt auth requires a session secret even for a build-only local gate. Keep
# this deterministic fallback process-local; operator environment wins.
export NUXT_SESSION_PASSWORD="${NUXT_SESSION_PASSWORD:-ai-code-local-guardrail-session-password-0123456789abcdef}"
pnpm exec nuxt prepare --dotenv .env.example
pnpm lint:web
pnpm typecheck:web
if [[ "$mode" != fast ]]; then
  pnpm build
  if [[ "${AI_CODE_GUARD_RUN_AUDIT:-0}" == 1 ]]; then
    pnpm audit:web
  else
    printf 'guardrail-nuxt: dependency audit deferred; set AI_CODE_GUARD_RUN_AUDIT=1 for dependency-change closure\n'
  fi
fi
printf 'AI_CODE_GUARD_PASS scope=nuxt mode=%s\n' "$mode"
