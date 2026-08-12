#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

failed=0

fail() {
  printf 'agent-docs: %s\n' "$*" >&2
  failed=1
}

# AGENTS.md + .agents/ are the only repository-owned agent guidance surfaces.
for path in EXTERNAL MCP CLIENT.md GEMINI.md .external-mcp .gemini; do
  if [ -e "$path" ]; then
    fail "vendor-specific agent path must not be tracked: $path"
  fi
done

# Shared guidance, root README, and package skills stay client/vendor neutral.
# Runtime/provider support is intentionally outside this scan.
if grep -RInE \
  --exclude='check-agent-docs.sh' \
  --exclude-dir='.git' \
  --exclude-dir='node_modules' \
  --exclude-dir='.nuxt' \
  --exclude-dir='.output' \
  '([Cc]laude|EXTERNAL MCP CLIENT\.md|\.external-mcp/|GEMINI\.md|\.gemini/|Gemini/Antigravity)' \
  README.md AGENTS.md .agents packages/*/SKILL.md 2>/dev/null; then
  fail 'vendor-specific agent guidance/reference found; use general agent wording instead'
fi

# Durable memory is intentionally compacted to one canonical Markdown file.
if [ ! -f '.agents/memories/README.md' ]; then
  fail 'missing canonical memory: .agents/memories/README.md'
fi

extra_memories="$(find .agents/memories -maxdepth 1 -type f -name '*.md' ! -name 'README.md' -print | sort)"
if [ -n "$extra_memories" ]; then
  fail "durable memory must stay in one file; unexpected memory files: $extra_memories"
fi

# Plan 030 is the one-time pre-reset historical summary. Future plans are
# separate incrementing NNN-kebab-case files starting at 031.
if [ ! -f '.agents/plans/030-previous-plans-summary.md' ]; then
  fail 'missing historical plan snapshot: .agents/plans/030-previous-plans-summary.md'
fi
if [ -e '.agents/plans/README.md' ]; then
  fail 'plans/README.md must not be reintroduced; numbered plan files own their status'
fi

declare -A seen_plan_numbers=()
while IFS= read -r file; do
  base="$(basename "$file")"
  if [[ ! "$base" =~ ^([0-9]{3})-[a-z0-9][a-z0-9-]*\.md$ ]]; then
    fail "invalid plan filename: $file (expected NNN-kebab-case.md)"
    continue
  fi

  prefix="${BASH_REMATCH[1]}"
  number=$((10#$prefix))

  if (( number < 30 )); then
    fail "pre-reset plan file must stay compacted into Plan 030: $file"
  fi
  if (( number == 30 )) && [ "$base" != '030-previous-plans-summary.md' ]; then
    fail "plan number 030 is reserved for 030-previous-plans-summary.md: $file"
  fi
  if [ -n "${seen_plan_numbers[$prefix]:-}" ]; then
    fail "duplicate plan number $prefix: ${seen_plan_numbers[$prefix]} and $base"
  else
    seen_plan_numbers[$prefix]="$base"
  fi
done < <(find .agents/plans -maxdepth 1 -type f -name '*.md' -print | sort)

if [ "$failed" -ne 0 ]; then
  exit 1
fi

printf 'agent-docs: OK — general guidance, one canonical memory, Plan 030 history, incrementing future plans.\n'
