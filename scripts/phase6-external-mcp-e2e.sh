#!/usr/bin/env bash
set -euo pipefail

# external MCP client/public-MCP acceptance harness. Static checks always run; live
# deployment evidence is opt-in because this repository cannot drive a user's
# external MCP client workspace or provision the external tunnel/Authorization Server.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

required=(
  "$root/packages/rust-tools/interfaces/src/mcp.rs"
  "$root/packages/rust-tools/infrastructure/src/transport.rs"
  "$root/packages/rust-tools/application/src/execution.rs"
  "$root/packages/rust-tools/infrastructure/src/security.rs"
  "$root/packages/rust-tools/infrastructure/src/auth.rs"
)
for file in "${required[@]}"; do test -f "$file"; done

transport="$root/packages/rust-tools/infrastructure/src/transport.rs"
protocol="$root/packages/rust-tools/interfaces/src/mcp.rs"

# Keep the published endpoint on the stateless 2026 transport. Legacy
# compatibility may still exist in code, but the canonical protocol constant
# and modern route must stay present.
rg -q 'PROTOCOL_VERSION: &str = "2026-07-28"' "$protocol"
rg -q 'route\("/mcp", post\(handle_mcp\)\)' "$transport"
rg -q 'oauth-protected-resource' "$transport"

if [[ -n "${PHASE6_MCP_URL:-}" ]]; then
  command -v curl >/dev/null
  command -v node >/dev/null

  metadata_url="$({ node - "$PHASE6_MCP_URL" <<'NODE'
const endpoint = new URL(process.argv[2])
if (endpoint.protocol !== 'https:') throw new Error('PHASE6_MCP_URL must use HTTPS')
const resourcePath = endpoint.pathname.replace(/^\/+|\/+$/g, '')
const metadataPath = resourcePath
  ? `/.well-known/oauth-protected-resource/${resourcePath}`
  : '/.well-known/oauth-protected-resource'
console.log(new URL(metadataPath, endpoint.origin).href)
NODE
  } 2>/dev/null)"

  metadata="$(curl --fail --silent --show-error \
    -H 'Accept: application/json' \
    "$metadata_url")"

  # Metadata must identify the same public resource and advertise at least one
  # Authorization Server. Do not print the document: deployments may consider
  # issuer topology operational metadata.
  node - "$metadata" "$PHASE6_MCP_URL" <<'NODE'
const metadata = JSON.parse(process.argv[2])
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
NODE

  echo 'PUBLIC_MCP_METADATA=pass' >&2
else
  echo 'PUBLIC_MCP_METADATA=unavailable (set PHASE6_MCP_URL to probe a deployment)' >&2
fi

# This is intentionally not called "external MCP client E2E pass". A successful metadata
# probe is necessary deployment evidence, but the actual external MCP client OAuth + tool
# flow still has to be exercised from external MCP client developer mode.
echo 'phase6 static acceptance: pass'
