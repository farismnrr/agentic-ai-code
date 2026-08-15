#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

test -f packages/rust-tools/cli/src/main.rs
test -f packages/rust-tools/infrastructure/src/transport.rs
test -f scripts/phase7-external-mcp-contract.sh

# The canonical MCP surface and the remote security boundary are source-level
# release invariants. Keep these checks deterministic and independent of tests.
# HTTP status/header/body and dispatch-order behavior is exercised by the
# phase4 black-box harness. These checks remain structural only.
! rg -q 'offline_access' packages/rust-tools
! rg -q 'session_id|EventSource|/sse|/message' packages/rust-tools
! find packages/rust-tools -type f \( -name '*.mjs' -o -name '*.cjs' -o -name '*.js' -o -name '*.pkg' \) -print -quit | grep -q .
! rg -n '#\[allow\((dead_code|unused|warnings)' packages/rust-tools

# Repository policy: no GitHub Actions/CI workflow is tracked. Local commit
# gates own lint/type enforcement instead.
if [ -d .github/workflows ] && find .github/workflows -type f -print -quit | grep -q .; then
  echo 'phase8: CI workflow found, but repository policy is no CI' >&2
  exit 1
fi

# Local verification scripts must fail closed rather than masking failures.
! rg -n '\|\| *true|; *true' scripts

echo 'phase8 zero-bypass conformance: pass'
