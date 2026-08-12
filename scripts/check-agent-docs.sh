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
# Keep vendor client configuration out of the repository so every coding agent
# receives the same durable rules instead of a forked client-specific variant.
for path in EXTERNAL MCP CLIENT.md GEMINI.md .external-mcp .gemini; do
  if [ -e "$path" ]; then
    fail "vendor-specific agent path must not be tracked: $path"
  fi
done

# Shared guidance, the root README, and package skills must stay client/vendor
# neutral. Product runtime/provider support is intentionally outside this scan;
# an Anthropic-compatible inference adapter is a product capability, not repo
# agent guidance.
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

check_index() {
  local directory="$1"
  local index="$directory/README.md"
  local file base

  [ -f "$index" ] || {
    fail "missing index: $index"
    return
  }

  while IFS= read -r file; do
    base="$(basename "$file")"
    if ! grep -Fq "]($base)" "$index"; then
      fail "$file is not linked from $index"
    fi
  done < <(find "$directory" -maxdepth 1 -type f -name '*.md' ! -name 'README.md' -print | sort)
}

check_index '.agents/memories'
check_index '.agents/plans'

if [ "$failed" -ne 0 ]; then
  exit 1
fi

printf 'agent-docs: OK — general guidance only; plan/memory indexes are complete.\n'
