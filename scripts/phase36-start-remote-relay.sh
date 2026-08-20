#!/usr/bin/env bash
set -euo pipefail

# Start the laptop relay in the Plan 036 remote profile.
#
# This script deliberately keeps the Rust listener on 127.0.0.1 and trusts
# HTTPS forwarding only from an IPv4 loopback tunnel process on the same host.
# A public tunnel/edge (reference profile: cloudflared) must forward to
# http://127.0.0.1:${RELAY_AGENT_PORT:-47821}.
#
# Required environment:
#   REMOTE_MCP_URL=https://mcp.farismunir.my.id/mcp
#   OAUTH_ISSUER=https://<authorization-server-issuer>/
#   OAUTH_OWNER_SUBJECT=<stable owner sub claim>
#   EXECUTION_ROOT=/home/<user> (canonical non-root owner home)
#
# Optional:
#   RELAY_WORKING_DIR=$EXECUTION_ROOT
#   RELAY_AGENT_PORT=47821
#   AI_TOOLS_BIN=target/release/ai-tools
#   RELAY_AGENT_ORIGIN=https://<hosted-nuxt-origin>
#
# `RELAY_AGENT_ORIGIN` only affects browser CORS convenience. Remote MCP
# authorization remains the OAuth Resource Server policy in the Rust relay.

: "${REMOTE_MCP_URL:?set REMOTE_MCP_URL to the canonical public MCP resource}"
: "${OAUTH_ISSUER:?set OAUTH_ISSUER to the Authorization Server issuer}"
: "${OAUTH_OWNER_SUBJECT:?set OAUTH_OWNER_SUBJECT to the allowed owner subject}"
EXECUTION_ROOT="${EXECUTION_ROOT:-${HOME:?HOME must be set for the owner-home relay scope}}"

command -v node >/dev/null

relay_port="${RELAY_AGENT_PORT:-47821}"
working_dir="${RELAY_WORKING_DIR:-$EXECUTION_ROOT}"
ai_tools_bin="${AI_TOOLS_BIN:-target/release/ai-tools}"

node - "$REMOTE_MCP_URL" "$OAUTH_ISSUER" "$relay_port" <<'NODE'
const [resourceRaw, issuerRaw, portRaw] = process.argv.slice(2)
const resource = new URL(resourceRaw)
if (resource.protocol !== 'https:') throw new Error('REMOTE_MCP_URL must use HTTPS')
if (resource.username || resource.password || resource.search || resource.hash) {
  throw new Error('REMOTE_MCP_URL must not contain credentials, query, or fragment')
}
if (resource.pathname !== '/mcp') {
  throw new Error('REMOTE_MCP_URL must use the canonical /mcp path')
}
if (resourceRaw !== resource.href) {
  throw new Error(`REMOTE_MCP_URL must be canonical; use ${resource.href}`)
}

const issuer = new URL(issuerRaw)
if (issuer.protocol !== 'https:') throw new Error('OAUTH_ISSUER must use HTTPS')
if (issuer.username || issuer.password || issuer.search || issuer.hash) {
  throw new Error('OAUTH_ISSUER must not contain credentials, query, or fragment')
}
if (issuerRaw !== issuer.href) {
  throw new Error(`OAUTH_ISSUER must be canonical; use ${issuer.href}`)
}

const port = Number(portRaw)
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error('RELAY_AGENT_PORT must be an integer from 1 to 65535')
}
NODE

if [[ ! -d "$EXECUTION_ROOT" ]]; then
  echo 'EXECUTION_ROOT must be an existing directory' >&2
  exit 1
fi
if [[ ! -d "$working_dir" ]]; then
  echo 'RELAY_WORKING_DIR must be an existing directory' >&2
  exit 1
fi

if [[ "$ai_tools_bin" == */* ]]; then
  if [[ ! -x "$ai_tools_bin" ]]; then
    echo "AI_TOOLS_BIN is not executable: $ai_tools_bin" >&2
    exit 1
  fi
else
  command -v "$ai_tools_bin" >/dev/null
fi

# This repository wrapper is the external MCP client-facing remote deployment path. Pin
# it to Primary explicitly, even if a caller inherited Full, while keeping
# the Rust CLI's Full default intact for direct/local and other deployments.
export RELAY_TOOL_PROFILE=primary

exec "$ai_tools_bin" relay \
  --mode remote \
  --trusted-proxy \
  --trusted-proxy-cidr '127.0.0.1/32' \
  --port "$relay_port" \
  --dir "$working_dir" \
  --execution-root "$EXECUTION_ROOT" \
  --oauth-issuer "$OAUTH_ISSUER" \
  --oauth-audience "$REMOTE_MCP_URL" \
  --oauth-owner-subject "$OAUTH_OWNER_SUBJECT"
