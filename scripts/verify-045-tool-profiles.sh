#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"; cd "$root"
cargo run --quiet --locked -p relay-application --example plan045_tool_profiles_acceptance
rg -q 'RELAY_TOOL_PROFILE' packages/rust-tools/core/src/config/cli.rs
rg -q 'default_value = "full"' packages/rust-tools/core/src/config/cli.rs
rg -q 'tool_profile: ToolProfile::Full' packages/rust-tools/core/src/config.rs
rg -q 'ToolProfile::Full' packages/rust-tools/infrastructure/src/transport/tools.rs
rg -q 'tool_catalog_for_profile' packages/rust-tools/infrastructure/src/transport/mcp_http.rs
test -x scripts/fixtures/verify-045-relay-profile.sh
# The ChatGPT-facing wrapper must widen an inherited fast-path profile to the
# complete catalog explicitly.
RELAY_TOOL_PROFILE=primary \
REMOTE_MCP_URL='https://mcp.example.com/mcp' \
OAUTH_ISSUER='https://auth.example.com/' \
OAUTH_OWNER_SUBJECT='plan045-test-subject' \
EXECUTION_ROOT="$root" \
RELAY_WORKING_DIR="$root" \
RELAY_AGENT_PORT=47899 \
AI_TOOLS_BIN="$root/scripts/fixtures/verify-045-relay-profile.sh" \
bash scripts/phase36-start-remote-relay.sh
echo '045 tool profile deterministic verification: PASS'
