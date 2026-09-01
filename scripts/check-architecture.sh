#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
application_root="${ARCHITECTURE_APPLICATION_ROOT:-$repo_root/server/application}"
api_root="${ARCHITECTURE_API_ROOT:-$repo_root/server/api}"
core_root="${ARCHITECTURE_CORE_ROOT:-$repo_root/server/core}"
rust_package_root="$repo_root/packages/rust-tools"
rust_root="${ARCHITECTURE_RUST_ROOT:-$rust_package_root/src}"

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

# The native implementation is intentionally one Cargo package. Reintroducing
# nested package manifests would recreate the packaging boundary Plan 064 removes.
nested_rust_manifests="$(find "$rust_package_root" -mindepth 2 -name Cargo.toml -print 2>/dev/null || true)"
if [[ -n "$nested_rust_manifests" ]]; then
  printf 'architecture: packages/rust-tools must remain one Cargo package; nested manifests are forbidden\n%s\n' "$nested_rust_manifests" >&2
  exit 1
fi

legacy_rust_namespaces="$(rg -n --glob '*.rs' 'relay_(core|application|infrastructure|interfaces)::' "$rust_root" 2>/dev/null || true)"
if [[ -n "$legacy_rust_namespaces" ]]; then
  printf 'architecture: legacy internal relay-* crate namespaces are forbidden in the single-crate tree\n%s\n' "$legacy_rust_namespaces" >&2
  exit 1
fi

check_rust_boundaries() {
  local root="$1"
  fail_matches 'rust core must not depend on application/interfaces/infrastructure' \
    "$root/core" \
    '(crate::|ai_tools::|super::)(application|interfaces|infrastructure)(::|[[:space:];,{])' || return 1
  fail_matches 'rust interfaces must not depend on application/infrastructure' \
    "$root/interfaces" \
    '(crate::|ai_tools::|super::)(application|infrastructure)(::|[[:space:];,{])' || return 1
  fail_matches 'rust application must not depend on infrastructure' \
    "$root/application" \
    '(crate::|ai_tools::|super::)infrastructure(::|[[:space:];,{])' || return 1

  # The protocol core must not acquire HTTP/transport dependencies.
  if [[ -f "$root/interfaces/mcp.rs" ]] && rg -n \
    '^[[:space:]]*(use|pub[[:space:]]+use).*(axum|transport::|super::transport)' \
    "$root/interfaces/mcp.rs"; then
    echo 'architecture: interfaces/mcp.rs must remain transport-independent' >&2
    return 1
  fi
}

check_rust_boundaries "$rust_root"

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

