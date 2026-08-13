#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
application_root="${ARCHITECTURE_APPLICATION_ROOT:-$repo_root/server/application}"
api_root="${ARCHITECTURE_API_ROOT:-$repo_root/server/api}"

fail_matches() {
  local message="$1"
  local root="$2"
  local pattern="$3"
  local matches
  matches="$(rg -n "$pattern" "$root" 2>/dev/null || true)"
  if [[ -n "$matches" ]]; then
    printf 'architecture: %s\n%s\n' "$message" "$matches" >&2
    return 1
  fi
}

# The protocol core must not acquire HTTP/transport dependencies.
if rg -n '^[[:space:]]*(use|pub[[:space:]]+use).*(axum|transport::|super::transport)' \
  "$repo_root/packages/rust-tools/interfaces/src/mcp.rs"; then
  echo 'architecture: mcp.rs must remain transport-independent' >&2
  exit 1
fi

# Application code depends on application/shared contracts only. Keep this
# import-level check deliberately broad: it catches value, type-only, aliased,
# re-export, and dynamic imports without a dependency or AST parser.
fail_matches 'server/application must not import infrastructure implementations' \
  "$application_root" \
  "(from|import\\()[[:space:]]*['\"][^'\"]*(server/)?infrastructure/|from[[:space:]]+['\"][.]{1,2}/infrastructure/"
fail_matches 'server/application must not import Drizzle, schema, or database runtime' \
  "$application_root" \
  "(from|import\\()[[:space:]]*['\"][^'\"]*(database/schema|drizzle-orm)|\\buseDb\\b"
fail_matches 'server/application must not import H3/Nitro event types or runtime' \
  "$application_root" \
  "(from|import\\()[[:space:]]*['\"][^'\"]*(^|/)(h3|nitropack|nitro)('|\")|\\b(H3Event|EventHandlerRequest)\\b"
fail_matches 'server/application must not import AI/provider/MCP implementations' \
  "$application_root" \
  "(from|import\\()[[:space:]]*['\"](ai|@ai-sdk/|@langchain/|@modelcontextprotocol/"

# Migrated API routes compose application use cases; they do not reach the
# persistence implementation or schema directly.
api_database_matches="$(rg -n --glob '*.ts' '(from|import\()[[:space:]]*[\"'"'"'][^\"'"'"']*(server/database|database/schema|infrastructure/database|drizzle-orm)|\buseDb\b' "$api_root" --glob '!_composition.ts' 2>/dev/null || true)"
if [[ -n "$api_database_matches" ]]; then
  printf 'architecture: migrated server/api routes must not access DB/schema implementations\n%s\n' "$api_database_matches" >&2
  exit 1
fi

# Transport handlers depend on application use cases only. Infrastructure and
# composition are confined to the server plugin composition edge.
api_infrastructure_matches="$(rg -n --glob '*.ts' 'server/infrastructure|\.\./infrastructure|\.\./\.\./infrastructure' "$api_root" 2>/dev/null || true)"
if [[ -n "$api_infrastructure_matches" ]]; then
  printf 'architecture: API routes must not import infrastructure or composition\n%s\n' "$api_infrastructure_matches" >&2
  exit 1
fi

run_fixture_checks() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN
  mkdir -p "$fixture_dir/application" "$fixture_dir/api"
  printf "import type { Contract } from './contract'\n" > "$fixture_dir/application/positive.ts"
  for fixture in type-only facade; do
    rm -f "$fixture_dir/application/type-only.ts" "$fixture_dir/application/facade.ts"
    if [[ "$fixture" == type-only ]]; then
      printf "import type { Db } from '../infrastructure/database/db'\n" > "$fixture_dir/application/type-only.ts"
    else
      printf "export { value } from '../infrastructure/database/db'\n" > "$fixture_dir/application/facade.ts"
    fi
    if ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" \
      "$0" --skip-fixtures >/dev/null 2>&1; then
      echo "architecture: negative application fixture '$fixture' was not rejected" >&2
      return 1
    fi
  done
  rm -f "$fixture_dir/application/type-only.ts" "$fixture_dir/application/facade.ts"
  printf "import { schema } from '../database/schema'\n" > "$fixture_dir/api/direct-db.ts"
  if ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" \
    "$0" --skip-fixtures >/dev/null 2>&1; then
    echo 'architecture: negative API database fixture was not rejected' >&2
    return 1
  fi
  rm -f "$fixture_dir/api/direct-db.ts"
  printf "import { adapter } from '../infrastructure/adapter'\n" > "$fixture_dir/api/direct-infrastructure.ts"
  if ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" \
    "$0" --skip-fixtures >/dev/null 2>&1; then
    echo 'architecture: negative API infrastructure fixture was not rejected' >&2
    return 1
  fi
  rm -f "$fixture_dir/api/direct-infrastructure.ts"
  printf "import { useCase } from '../application/use-case'\n" > "$fixture_dir/api/application-route.ts"
  ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" \
    "$0" --skip-fixtures >/dev/null
  ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" \
    "$0" --skip-fixtures >/dev/null
}

if [[ "${1:-}" != "--skip-fixtures" && -z "${ARCHITECTURE_SKIP_FIXTURES:-}" ]]; then
  run_fixture_checks
fi

echo 'architecture: ownership checks passed'
