#!/usr/bin/env bash
set -euo pipefail

# Plan 035 Phase 11 — canary-secret negative test.
# Sends a distinctive, unmistakably-fake marker string through the frontend
# telemetry endpoint and (optionally) other request paths, then greps every
# captured evidence artifact for that marker. The marker must appear ONLY in
# files that intentionally recorded the raw input (e.g. a captured HTTP
# request body under .agents/contracts/035-evidence/), never inside any
# server log / Loki / Jaeger export snapshot also captured there.
#
# Usage:
#   BASE_URL=http://127.0.0.1:3333 SESSION_COOKIE="nuxt-session=..." \
#     EVIDENCE_DIR=.agents/contracts/035-evidence \
#     ./scripts/verify-no-secret-leakage.sh

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

BASE_URL="${BASE_URL:-http://127.0.0.1:3333}"
SESSION_COOKIE="${SESSION_COOKIE:-}"
EVIDENCE_DIR="${EVIDENCE_DIR:-.agents/contracts/035-evidence}"
MARKER="canary-secret-fake-token-DO-NOT-LEAK-12345"

mkdir -p "$EVIDENCE_DIR"

if [ -z "$SESSION_COOKIE" ]; then
  echo "SESSION_COOKIE not set; cannot exercise authenticated /api/telemetry canary send"
  echo "skipped=telemetry-canary-send"
else
  echo "Sending canary payload to /api/telemetry ..."
  curl -s -o /dev/null -w 'telemetry canary send -> %{http_code}\n' \
    -X POST "$BASE_URL/api/telemetry" \
    -H 'Content-Type: application/json' \
    -H "Cookie: $SESSION_COOKIE" \
    -H "Authorization: Bearer $MARKER" \
    -d "[{\"level\":\"error\",\"message\":\"chat.execute\",\"attributes\":{\"password\":\"$MARKER\",\"cookie\":\"$MARKER\",\"api_key\":\"$MARKER\",\"prompt\":\"$MARKER\"}}]"
fi

echo
echo "== Grepping captured evidence for marker leakage (outside intentional-input files) =="
found=0
if [ -d "$EVIDENCE_DIR" ]; then
  while IFS= read -r -d '' f; do
    case "$(basename "$f")" in
      *canary-secret-test-results.md|*-request-body.*|*canary-input*|*value-level-secret-redaction-results.md)
        continue
        ;;
    esac
    if grep -l "$MARKER" "$f" >/dev/null 2>&1; then
      echo "LEAK FOUND: $f contains the canary marker"
      found=1
    fi
  done < <(find "$EVIDENCE_DIR" -type f -print0)
else
  echo "Evidence dir $EVIDENCE_DIR does not exist yet — nothing to check"
fi

if [ "$found" -eq 0 ]; then
  echo "PASS: canary marker not found in any non-input evidence file"
else
  echo "FAIL: canary marker leaked into captured evidence"
fi

[ "$found" -eq 0 ]
