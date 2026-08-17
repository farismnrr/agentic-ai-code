#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
scan_root="${PHASE8_SCAN_ROOT:-$root}"

test -f packages/rust-tools/cli/src/main.rs
test -f packages/rust-tools/infrastructure/src/transport.rs
test -f scripts/phase7-chatgpt-contract.sh
test -f scripts/phase-039c-contract.sh

# The canonical MCP surface and the remote security boundary are source-level
# release invariants. Keep these checks deterministic and independent of tests.
# HTTP status/header/body and dispatch-order behavior is exercised by the
# phase4 black-box harness. These checks remain structural only.
if rg -n 'offline_access' "$scan_root/packages/rust-tools"; then echo 'phase8: forbidden offline_access found' >&2; exit 1; fi
if rg -n 'session_id|EventSource|/sse|/message' "$scan_root/packages/rust-tools"; then echo 'phase8: forbidden session/legacy transport canary found' >&2; exit 1; fi
if find "$scan_root/packages/rust-tools" -type f \( -name '*.mjs' -o -name '*.cjs' -o -name '*.js' -o -name '*.pkg' \) -print -quit | grep -q .; then echo 'phase8: forbidden executable artifact found' >&2; exit 1; fi
if rg -n '#\[allow\((dead_code|unused|warnings)' "$scan_root/packages/rust-tools"; then echo 'phase8: warning suppression found' >&2; exit 1; fi

# Repository policy: no GitHub Actions/CI workflow is tracked. Local commit
# gates own lint/type enforcement instead.
if [ -d .github/workflows ] && find .github/workflows -type f -print -quit | grep -q .; then
  echo 'phase8: CI workflow found, but repository policy is no CI' >&2
  exit 1
fi

# Local verification scripts must fail closed rather than masking failures.
if rg -n '\|\| *true|; *true' scripts/phase8-zero-bypass.sh; then echo 'phase8: bypass assertion found in phase8 gate' >&2; exit 1; fi

echo 'phase8 zero-bypass conformance: pass'
