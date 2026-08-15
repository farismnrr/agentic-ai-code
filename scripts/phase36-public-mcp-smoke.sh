#!/usr/bin/env bash
set -euo pipefail

# Public Plan 036 smoke test. This validates the deployable MCP/OAuth edge
# without executing any tool. It proves:
#   1) the public HTTPS endpoint and protected-resource metadata are reachable;
#   2) an unauthenticated modern MCP request gets an OAuth Bearer challenge;
#   3) optionally, an operator-supplied token can complete server/discover and
#      tools/list through the public edge.
#
# Required:
#   REMOTE_MCP_URL=https://mcp.example.com/mcp
# Optional:
#   REMOTE_MCP_ACCESS_TOKEN_FILE=/path/to/0600-token-file
#
# The token is read from a file and written only to a mode-0600 temporary curl
# config so it never appears in curl's command-line arguments or script output.

: "${REMOTE_MCP_URL:?set REMOTE_MCP_URL to the public HTTPS MCP resource}"
command -v curl >/dev/null
command -v node >/dev/null

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
chmod 700 "$tmp"

readarray -t urls < <(node - "$REMOTE_MCP_URL" <<'NODE'
const endpoint = new URL(process.argv[2])
if (endpoint.protocol !== 'https:') throw new Error('REMOTE_MCP_URL must use HTTPS')
if (endpoint.username || endpoint.password || endpoint.search || endpoint.hash) {
  throw new Error('REMOTE_MCP_URL must not contain credentials, query, or fragment')
}
const resourcePath = endpoint.pathname.replace(/^\/+|\/+$/g, '')
const metadataPath = resourcePath
  ? `/.well-known/oauth-protected-resource/${resourcePath}`
  : '/.well-known/oauth-protected-resource'
console.log(endpoint.href)
console.log(new URL(metadataPath, endpoint.origin).href)
console.log(new URL('/health', endpoint.origin).href)
NODE
)

endpoint="${urls[0]}"
metadata_url="${urls[1]}"
health_url="${urls[2]}"

curl --fail --silent --show-error "$health_url" >/dev/null
metadata_file="$tmp/metadata.json"
curl --fail --silent --show-error \
  -H 'Accept: application/json' \
  "$metadata_url" >"$metadata_file"

node - "$metadata_file" "$endpoint" <<'NODE'
const fs = require('node:fs')
const metadata = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))
const expected = new URL(process.argv[3]).href
if (metadata.resource !== expected) {
  throw new Error(`protected resource mismatch: expected ${expected}`)
}
if (!Array.isArray(metadata.authorization_servers) || metadata.authorization_servers.length === 0) {
  throw new Error('protected resource metadata has no authorization_servers')
}
if (!Array.isArray(metadata.scopes_supported) || !metadata.scopes_supported.includes('relay.coding')) {
  throw new Error('protected resource metadata does not advertise relay.coding')
}
for (const issuer of metadata.authorization_servers) {
  if (new URL(issuer).protocol !== 'https:') {
    throw new Error('authorization server metadata must use HTTPS')
  }
}
NODE

discover_payload='{"jsonrpc":"2.0","id":"plan036-discover","method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}'
unauth_headers="$tmp/unauth.headers"
unauth_body="$tmp/unauth.json"
unauth_status="$(curl --silent --show-error \
  -D "$unauth_headers" \
  -o "$unauth_body" \
  -w '%{http_code}' \
  -X POST \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  --data "$discover_payload" \
  "$endpoint")"

if [[ "$unauth_status" != "401" ]]; then
  echo "expected unauthenticated server/discover to return 401, got $unauth_status" >&2
  exit 1
fi
if ! grep -iq '^www-authenticate:[[:space:]]*Bearer ' "$unauth_headers"; then
  echo 'OAuth Bearer challenge missing from unauthenticated MCP response' >&2
  exit 1
fi
if ! grep -iq 'resource_metadata=' "$unauth_headers"; then
  echo 'OAuth Bearer challenge does not advertise resource_metadata' >&2
  exit 1
fi

echo 'REMOTE_MCP_PUBLIC_EDGE=pass'
echo 'REMOTE_MCP_OAUTH_CHALLENGE=pass'

if [[ -z "${REMOTE_MCP_ACCESS_TOKEN_FILE:-}" ]]; then
  echo 'REMOTE_MCP_AUTHENTICATED=unavailable (set REMOTE_MCP_ACCESS_TOKEN_FILE for discover/tools-list proof)' >&2
  exit 0
fi

if [[ ! -f "$REMOTE_MCP_ACCESS_TOKEN_FILE" ]]; then
  echo 'REMOTE_MCP_ACCESS_TOKEN_FILE does not exist' >&2
  exit 1
fi

token="$(cat "$REMOTE_MCP_ACCESS_TOKEN_FILE")"
if [[ -z "$token" || "$token" == *$'\n'* || "$token" == *$'\r'* ]]; then
  echo 'access token file must contain exactly one non-empty token without embedded newlines' >&2
  exit 1
fi

curl_config="$tmp/auth.curlrc"
umask 077
printf 'header = "Authorization: Bearer %s"\n' "$token" >"$curl_config"
unset token

run_authenticated_rpc() {
  local method="$1"
  local payload="$2"
  local output="$3"
  shift 3

  local status
  status="$(curl --silent --show-error \
    --config "$curl_config" \
    -o "$output" \
    -w '%{http_code}' \
    -X POST \
    -H 'Content-Type: application/json' \
    -H 'Accept: application/json' \
    -H 'MCP-Protocol-Version: 2026-07-28' \
    -H "Mcp-Method: $method" \
    "$@" \
    --data "$payload" \
    "$endpoint")"

  if [[ "$status" != "200" ]]; then
    echo "authenticated $method returned HTTP $status" >&2
    exit 1
  fi
}

auth_discover="$tmp/discover.json"
run_authenticated_rpc 'server/discover' "$discover_payload" "$auth_discover"
node - "$auth_discover" <<'NODE'
const fs = require('node:fs')
const response = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))
if (!response.result || !Array.isArray(response.result.supportedVersions)) {
  throw new Error('server/discover response is missing supportedVersions')
}
if (!response.result.supportedVersions.includes('2026-07-28')) {
  throw new Error('server/discover does not advertise MCP 2026-07-28')
}
NODE

tools_payload='{"jsonrpc":"2.0","id":"plan036-tools","method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}'
auth_tools="$tmp/tools.json"
run_authenticated_rpc 'tools/list' "$tools_payload" "$auth_tools"
node - "$auth_tools" <<'NODE'
const fs = require('node:fs')
const response = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))
const tools = response.result?.tools
if (!Array.isArray(tools) || tools.length === 0) {
  throw new Error('tools/list returned no tools')
}
if (!tools.some(tool => tool.name === 'terminal_exec')) {
  throw new Error('tools/list does not expose terminal_exec')
}
NODE

echo 'REMOTE_MCP_AUTHENTICATED=pass'
echo 'REMOTE_MCP_DISCOVERY=pass'
echo 'REMOTE_MCP_TOOLS_LIST=pass'
