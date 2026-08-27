#!/usr/bin/env bash
set -euo pipefail

# Generate a locally-managed Cloudflare Tunnel config for the Plan 036
# reference deployment profile. The generated config always connects the
# public hostname to IPv4 loopback so the Rust relay can restrict trusted
# X-Forwarded-Proto assertions to 127.0.0.1/32.
#
# Required environment:
#   CLOUDFLARED_TUNNEL_ID=<uuid>
#   CLOUDFLARED_CREDENTIALS_FILE=/home/<user>/.cloudflared/<uuid>.json
#
# Optional:
#   REMOTE_MCP_URL=https://mcp.farismunir.my.id/mcp
#   RELAY_AGENT_PORT=47821
#
# Usage:
#   ops/remote-mcp/cloudflared-config.sh > ~/.cloudflared/config.yml
#   cloudflared tunnel ingress validate
#   cloudflared tunnel run "$CLOUDFLARED_TUNNEL_ID"

: "${CLOUDFLARED_TUNNEL_ID:?set CLOUDFLARED_TUNNEL_ID}"
: "${CLOUDFLARED_CREDENTIALS_FILE:?set CLOUDFLARED_CREDENTIALS_FILE}"

command -v node >/dev/null

remote_mcp_url="${REMOTE_MCP_URL:-https://mcp.farismunir.my.id/mcp}"
relay_port="${RELAY_AGENT_PORT:-47821}"

if [[ ! -f "$CLOUDFLARED_CREDENTIALS_FILE" ]]; then
  echo 'CLOUDFLARED_CREDENTIALS_FILE must point to an existing tunnel credentials file' >&2
  exit 1
fi

node - \
  "$remote_mcp_url" \
  "$CLOUDFLARED_TUNNEL_ID" \
  "$CLOUDFLARED_CREDENTIALS_FILE" \
  "$relay_port" <<'NODE'
const [resourceRaw, tunnelId, credentialsFile, portRaw] = process.argv.slice(2)
const resource = new URL(resourceRaw)
if (resource.protocol !== 'https:') throw new Error('REMOTE_MCP_URL must use HTTPS')
if (resource.username || resource.password || resource.search || resource.hash) {
  throw new Error('REMOTE_MCP_URL must not contain credentials, query, or fragment')
}
if (resource.pathname !== '/mcp') throw new Error('REMOTE_MCP_URL must use /mcp')
if (resource.port) throw new Error('reference Cloudflare profile requires the default HTTPS port')
if (resourceRaw !== resource.href) {
  throw new Error(`REMOTE_MCP_URL must be canonical; use ${resource.href}`)
}

const port = Number(portRaw)
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error('RELAY_AGENT_PORT must be an integer from 1 to 65535')
}

// JSON string literals are valid YAML double-quoted scalars and safely escape
// unusual paths/identifiers without attempting a shell/YAML escaping scheme.
console.log(`tunnel: ${JSON.stringify(tunnelId)}`)
console.log(`credentials-file: ${JSON.stringify(credentialsFile)}`)
console.log('')
console.log('ingress:')
console.log(`  - hostname: ${JSON.stringify(resource.hostname)}`)
console.log(`    service: ${JSON.stringify(`http://127.0.0.1:${port}`)}`)
console.log('  - service: "http_status:404"')
NODE
