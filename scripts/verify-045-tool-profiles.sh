#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"; cd "$root"
cargo run --quiet --locked -p relay-application --example plan045_tool_profiles_acceptance
rg -q 'RELAY_TOOL_PROFILE' packages/rust-tools/core/src/config/cli.rs
rg -q 'ToolProfile::Full' packages/rust-tools/infrastructure/src/transport/tools.rs
rg -q 'tool_catalog_for_profile' packages/rust-tools/infrastructure/src/transport/mcp_http.rs
echo '045 tool profile deterministic verification: PASS'
