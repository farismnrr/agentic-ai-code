#!/usr/bin/env bash
set -euo pipefail
mode="${1:-fast}"
case "$mode" in fast|full|release) ;; *) echo 'guardrail-rust: expected fast, full, or release' >&2; exit 2 ;; esac
pnpm lint:rust
pnpm typecheck:rust
if [[ "$mode" != fast ]]; then pnpm test:rust; fi
if [[ "$mode" == release ]]; then cargo build --release --locked; fi
printf 'AI_CODE_GUARD_PASS scope=rust mode=%s\n' "$mode"
