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
for path in CLAUDE.md GEMINI.md .claude .gemini; do
  if [ -e "$path" ]; then
    fail "vendor-specific agent path must not be tracked: $path"
  fi
done

# Vendor-specific lifecycle/discovery wording is also forbidden in shared agent
# guidance. Product-level model/provider support (for example Anthropic-compatible
# inference) is intentionally outside this check.
if grep -RInE \
  --exclude='check-agent-docs.sh' \
  --exclude-dir='.git' \
  --exclude-dir='node_modules' \
  --exclude-dir='.nuxt' \
  --exclude-dir='.output' \
  '(Claude Code|CLAUDE\.md|\.claude/|GEMINI\.md|\.gemini/|Gemini/Antigravity)' \
  AGENTS.md .agents packages app 2>/dev/null; then
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
