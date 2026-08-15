#!/usr/bin/env bash
set -euo pipefail

: "${JAEGER_QUERY_URL:?set JAEGER_QUERY_URL (for example http://localhost:16686)}"
: "${NUXT_OTEL_JAEGER_ENDPOINT:?set NUXT_OTEL_JAEGER_ENDPOINT (for example http://localhost:4317)}"
root=$(cd "$(dirname "$0")/.." && pwd)
bin="$root/target/release/ai-tools"
port=18787
trace_id=$(printf '%032x' "$$")
tmp=$(mktemp -d)
pid=''
cleanup() { if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi; rm -rf "$tmp"; }
trap cleanup EXIT

cargo build --quiet --release --bin ai-tools --manifest-path "$root/packages/rust-tools/cli/Cargo.toml"
eval "$(PHASE4_PRINT_ENV=1 PHASE4_TRACE_ID="$trace_id" NUXT_OTEL_JAEGER_ENDPOINT="$NUXT_OTEL_JAEGER_ENDPOINT" node "$root/scripts/verify-phase4-process-boundary.ts" | tail -n 4)"
export NUXT_OTEL_ENABLED NUXT_OTEL_JAEGER_ENDPOINT AI_TOOLS_TRACEPARENT
"$bin" relay --mode local --port "$port" --dir "$tmp" --execution-root "$tmp" --origin "http://localhost:$port" \
  >"$tmp/stdout" 2>"$tmp/stderr" &
pid=$!
for _ in $(seq 1 50); do curl -fsS -H 'Origin: http://localhost:18787' "http://127.0.0.1:$port/health" >/dev/null && break; sleep 0.1; done
curl -fsS -H 'Origin: http://localhost:18787' "http://127.0.0.1:$port/health" >/dev/null

headers=(-H 'Content-Type: application/json' -H 'Origin: http://localhost:18787' -H 'Mcp-Method: initialize' -H 'Mcp-Name: phase3-acceptance' -H 'MCP-Protocol-Version: 2026-07-28')
curl -sS "http://127.0.0.1:$port/mcp" "${headers[@]}" --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"phase3","version":"1"},"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' >/dev/null
headers=(-H 'Content-Type: application/json' -H 'Origin: http://localhost:18787' -H 'Mcp-Method: tools/list' -H 'Mcp-Name: phase3-acceptance' -H 'MCP-Protocol-Version: 2026-07-28')
curl -sS "http://127.0.0.1:$port/mcp" "${headers[@]}" --data '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' >/dev/null
headers=(-H 'Content-Type: application/json' -H 'Origin: http://localhost:18787' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: phase3-acceptance' -H 'MCP-Protocol-Version: 2026-07-28')
  curl -sS "http://127.0.0.1:$port/mcp" "${headers[@]}" --data '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"terminal_exec","arguments":{"command":"printf phase4-canary"}}}' >/dev/null

kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
pid=''
sleep 1
payload=$(curl -fsS --get "$JAEGER_QUERY_URL/api/traces" --data-urlencode "traceID=$trace_id" --data-urlencode 'limit=100')
grep -q "${trace_id: -8}" <<<"$payload"
grep -q 'ai_tools.command' <<<"$payload"
grep -q 'bbbbbbbbbbbbbbbb' <<<"$payload"
! grep -Eq 'h2::|hyper::|tonic::|reqwest::|/\.cargo/registry|/packages/rust-tools/|code\.file\.path|phase4-canary' <<<"$payload"
relay_payload=$(curl -fsS --get "$JAEGER_QUERY_URL/api/traces" --data-urlencode 'service=ai-code-relay' --data-urlencode 'limit=100')
grep -q 'relay.request' <<<"$relay_payload"
grep -q 'relay.mcp.initialize' <<<"$relay_payload"
grep -q 'relay.mcp.tools_list' <<<"$relay_payload"
echo 'PASS: Rust first-party spans exported; dependency noise, paths, and canary absent'
