import { exchangeAuthorization, discoverOAuthServerInfo } from '@modelcontextprotocol/sdk/client/auth.js'
import type { OAuthClientInformationMixed, OAuthTokens } from '@modelcontextprotocol/sdk/shared/auth.js'
import { McpOAuthCallbackError } from '#server/application/mcp'
import type { McpRemoteConfig, McpRemoteTransport } from '#shared/types/chat'
import { consumeMcpOAuthFlow, createOAuthPendingMcpServer, deleteMcpServer, mcpServerIdFor } from '../database/mcp-servers'
import { decryptSecret, encryptSecret } from '../security/crypto'
import { assertSafeUrl, createSsrfSafeFetch } from '../security/ssrf-guard'
import { withInfrastructureSpan } from '../observability/span'
import { testMcpServer } from './server-management'
import { hashMcpOAuthState } from './oauth-start'

function parseJsonSecret<T>(encrypted: string, label: string): T {
  try {
    return JSON.parse(decryptSecret(encrypted)) as T
  } catch {
    throw new McpOAuthCallbackError(`Stored OAuth ${label} is invalid`)
  }
}

function remoteTransport(value: string): McpRemoteTransport {
  if (value === 'http' || value === 'sse') return value
  throw new McpOAuthCallbackError('Stored MCP transport is invalid')
}

export async function completeMcpOAuthAuthorization(state: string, authorizationCode: string) {
  const stateHash = hashMcpOAuthState(state)
  const flowId = stateHash.slice(0, 16)
  const baseTelemetry = { 'mcp.flow_id': flowId, 'mcp.oauth': true }
  const flow = await withInfrastructureSpan(
    'mcp.oauth.db.consume_flow',
    { ...baseTelemetry, 'mcp.stage': 'consume_flow', 'db.operation': 'consume_oauth_flow' },
    () => consumeMcpOAuthFlow(stateHash)
  )
  if (!flow) throw new McpOAuthCallbackError('OAuth state is invalid or already used')
  if (flow.expiresAt.getTime() <= Date.now()) throw new McpOAuthCallbackError('OAuth state has expired')
  const userId = flow.userId

  const serverUrl = new URL(flow.serverUrl)
  await assertSafeUrl(serverUrl, 'MCP OAuth callback')
  const telemetry = { ...baseTelemetry, 'mcp.transport': flow.transport }
  const fetchFn = createSsrfSafeFetch('MCP OAuth callback')
  const info = await withInfrastructureSpan(
    'mcp.oauth.external.callback_discovery',
    { ...telemetry, 'mcp.stage': 'callback_discovery', 'external.system': 'oauth_authorization_server' },
    () => discoverOAuthServerInfo(serverUrl, { fetchFn })
  )
  const authorizationServerUrl = new URL(info.authorizationServerUrl)
  if (authorizationServerUrl.href !== flow.authorizationServer) {
    throw new McpOAuthCallbackError('OAuth authorization server changed during sign-in')
  }
  if (!info.authorizationServerMetadata?.token_endpoint) {
    throw new McpOAuthCallbackError('OAuth token endpoint is unavailable')
  }

  const clientInformation = parseJsonSecret<OAuthClientInformationMixed>(flow.clientInformationEncrypted, 'client information')
  const codeVerifier = decryptSecret(flow.codeVerifierEncrypted)
  const resource = new URL(flow.resource)
  const tokens: OAuthTokens = await withInfrastructureSpan(
    'mcp.oauth.external.token_exchange',
    { ...telemetry, 'mcp.stage': 'token_exchange', 'external.system': 'oauth_authorization_server' },
    () => exchangeAuthorization(authorizationServerUrl, {
      metadata: info.authorizationServerMetadata,
      clientInformation,
      authorizationCode,
      codeVerifier,
      redirectUri: flow.redirectUri,
      resource,
      fetchFn
    })
  )

  const config: McpRemoteConfig = {
    name: flow.name,
    description: flow.description,
    transport: remoteTransport(flow.transport),
    url: flow.serverUrl
  }
  const id = mcpServerIdFor(userId, config.name)
  await withInfrastructureSpan(
    'mcp.oauth.db.persist_server',
    { ...telemetry, 'mcp.stage': 'persist_server', 'db.operation': 'insert_pending_mcp_server' },
    () => createOAuthPendingMcpServer(userId, {
      ...config,
      id,
      oauthAuthorizationServer: flow.authorizationServer,
      oauthResource: flow.resource,
      oauthRedirectUri: flow.redirectUri,
      oauthClientInformationEncrypted: encryptSecret(JSON.stringify(clientInformation)),
      oauthTokensEncrypted: encryptSecret(JSON.stringify(tokens))
    })
  )

  try {
    const verified = await withInfrastructureSpan(
      'mcp.oauth.external.verify_mcp',
      { ...telemetry, 'mcp.stage': 'verify_mcp', 'external.system': 'mcp_server' },
      () => testMcpServer(userId, id)
    )
    if (!verified) throw new McpOAuthCallbackError('OAuth MCP connection could not be verified')
    return { id }
  } catch (error) {
    try {
      await withInfrastructureSpan(
        'mcp.oauth.db.rollback_server',
        { ...telemetry, 'mcp.stage': 'rollback_server', 'db.operation': 'delete_mcp_server' },
        () => deleteMcpServer(userId, id)
      )
    } catch {
      // Rollback is best-effort; the original verification failure remains authoritative.
    }
    throw error
  }
}
