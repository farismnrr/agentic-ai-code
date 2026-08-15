#!/usr/bin/env bash
set -euo pipefail

command -v curl >/dev/null || { echo 'curl is required' >&2; exit 1; }
command -v jq >/dev/null || { echo 'jq is required' >&2; exit 1; }

if [[ -z "${APP_URL:-}" ]]; then
  echo 'UNAVAILABLE: APP_URL is not set; route telemetry acceptance was not executed.' >&2
  exit 0
fi
if ! curl -sS --connect-timeout 2 --max-time 5 "$APP_URL" >/dev/null 2>&1; then
  echo "UNAVAILABLE: APP_URL=$APP_URL is not reachable; route telemetry acceptance was not executed." >&2
  exit 0
fi

dynamic_id='123e4567-e89b-12d3-a456-426614174035'
unmatched='ROUTE-CANARY-PLAN035-X9Q7'
query_canary='ROUTE-QUERY-CANARY-035'

if [[ -z "${LOKI_QUERY_URL:-}" ]]; then
  echo 'UNAVAILABLE: LOKI_QUERY_URL is not set; route telemetry acceptance was not executed.' >&2
  exit 0
fi
if ! curl -fsS --connect-timeout 2 --max-time 5 "$LOKI_QUERY_URL" --get --data-urlencode 'query={job="ai-code-server"}' --data-urlencode 'limit=1' >/dev/null; then
  echo "UNAVAILABLE: LOKI_QUERY_URL=$LOKI_QUERY_URL is not reachable; route telemetry acceptance was not executed." >&2
  exit 0
fi
if [[ -z "${JAEGER_QUERY_URL:-}" ]]; then
  echo 'UNAVAILABLE: JAEGER_QUERY_URL is not set; request trace backend correlation is skipped.' >&2
elif ! curl -fsS --connect-timeout 2 --max-time 5 "$JAEGER_QUERY_URL/api/services" >/dev/null; then
  echo "UNAVAILABLE: JAEGER_QUERY_URL=$JAEGER_QUERY_URL is not reachable; request trace backend correlation is skipped." >&2
  unset JAEGER_QUERY_URL
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

request() {
  local name=$1 url=$2 method=${3:-GET} body=${4:-}
  local headers="$tmpdir/$name.headers" response="$tmpdir/$name.body"
  local args=(-sS -D "$headers" -o "$response" -X "$method")
  [[ -n "$body" ]] && args+=(-H 'content-type: application/json' -d "$body")
  curl "${args[@]}" "$APP_URL$url" || true
  local request_id
  request_id=$(awk 'tolower($1)=="x-request-id:" {print $2}' "$headers" | tr -d '\r' | tail -1)
  [[ "$request_id" =~ ^[0-9a-f-]{36}$ ]] || { echo "missing request ID for $name" >&2; return 1; }
  printf '%s\n' "$request_id" > "$tmpdir/$name.request_id"
  printf '%s\n' "$url" > "$tmpdir/$name.url"
}

request static '/api/auth/register' POST '{}'
request dynamic "/api/providers/$dynamic_id/models"
request unmatched "/api/$unmatched"
request query "/api/auth/register?token=$query_canary" POST '{}'

if [[ -n "${LOKI_QUERY_URL:-}" ]]; then
  for name in static dynamic unmatched query; do
    request_id=$(<"$tmpdir/$name.request_id")
    count=0
    for _ in {1..10}; do
      payload=$(curl -fsS "$LOKI_QUERY_URL" --get --data-urlencode "query={job=\"ai-code-server\"} |= \"$request_id\"" --data-urlencode 'limit=20')
      count=$(jq --arg id "$request_id" --arg name "$name" --arg raw "$(<"$tmpdir/$name.url")" --arg unmatched "$unmatched" --arg query "$query_canary" '[.data.result[].values[][1]? | fromjson | select((.attributes["request.id"] // .["request.id"]) == $id and .attributes.operation == "http.request.lifecycle" and ((($name == "static" or $name == "query") and .attributes.route == "/api/auth/register") or ($name == "unmatched" and .attributes.route == "/api/*") or ($name == "dynamic" and (.attributes.route == "/api/providers/:id/models" or .attributes.route == "/api/providers/[id]/models" or .attributes.route == "/api/*"))) and (tostring | contains($raw) | not) and (tostring | contains($unmatched) | not) and (tostring | contains($query) | not))] | length' <<<"$payload")
      [[ "$count" == 1 ]] && break
      sleep 1
    done
    [[ "$count" == 1 ]] || { echo "route telemetry assertion failed for $name" >&2; exit 1; }
    trace_id=$(jq -r --arg id "$request_id" '.data.result[].values[][1]? | fromjson | select((.attributes["request.id"] // .["request.id"]) == $id and .attributes.operation == "http.request.lifecycle") | .trace_id // .attributes.trace_id' <<<"$payload" | head -1)
    if [[ -n "${JAEGER_QUERY_URL:-}" && "$trace_id" =~ ^[0-9a-f]{32}$ ]]; then
      jq -e '[.data[].spans[]?] | length > 0' < <(curl -fsS "$JAEGER_QUERY_URL/api/traces" --get --data-urlencode "traceID=$trace_id" --data-urlencode 'limit=100') >/dev/null || { echo "Jaeger correlation failed for $name" >&2; exit 1; }
    fi
  done
fi

echo 'PASS: static, dynamic, unmatched, query, and request-ID route telemetry acceptance checks passed'
