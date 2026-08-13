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

# Plan 031A finding G/H: server/application coordinates narrow capabilities
# and must not directly own Drizzle persistence or provider/AI SDK
# construction — those belong to server/infrastructure and server/utils.
if rg -n "from '.*database/schema'|from 'drizzle-orm'" server/application; then
  echo 'architecture: server/application must not import Drizzle schema/drizzle-orm directly; use a server/infrastructure adapter' >&2
  exit 1
fi

if rg -n "from '@ai-sdk/|from '@langchain/" server/application; then
  echo 'architecture: server/application must not construct provider/AI SDK clients directly; use a server/infrastructure adapter' >&2
  exit 1
fi

echo 'architecture: ownership checks passed'