# server/core owns error semantics/data only — it must never import concrete
# infrastructure implementations. Plan 035 P1: catches value, type-only,
# aliased, re-export, and dynamic imports, at any relative-path depth
# reachable from server/core/** (../infrastructure/, ../../infrastructure/,
# etc.), plus the '#server/infrastructure' alias form.
fail_matches 'server/core must not import infrastructure implementations' \
  "$core_root" \
  "(from|import\\()[[:space:]]*['\"][^'\"]*(server/)?infrastructure/|from[[:space:]]+['\"](\\.\\./)+infrastructure/"

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
  mkdir -p "$fixture_dir/application" "$fixture_dir/api" "$fixture_dir/core"
  printf "import type { Contract } from './contract'\n" > "$fixture_dir/application/positive.ts"
  for fixture in type-only facade; do
    rm -f "$fixture_dir/application/type-only.ts" "$fixture_dir/application/facade.ts"
    if [[ "$fixture" == type-only ]]; then
      printf "import type { Db } from '../infrastructure/database/db'\n" > "$fixture_dir/application/type-only.ts"
    else
      printf "export { value } from '../infrastructure/database/db'\n" > "$fixture_dir/application/facade.ts"
    fi
    if ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" ARCHITECTURE_CORE_ROOT="$fixture_dir/core" \
      "$0" --skip-fixtures >/dev/null 2>&1; then
      echo "architecture: negative application fixture '$fixture' was not rejected" >&2
      return 1
    fi
  done
  rm -f "$fixture_dir/application/type-only.ts" "$fixture_dir/application/facade.ts"
  printf "import { schema } from '../database/schema'\n" > "$fixture_dir/api/direct-db.ts"
  if ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" ARCHITECTURE_CORE_ROOT="$fixture_dir/core" \
    "$0" --skip-fixtures >/dev/null 2>&1; then
    echo 'architecture: negative API database fixture was not rejected' >&2
    return 1
  fi
  rm -f "$fixture_dir/api/direct-db.ts"
  printf "import { adapter } from '../infrastructure/adapter'\n" > "$fixture_dir/api/direct-infrastructure.ts"
  if ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" ARCHITECTURE_CORE_ROOT="$fixture_dir/core" \
    "$0" --skip-fixtures >/dev/null 2>&1; then
    echo 'architecture: negative API infrastructure fixture was not rejected' >&2
    return 1
  fi
  rm -f "$fixture_dir/api/direct-infrastructure.ts"

  # server/core -> server/infrastructure negative fixtures, mirroring the
  # application-boundary fixtures above: value import, type-only import, a
  # re-export facade, and a deeper relative-path variation.
  printf "import type { Contract } from './contract'\n" > "$fixture_dir/core/positive.ts"
  for fixture in value type-only facade deep-relative; do
    rm -f "$fixture_dir/core/value.ts" "$fixture_dir/core/type-only.ts" "$fixture_dir/core/facade.ts" "$fixture_dir/core/deep-relative.ts"
    case "$fixture" in
      value) printf "import { logger } from '../infrastructure/observability/logger'\n" > "$fixture_dir/core/value.ts" ;;
      type-only) printf "import type { Db } from '../infrastructure/database/db'\n" > "$fixture_dir/core/type-only.ts" ;;
      facade) printf "export { logger } from '../infrastructure/observability/logger'\n" > "$fixture_dir/core/facade.ts" ;;
      deep-relative) printf "import { logger } from '../../infrastructure/observability/logger'\n" > "$fixture_dir/core/deep-relative.ts" ;;
    esac
    if ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" ARCHITECTURE_CORE_ROOT="$fixture_dir/core" \
      "$0" --skip-fixtures >/dev/null 2>&1; then
      echo "architecture: negative core fixture '$fixture' was not rejected" >&2
      return 1
    fi
  done
  rm -f "$fixture_dir/core/value.ts" "$fixture_dir/core/type-only.ts" "$fixture_dir/core/facade.ts" "$fixture_dir/core/deep-relative.ts"

  printf "import { useCase } from '../application/use-case'\n" > "$fixture_dir/api/application-route.ts"
  ARCHITECTURE_APPLICATION_ROOT="$fixture_dir/application" ARCHITECTURE_API_ROOT="$fixture_dir/api" ARCHITECTURE_CORE_ROOT="$fixture_dir/core" \
    "$0" --skip-fixtures >/dev/null

  # Single-crate Rust layering must replace the compile-time crate graph with
  # deterministic repository enforcement. Exercise the same checker against
  # positive and negative fixtures so a regex regression cannot silently pass.
  mkdir -p "$fixture_dir/rust/core" "$fixture_dir/rust/interfaces" "$fixture_dir/rust/application" "$fixture_dir/rust/infrastructure"
  printf 'pub fn core_ok() {}\n' > "$fixture_dir/rust/core/positive.rs"
  printf 'use crate::core::error::McpError;\n' > "$fixture_dir/rust/interfaces/positive.rs"
  printf 'use crate::core::error::McpError; use crate::interfaces::mcp;\n' > "$fixture_dir/rust/application/positive.rs"
  printf 'use crate::application::dispatcher;\n' > "$fixture_dir/rust/infrastructure/positive.rs"
  check_rust_boundaries "$fixture_dir/rust" >/dev/null

  for fixture in core-application core-interfaces core-infrastructure interfaces-application interfaces-infrastructure application-infrastructure; do
    rm -f "$fixture_dir/rust/core/negative.rs" "$fixture_dir/rust/interfaces/negative.rs" "$fixture_dir/rust/application/negative.rs"
    case "$fixture" in
      core-application) printf 'use crate::application::dispatcher;\n' > "$fixture_dir/rust/core/negative.rs" ;;
      core-interfaces) printf 'use crate::interfaces::mcp;\n' > "$fixture_dir/rust/core/negative.rs" ;;
      core-infrastructure) printf 'use crate::infrastructure::transport;\n' > "$fixture_dir/rust/core/negative.rs" ;;
      interfaces-application) printf 'use crate::application::dispatcher;\n' > "$fixture_dir/rust/interfaces/negative.rs" ;;
      interfaces-infrastructure) printf 'use crate::infrastructure::transport;\n' > "$fixture_dir/rust/interfaces/negative.rs" ;;
      application-infrastructure) printf 'use crate::infrastructure::transport;\n' > "$fixture_dir/rust/application/negative.rs" ;;
    esac
    if check_rust_boundaries "$fixture_dir/rust" >/dev/null 2>&1; then
      echo "architecture: negative Rust fixture '$fixture' was not rejected" >&2
      return 1
    fi
  done
  rm -f "$fixture_dir/rust/core/negative.rs" "$fixture_dir/rust/interfaces/negative.rs" "$fixture_dir/rust/application/negative.rs"
  check_rust_boundaries "$fixture_dir/rust" >/dev/null
}

if [[ "${1:-}" != "--skip-fixtures" && -z "${ARCHITECTURE_SKIP_FIXTURES:-}" ]]; then
  run_fixture_checks
fi

echo 'architecture: ownership checks passed'
