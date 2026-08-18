#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"

node --experimental-strip-types scripts/verify-039j-agent-ux-observability.ts

# 039J composes rather than replaces the verified 039H/039I surfaces.
bash scripts/phase-039h-contract.sh
bash scripts/phase-039i-contract.sh

echo "phase-039j composed UX/observability contract acceptance: pass"
