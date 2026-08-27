import { discoverOAuthServerInfo, refreshAuthorization } from '@modelcontextprotocol/sdk/client/auth.js'
import type { OAuthClientInformationMixed, OAuthTokens } from '@modelcontextprotocol/sdk/shared/auth.js'
import { getMcpServerOAuthCredentials, updateMcpServerOAuthTokens } from '../database/mcp-servers'
import { decryptSecret, encryptSecret } from '../security/crypto'

export async function refreshStoredMcpOAuthAccessToken(
  userId: string,
  serverId: string,
  serverUrl: URL,
  fetchFn: typeof fetch
): Promise<string | undefined> {
  const credentials = await getMcpServerOAuthCredentials(userId, serverId)
  if (!credentials) return undefined

  const tokens = JSON.parse(decryptSecret(credentials.tokensEncrypted)) as OAuthTokens
  if (!tokens.refresh_token) return undefined

  const clientInformation = JSON.parse(decryptSecret(credentials.clientInformationEncrypted)) as OAuthClientInformationMixed
  const info = await discoverOAuthServerInfo(serverUrl, { fetchFn })
  const authorizationServerUrl = new URL(info.authorizationServerUrl)
  if (authorizationServerUrl.href !== credentials.authorizationServer) {
    throw new Error('Stored MCP OAuth authorization server changed')
  }
  if (!info.authorizationServerMetadata?.token_endpoint) {
    throw new Error('Stored MCP OAuth token endpoint is unavailable')
  }

  const refreshed = await refreshAuthorization(authorizationServerUrl, {
    metadata: info.authorizationServerMetadata,
    clientInformation,
    refreshToken: tokens.refresh_token,
    resource: new URL(credentials.resource),
    fetchFn
  })
  await updateMcpServerOAuthTokens(userId, serverId, encryptSecret(JSON.stringify(refreshed)))
  return refreshed.access_token
}
