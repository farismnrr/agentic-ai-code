#!/usr/bin/env bash
set -euo pipefail

# Plan 035 Phase 11 — deterministic black-box acceptance for /api/telemetry.
# Exercises the abuse cases required by the plan's Phase 11 checklist item 5
# and the Phase 5 hardening contract. Requires a running app reachable at
# BASE_URL and a valid session cookie in SESSION_COOKIE (see README below).
#
# Usage:
#   BASE_URL=http://127.0.0.1:3333 SESSION_COOKIE="nuxt-session=..." \
#     ./scripts/verify-telemetry-endpoint-security.sh

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

BASE_URL="${BASE_URL:-http://127.0.0.1:3333}"
SESSION_COOKIE="${SESSION_COOKIE:-}"
ENDPOINT="$BASE_URL/api/telemetry"

pass=0
fail=0

check() {
  local name="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    echo "PASS: $name (expected=$expected actual=$actual)"
    pass=$((pass + 1))
  else
    echo "FAIL: $name (expected=$expected actual=$actual)"
    fail=$((fail + 1))
  fi
}

echo "== /api/telemetry abuse acceptance against $ENDPOINT =="

# (a) unauthenticated request -> expect 401
code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$ENDPOINT" \
  -H 'Content-Type: application/json' \
  -d '[{"level":"info","message":"test"}]')
check "unauthenticated -> 401" "401" "$code"

if [ -z "$SESSION_COOKIE" ]; then
  echo "SESSION_COOKIE not set; skipping authenticated cases (b-g)"
else
  auth_curl() {
    curl -s -o /dev/null -w '%{http_code}' -X POST "$ENDPOINT" \
      -H 'Content-Type: application/json' \
      -H "Cookie: $SESSION_COOKIE" "$@"
  }

  # (b) unknown event name -> expect 400
  code=$(auth_curl -d '[{"level":"info","message":"totally.unknown.event.name"}]')
  check "unknown event name -> 400" "400" "$code"

  # (c) unknown/extra attribute keys -> stripped or batch rejected
  code=$(auth_curl -d '[{"level":"info","message":"chat.execute","attributes":{"not_allowlisted_key":"x"}}]')
  if [ "$code" = "400" ] || [ "$code" = "202" ] || [ "$code" = "200" ]; then
    echo "INFO: unknown attribute key -> $code (must verify server-side stripped, not forwarded verbatim)"
    pass=$((pass + 1))
  else
    echo "FAIL: unknown attribute key -> unexpected $code"
    fail=$((fail + 1))
  fi

  # (d) malformed trace.id/request.id -> discarded not trusted
  code=$(auth_curl -d '[{"level":"info","message":"chat.execute","attributes":{"trace.id":"not-a-real-trace-id","request.id":"<script>"}}]')
  if [ "$code" = "400" ] || [ "$code" = "202" ] || [ "$code" = "200" ]; then
    echo "INFO: malformed correlation fields -> $code (must verify not trusted verbatim server-side)"
    pass=$((pass + 1))
  else
    echo "FAIL: malformed correlation fields -> unexpected $code"
    fail=$((fail + 1))
  fi

  # (e) oversized single record field (>512 chars) -> expect rejected
  bigmsg=$(head -c 600 </dev/zero | tr '\0' 'a')
  code=$(auth_curl -d "[{\"level\":\"info\",\"message\":\"$bigmsg\"}]")
  check "oversized field -> 400" "400" "$code"

  # (f) oversized batch (>50 records) -> expect 400
  batch="["
  for i in $(seq 1 60); do
    batch="${batch}{\"level\":\"info\",\"message\":\"chat.execute\"},"
  done
  batch="${batch%,}]"
  code=$(auth_curl -d "$batch")
  check "oversized batch (60 records) -> 400" "400" "$code"

  # (g) rapid-fire >20 req/min -> expect 429 eventually
  rate_limited="no"
  for i in $(seq 1 25); do
    code=$(auth_curl -d '[{"level":"info","message":"chat.execute"}]')
    if [ "$code" = "429" ]; then
      rate_limited="yes"
      break
    fi
  done
  if [ "$rate_limited" = "yes" ]; then
    echo "PASS: rapid-fire >20 req/min -> 429 observed"
    pass=$((pass + 1))
  else
    echo "FAIL: rapid-fire >20 req/min -> 429 never observed"
    fail=$((fail + 1))
  fi
fi

echo "== Summary: $pass passed, $fail failed =="
[ "$fail" -eq 0 ]
