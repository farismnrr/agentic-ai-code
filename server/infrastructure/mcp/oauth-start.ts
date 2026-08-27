import crypto from 'node:crypto'
import { discoverOAuthServerInfo, registerClient, startAuthorization } from '@modelcontextprotocol/sdk/client/auth.js'
import { McpOAuthStartError } from '#server/application/mcp'
import { assertSafeUrl, createSsrfSafeFetch } from '../security/ssrf-guard'

function stringScopes(value: unknown) {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is string => typeof item === 'string' && item.length > 0 && item.length <= 256).slice(0, 64)
}

/**
 * Starts an interactive OAuth authorization flow for one MCP resource.
 *
 * This phase intentionally stops at the authorization redirect. Token exchange
 * and callback persistence are a later phase; no OAuth credential is stored here.
 */
export async function startMcpOAuthAuthorization(serverUrlValue: string, redirectUrlValue: string) {
  const serverUrl = new URL(serverUrlValue)
  await assertSafeUrl(serverUrl, 'MCP OAuth authorization')

  const redirectUrl = new URL(redirectUrlValue)
  if (!['http:', 'https:'].includes(redirectUrl.protocol)) {
    throw new McpOAuthStartError('OAuth callback URL must use HTTP or HTTPS')
  }

  const fetchFn = createSsrfSafeFetch('MCP OAuth authorization')
  const info = await discoverOAuthServerInfo(serverUrl, { fetchFn })
  const metadata = info.authorizationServerMetadata
  if (!metadata?.authorization_endpoint || !metadata.token_endpoint) {
    throw new McpOAuthStartError('MCP authorization server metadata is incomplete')
  }
  if (!metadata.registration_endpoint) {
    throw new McpOAuthStartError('MCP authorization server does not advertise Dynamic Client Registration')
  }

  const scopes = stringScopes(info.resourceMetadata?.scopes_supported ?? metadata.scopes_supported)
  const scope = scopes.length > 0 ? scopes.join(' ') : undefined
  const clientInformation = await registerClient(info.authorizationServerUrl, {
    metadata,
    scope,
    fetchFn,
    clientMetadata: {
      client_name: 'AI Code',
      redirect_uris: [redirectUrl.href],
      grant_types: ['authorization_code', 'refresh_token'],
      response_types: ['code'],
      token_endpoint_auth_method: 'none'
    }
  })

  const resourceValue = info.resourceMetadata?.resource
  const resource = typeof resourceValue === 'string' ? new URL(resourceValue) : serverUrl
  const { authorizationUrl } = await startAuthorization(info.authorizationServerUrl, {
    metadata,
    clientInformation,
    redirectUrl,
    scope,
    state: crypto.randomUUID(),
    resource
  })

  return { authorizationUrl: authorizationUrl.href }
}
