#!/usr/bin/env bash
# Stop hook: if source files changed but .agents/ wasn't updated, nudge the agent
# to record what it learned before the turn ends.
#
# Acknowledge (no update needed) with:  touch .agents/.last-sync
# Updating any file under .agents/ also clears the nudge on the next turn.
#
# Fires at most once per session so it can never loop.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT" 2>/dev/null || exit 0

MARKER=".agents/.last-sync"
[ -f "$MARKER" ] || touch "$MARKER"

INPUT="$(cat)"
SESSION="$(printf '%s' "$INPUT" | jq -r '.session_id // "nosession"' 2>/dev/null)"
[ -n "$SESSION" ] || SESSION="nosession"

STATE_DIR=".agents/.sync-state"
STATE="$STATE_DIR/$SESSION"
[ -f "$STATE" ] && exit 0

# Watched paths that actually exist in this repo
WATCH=()
for p in app server shared modules plugins nuxt.config.ts package.json eslint.config.mjs .mcp.json; do
  [ -e "$p" ] && WATCH+=("$p")
done
[ ${#WATCH[@]} -gt 0 ] || exit 0

CHANGED="$(find "${WATCH[@]}" -type f -newer "$MARKER" \
  -not -path '*/node_modules/*' -not -path '*/.nuxt/*' -not -path '*/.output/*' \
  -print 2>/dev/null | head -15)"

[ -n "$CHANGED" ] || exit 0

mkdir -p "$STATE_DIR"
touch "$STATE"

jq -n --arg files "$CHANGED" '{
  decision: "block",
  reason: (
    "Source changed but .agents/ was not updated this session:\n" + $files +
    "\n\nBefore finishing, decide whether anything here is worth persisting:\n" +
    "- A durable decision, constraint, or trap someone could repeat -> .agents/memories/<topic>.md (add it to the index in memories/README.md)\n" +
    "- A new convention, command, or rule for how this project is built -> the right file under .agents/knowledge/\n" +
    "- Multi-step work still in flight -> .agents/plans/<effort>.md\n\n" +
    "Record only what is NOT already derivable from the code itself. " +
    "If nothing is worth saving, run `touch .agents/.last-sync` and say so briefly. " +
    "This fires once per session."
  )
}'
