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

# Plan 031A finding T: the checks above only caught @ai-sdk/@langchain
# *package* imports. server/application must also not import the `ai`
# package itself (or any provider SDK package) as a value — only
# `import type { ... } from 'ai'` is allowed, since the concrete tool/model
# construction surface belongs to server/infrastructure/ai/**
# (server/infrastructure/ai/chat-turn-dependencies.ts and
# server/infrastructure/ai/local-terminal-tool.ts are the narrow contract
# and adapter application is allowed to depend on).
ai_pkg_violations=$(rg -n "from '(ai|@ai-sdk/[^']*|@langchain/[^']*)'" server/application | grep -vE ':[0-9]+:import type ' || true)
if [ -n "$ai_pkg_violations" ]; then
  echo "architecture: server/application must not import the 'ai'/@ai-sdk/@langchain packages as values (type-only imports are fine):" >&2
  echo "$ai_pkg_violations" >&2
  exit 1
fi

# Plan 031A finding T: server/application must not reach concrete
# server/infrastructure/ai/** or server/infrastructure/mcp/** modules as
# values either (these are exactly the modules Phase 10 collapsed behind the
# narrow ChatTurnDependencies contract). Only `import type` from those paths
# is allowed. server/infrastructure/ai/local-terminal-tool.ts is a plain
# tool-schema builder with no provider/model/stream construction and is the
# one explicit adapter application is allowed to call as a value, matching
# the existing server/infrastructure/database/** adapter pattern.
infra_ai_mcp_violations=$(rg -n "^import .*from '.*infrastructure/(ai|mcp)/" server/application | grep -vE ':[0-9]+:import type ' | grep -v 'infrastructure/ai/local-terminal-tool' || true)
if [ -n "$infra_ai_mcp_violations" ]; then
  echo "architecture: server/application must not import server/infrastructure/ai/** or server/infrastructure/mcp/** as values (type-only imports, or the local-terminal-tool adapter, are fine):" >&2
  echo "$infra_ai_mcp_violations" >&2
  exit 1
fi

# Plan 031A finding T: application must not reach the same forbidden surface
# indirectly through a server/utils/** re-export shim. Find every
# server/utils/** module that itself imports the `ai` package, @ai-sdk/,
# @langchain/, or concrete server/infrastructure/ai|mcp modules as a value,
# then fail if server/application imports any of those specific shim files.
shim_files=$(rg -l "from '(ai|@ai-sdk/[^']*|@langchain/[^']*)'|^import .*from '.*infrastructure/(ai|mcp)/" server/utils 2>/dev/null || true)
if [ -n "$shim_files" ]; then
  for shim in $shim_files; do
    shim_name=$(basename "$shim" .ts)
    shim_hits=$(rg -n "from '(\.\./\.\./)?utils/${shim_name}'" server/application 2>/dev/null || true)
    if [ -n "$shim_hits" ]; then
      echo "architecture: server/application must not reach '${shim}' (a server/utils/** shim over forbidden AI/provider infrastructure) indirectly:" >&2
      echo "$shim_hits" >&2
      exit 1
    fi
  done
fi

echo 'architecture: ownership checks passed'
