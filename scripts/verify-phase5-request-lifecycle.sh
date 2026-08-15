#!/usr/bin/env bash
set -euo pipefail

: "${APP_URL:?set APP_URL to the running Nitro server}"
: "${LOKI_QUERY_URL:?set LOKI_QUERY_URL to the Loki query endpoint, for example http://localhost:3100/loki/api/v1/query_range}"
: "${JAEGER_QUERY_URL:?set JAEGER_QUERY_URL, for example http://localhost:16686}"
: "${PHASE5_SUCCESS_URL:?set the success journey URL}"
: "${PHASE5_4XX_URL:?set the 4xx journey URL}"
: "${PHASE5_5XX_URL:?set the handled 5xx journey URL}"
: "${PHASE5_UNHANDLED_URL:?set the raw/unhandled failure journey URL}"

command -v jq >/dev/null || { echo 'jq is required for the Loki/Jaeger correlation proof' >&2; exit 1; }
curl -fsS "$LOKI_QUERY_URL" --get --data-urlencode 'query={job="ai-code-server"}' --data-urlencode 'limit=1' >/dev/null
curl -fsS "$JAEGER_QUERY_URL/api/services" >/dev/null

for journey in success 4XX handled-5XX unhandled-failure; do
  case "$journey" in
    success) url_var=PHASE5_SUCCESS_URL; app_var=PHASE5_SUCCESS_APP_URL; method_var=PHASE5_SUCCESS_METHOD; body_var=PHASE5_SUCCESS_BODY; cookie_var=PHASE5_SUCCESS_COOKIE ;;
    '4XX') url_var=PHASE5_4XX_URL; app_var=PHASE5_4XX_APP_URL; method_var=PHASE5_4XX_METHOD; body_var=PHASE5_4XX_BODY; cookie_var=PHASE5_4XX_COOKIE ;;
    'handled-5XX') url_var=PHASE5_5XX_URL; app_var=PHASE5_5XX_APP_URL; method_var=PHASE5_5XX_METHOD; body_var=PHASE5_5XX_BODY; cookie_var=PHASE5_5XX_COOKIE ;;
    'unhandled-failure') url_var=PHASE5_UNHANDLED_URL; app_var=PHASE5_UNHANDLED_APP_URL; method_var=PHASE5_UNHANDLED_METHOD; body_var=PHASE5_UNHANDLED_BODY; cookie_var=PHASE5_UNHANDLED_COOKIE ;;
  esac
  url="${!url_var}"
  # Optional per-journey overrides: a different app instance (e.g. an
  # isolated second instance with a broken dependency for the unhandled
  # journey), HTTP method/body (e.g. a POST that needs a body to reach a
  # real 4xx/5xx code path), and a cookie jar (for journeys that require an
  # authenticated session, e.g. the handled-5xx provider-reachability path).
  # All default to the plain unauthenticated GET against $APP_URL so the
  # common case stays a one-liner.
  journey_app_url="${!app_var:-$APP_URL}"
  method="${!method_var:-GET}"
  body="${!body_var:-}"
  cookie="${!cookie_var:-}"
  response_headers=$(mktemp)
  response_body=$(mktemp)
  trap 'rm -f "$response_headers" "$response_body"' EXIT
  curl_args=(-sS -D "$response_headers" -o "$response_body" -X "$method")
  [[ -n "$body" ]] && curl_args+=(-H 'Content-Type: application/json' -d "$body")
  [[ -n "$cookie" ]] && curl_args+=(-b "$cookie")
  curl "${curl_args[@]}" "$journey_app_url$url" || true
  request_id=$(awk 'tolower($1)=="x-request-id:" {print $2}' "$response_headers" | tr -d '\r' | tail -1)
  [[ "$request_id" =~ ^[0-9a-f-]{36}$ ]] || { echo "missing x-request-id for $journey" >&2; exit 1; }

  loki_query='{job="ai-code-server"} |= "'"$request_id"'"'
  # The server's BatchLogRecordProcessor (server/plugins/otel.server.ts)
  # exports on a periodic (default ~5s) schedule, not synchronously per
  # request — poll for a bounded window instead of querying once
  # immediately, or this races the export and false-negatives.
  count=0
  for _attempt in 1 2 3 4 5 6 7 8 9 10; do
    loki=$(curl -fsS "$LOKI_QUERY_URL" --get --data-urlencode "query=$loki_query" --data-urlencode 'limit=20')
    # Loki stream values are [nanosecond-timestamp, line] pairs — the line
    # (index 1) is the JSON string to decode, not the whole pair.
    count=$(jq --arg request_id "$request_id" '[.data.result[].values[][1]? | fromjson | select((.attributes["request.id"] // .["request.id"]) == $request_id and .attributes.operation == "http.request.lifecycle")] | length' <<<"$loki")
    [[ "$count" == 1 ]] && break
    sleep 1
  done
  if [[ "$count" != 1 ]]; then
    echo "expected one lifecycle record for $journey [$request_id], got $count" >&2
    exit 1
  fi
  trace_id=$(jq -r --arg request_id "$request_id" '.data.result[].values[][1]? | fromjson | select((.attributes["request.id"] // .["request.id"]) == $request_id and .attributes.operation == "http.request.lifecycle") | .trace_id // .attributes.trace_id' <<<"$loki" | head -1)
  if [[ ! "$trace_id" =~ ^[0-9a-f]{32}$ ]]; then
    echo "lifecycle record had no trace_id for $journey" >&2
    exit 1
  fi
  # Prove the trace_id correlates to a REAL trace in the backend (non-empty
  # span set). We deliberately do not require a `request.id` *span tag* here:
  # request.id is attached to the structured Loki log record (see
  # request-lifecycle.ts), not to every auto-instrumented HTTP span — only
  # code paths that explicitly call the RequestTelemetryContext's
  # `withSpan()` attach it to a span. Requiring a span-level request.id tag
  # on the plain root HTTP span would mean instrumenting every route with an
  # extra span solely for this proof, which is a new logging surface beyond
  # what Phase 5 calls for. trace_id equality between the Loki record and a
  # real, non-empty Jaeger trace is sufficient to prove requestId -> Loki ->
  # trace correlation per the acceptance goal.
  jaeger=$(curl -fsS "$JAEGER_QUERY_URL/api/traces" --get --data-urlencode "traceID=$trace_id" --data-urlencode 'limit=100')
  jq -e '[.data[].spans[]?] | length > 0' <<<"$jaeger" >/dev/null || {
    echo "trace_id $trace_id for $journey did not resolve to a real Jaeger trace" >&2
    exit 1
  }
  rm -f "$response_headers" "$response_body"
done

echo 'PASS: returned request IDs correlate one bounded lifecycle record through Loki to Jaeger for all four journeys'
