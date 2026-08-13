#!/usr/bin/env bash
set -euo pipefail

# Small deterministic ownership checks; keep this source-level and dependency-free.
if rg -n '^[[:space:]]*(use|pub[[:space:]]+use).*\b(axum|transport::|super::transport)' packages/rust-tools/src/relay_agent/mcp.rs; then
  echo 'architecture: mcp.rs must remain transport-independent' >&2
  exit 1
fi

if rg -n 'H3Event|H3Event<|EventHandlerRequest' server/application server/infrastructure; then
  echo 'architecture: application/infrastructure modules must not depend on H3 event objects' >&2
  exit 1
fi

echo 'architecture: ownership checks passed'
