#!/usr/bin/env bash
# Plan-048 acceptance: local CLI authentication discovery and live provider filtering.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo run --quiet --locked -p relay-application --example plan045_tool_profiles_acceptance

! rg -n 'agent_delegate|agent[-_]env|agent[-_]auth|allow[-_]agent[-_]network' \
  packages/rust-tools/application/src packages/rust-tools/core/src \
  packages/rust-tools/infrastructure/src packages/rust-tools/interfaces/src \
  packages/rust-tools/application/Cargo.toml
echo 'Plan 050 provider capability removal verification: PASS'
