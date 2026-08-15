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
# Checked against git tracking, not raw filesystem existence: local tooling
# (for example an untracked .claude/ runtime directory used by a Claude Code
# session) may legitimately exist on disk without being repository-owned.
for path in CLAUDE.md GEMINI.md .claude .gemini; do
  if git ls-files --error-unmatch "$path" >/dev/null 2>&1; then
    fail "vendor-specific agent path must not be tracked: $path"
  fi
done

# Canonical/shared guidance and top-level skills stay client/vendor neutral.
# Skill reference material may document factual interoperability flags for
# external tools, so references are intentionally outside this guidance scan.
if grep -RInE \
  --exclude='check-agent-docs.sh' \
  --exclude-dir='.git' \
  --exclude-dir='node_modules' \
  --exclude-dir='.nuxt' \
  --exclude-dir='.output' \
  '([Cc]laude|CLAUDE\.md|\.claude/|GEMINI\.md|\.gemini/|Gemini/Antigravity)' \
  README.md AGENTS.md .agents/knowledge .agents/memories .agents/plans .agents/contracts \
  .agents/skills/*/SKILL.md packages/*/SKILL.md 2>/dev/null; then
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

# Plan 030 is the one-time pre-reset historical summary. Independent future
# plans use incrementing NNN-kebab-case names starting at 031. A closed plan
# may also have a focused lowercase-letter follow-up (for example 031a-*.md)
# when the user explicitly keeps the work under that parent plan family.
if [ ! -f '.agents/plans/030-previous-plans-summary.md' ]; then
  fail 'missing historical plan snapshot: .agents/plans/030-previous-plans-summary.md'
fi
if [ -e '.agents/plans/README.md' ]; then
  fail 'plans/README.md must not be reintroduced; numbered plan files own their status'
fi

declare -A seen_plan_keys=()
while IFS= read -r file; do
  base="$(basename "$file")"
  if [[ ! "$base" =~ ^([0-9]{3})([a-z]?)-[a-z0-9][a-z0-9-]*\.md$ ]]; then
    fail "invalid plan filename: $file (expected NNN-kebab-case.md or NNNx-kebab-case.md follow-up)"
    continue
  fi

  prefix="${BASH_REMATCH[1]}"
  suffix="${BASH_REMATCH[2]}"
  key="${prefix}${suffix}"
  number=$((10#$prefix))

  if (( number < 30 )); then
    fail "pre-reset plan file must stay compacted into Plan 030: $file"
  fi
  if (( number == 30 )) && { [ -n "$suffix" ] || [ "$base" != '030-previous-plans-summary.md' ]; }; then
    fail "plan number 030 is reserved for 030-previous-plans-summary.md: $file"
  fi
  if [ -n "$suffix" ] && (( number < 31 )); then
    fail "lettered follow-up plans are only valid for Plan 031 and later: $file"
  fi
  if [ -n "$suffix" ] && ! compgen -G ".agents/plans/${prefix}-*.md" >/dev/null; then
    fail "lettered follow-up $file has no parent Plan ${prefix} file"
  fi
  if [ -n "${seen_plan_keys[$key]:-}" ]; then
    fail "duplicate plan key $key: ${seen_plan_keys[$key]} and $base"
  else
    seen_plan_keys[$key]="$base"
  fi
done < <(find .agents/plans -maxdepth 1 -type f -name '*.md' -print | sort)

if [ "$failed" -ne 0 ]; then
  exit 1
fi

printf 'agent-docs: OK — general guidance, one canonical memory, Plan 030 history, numbered plans with optional lettered follow-ups.\n'
