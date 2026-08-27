import crypto from 'node:crypto'
import { discoverOAuthServerInfo, registerClient, startAuthorization } from '@modelcontextprotocol/sdk/client/auth.js'
import { McpOAuthStartError } from '#server/application/mcp'
import type { McpRemoteConfig } from '#shared/types/chat'
import { createMcpOAuthFlow } from '../database/mcp-servers'
import { encryptSecret } from '../security/crypto'
import { assertSafeUrl, createSsrfSafeFetch } from '../security/ssrf-guard'
import { withInfrastructureSpan } from '../observability/span'

const FLOW_TTL_MS = 10 * 60 * 1000

function stringScopes(value: unknown) {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is string => typeof item === 'string' && item.length > 0 && item.length <= 256).slice(0, 64)
}

export function hashMcpOAuthState(state: string) {
  return crypto.createHash('sha256').update(state, 'utf8').digest('hex')
}

/** Starts one user-scoped OAuth authorization flow and persists its one-time state + PKCE material. */
export async function startMcpOAuthAuthorization(userId: string, config: McpRemoteConfig, redirectUrlValue: string) {
  const serverUrl = new URL(config.url)
  await assertSafeUrl(serverUrl, 'MCP OAuth authorization')

  const redirectUrl = new URL(redirectUrlValue)
  if (!['http:', 'https:'].includes(redirectUrl.protocol)) {
    throw new McpOAuthStartError('OAuth callback URL must use HTTP or HTTPS')
  }

  const state = crypto.randomBytes(32).toString('base64url')
  const stateHash = hashMcpOAuthState(state)
  const flowId = stateHash.slice(0, 16)
  const telemetry = { 'mcp.flow_id': flowId, 'mcp.transport': config.transport, 'mcp.oauth': true }

  const fetchFn = createSsrfSafeFetch('MCP OAuth authorization')
  const info = await withInfrastructureSpan(
    'mcp.oauth.external.discovery',
    { ...telemetry, 'mcp.stage': 'discovery', 'external.system': 'oauth_authorization_server' },
    () => discoverOAuthServerInfo(serverUrl, { fetchFn })
  )
  const authorizationServerUrl = new URL(info.authorizationServerUrl)
  const metadata = info.authorizationServerMetadata
  if (!metadata?.authorization_endpoint || !metadata.token_endpoint) {
    throw new McpOAuthStartError('MCP authorization server metadata is incomplete')
  }
  if (!metadata.registration_endpoint) {
    throw new McpOAuthStartError('MCP authorization server does not advertise Dynamic Client Registration')
  }

  const scopes = stringScopes(info.resourceMetadata?.scopes_supported ?? metadata.scopes_supported)
  const scope = scopes.length > 0 ? scopes.join(' ') : undefined
  const clientInformation = await withInfrastructureSpan(
    'mcp.oauth.external.client_registration',
    { ...telemetry, 'mcp.stage': 'client_registration', 'external.system': 'oauth_authorization_server' },
    () => registerClient(authorizationServerUrl, {
      metadata,
      scope,
      fetchFn,
      clientMetadata: {
        client_name: 'AI Code',
        redirect_uris: [redirectUrl.href],
        grant_types: ['authorization_code', 'refresh_token'],
        response_types: ['code'],
        token_endpoint_auth_method: 'client_secret_basic'
      }
    })
  )

  const resourceValue = info.resourceMetadata?.resource
  const resource = typeof resourceValue === 'string' ? new URL(resourceValue) : serverUrl
  const { authorizationUrl, codeVerifier } = await withInfrastructureSpan(
    'mcp.oauth.authorization_request',
    { ...telemetry, 'mcp.stage': 'authorization_request' },
    () => startAuthorization(authorizationServerUrl, {
      metadata,
      clientInformation,
      redirectUrl,
      scope,
      state,
      resource
    })
  )

  await withInfrastructureSpan(
    'mcp.oauth.db.persist_flow',
    { ...telemetry, 'mcp.stage': 'persist_flow', 'db.operation': 'insert_oauth_flow' },
    () => createMcpOAuthFlow({
      stateHash,
      userId,
      name: config.name,
      description: config.description,
      transport: config.transport,
      serverUrl: resource.href,
      redirectUri: redirectUrl.href,
      authorizationServer: authorizationServerUrl.href,
      resource: resource.href,
      clientInformationEncrypted: encryptSecret(JSON.stringify(clientInformation)),
      codeVerifierEncrypted: encryptSecret(codeVerifier),
      expiresAt: new Date(Date.now() + FLOW_TTL_MS)
    })
  )

  return { authorizationUrl: authorizationUrl.href }
}
