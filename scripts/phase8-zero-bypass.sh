#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

test -f packages/rust-tools/src/bin/relay-agent.rs
test -f packages/rust-tools/src/relay_agent/transport.rs
test -f scripts/phase7-chatgpt-contract.sh

# The canonical MCP surface and the remote security boundary are source-level
# release invariants. Keep these checks deterministic and independent of tests.
rg -q 'POST /mcp|/mcp' packages/rust-tools/src/relay_agent/transport.rs
rg -q 'MCP_PROTOCOL_VERSION|2026-07-28' packages/rust-tools/src/relay_agent/transport.rs
rg -q 'oauth-protected-resource|Protected Resource Metadata|protected resource' packages/rust-tools/src/relay_agent/transport.rs
rg -q 'Remote.*unauthenticated|Remote.*authentication|SecurityMode::Remote|CODING_SCOPE' packages/rust-tools/src/relay_agent/transport.rs packages/rust-tools/src/relay_agent/config.rs
rg -q 'relay\.coding' packages/rust-tools/src/relay_agent/transport.rs
rg -q 'WWW-Authenticate|Insufficient scope|invalid_token' packages/rust-tools/src/relay_agent/transport.rs
! rg -q 'offline_access' packages/rust-tools/src/relay_agent
! rg -q 'session_id|EventSource|/sse|/message' packages/rust-tools/src/relay_agent
! find packages/relay-agent -type f \( -name '*.mjs' -o -name '*.cjs' -o -name '*.js' -o -name '*.pkg' \) -print -quit | grep -q .
! rg -n 'continue-on-error:|\|\| *true|; *true' .github/workflows scripts
! rg -n '#\[allow\((dead_code|unused|warnings|clippy)' packages/rust-tools/src
! rg -n 'cargo test|npm test|pnpm test|yarn test' .github/workflows/ci.yml

echo 'phase8 zero-bypass conformance: pass'
