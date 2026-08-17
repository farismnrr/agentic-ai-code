#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/packages/rust-tools"
printf 'session_id canary\n' > "$tmp/packages/rust-tools/forbidden.rs"
if PHASE8_SCAN_ROOT="$tmp" bash "$root/scripts/phase8-zero-bypass.sh" >/dev/null 2>&1; then
  echo 'phase8 canary acceptance: forbidden source unexpectedly passed' >&2
  exit 1
fi
bash "$root/scripts/phase8-zero-bypass.sh" >/dev/null
echo 'phase8 canary acceptance: pass'
