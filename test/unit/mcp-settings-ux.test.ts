import { strict as assert } from 'node:assert'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(import.meta.dirname, '../..')
const read = (path: string) => readFileSync(resolve(root, path), 'utf8')

const settings = read('app/pages/settings.vue')
assert.doesNotMatch(settings, /Local Terminal/)
assert.match(settings, /to: '\/settings\/mcp'/)

const nuxt = read('nuxt.config.ts')
assert.match(nuxt, /'\/settings\/local-terminal': \{ redirect: '\/settings\/mcp' \}/)

const page = read('app/pages/settings/mcp.vue')
assert.match(page, /MCP connections/)
assert.match(page, /v-if="!hasConnections"/)
assert.match(page, /No MCP connections yet/)
assert.match(page, /v-if="servers\.length"/)
assert.match(page, /Remote servers/)
assert.match(page, /label="Add MCP"/)
assert.match(page, /Connect a remote MCP server/)
assert.match(page, /Unsupported legacy transport/)
assert.match(page, /Remove MCP connection/)
assert.match(page, /More actions for/)
assert.doesNotMatch(page, /useRelayAgent|127\.0\.0\.1|Local relay/)

const dialog = read('app/components/settings/SettingsMcpConnectionDialog.vue')
assert.match(dialog, /New MCP connection/)
assert.match(dialog, /Server URL/)
assert.match(dialog, /Authentication/)
assert.match(dialog, /Advanced OAuth settings/)
assert.match(dialog, /SettingsMcpOAuthDetails/)
assert.match(dialog, /SettingsMcpRiskAcknowledgement/)
assert.match(dialog, /label="Create"/)
assert.match(dialog, /discoverOAuth/)
assert.match(dialog, /Scan tools/)
assert.match(dialog, /scanFresh/)
assert.match(dialog, /connectionChanged/)
assert.doesNotMatch(dialog, /'stdio'|"stdio"|Command/)
assert.doesNotMatch(dialog, /useRelayAgent|SettingsLocalRelaySetup|localConfigured|localConnected/)
assert.match(dialog, /form="mcp-connection-form"/)

const riskAcknowledgement = read('app/components/settings/SettingsMcpRiskAcknowledgement.vue')
assert.match(riskAcknowledgement, /Custom MCP servers introduce risk/)
assert.match(riskAcknowledgement, /I understand and want to continue/)

const oauthDetails = read('app/components/settings/SettingsMcpOAuthDetails.vue')
assert.match(oauthDetails, /Dynamic Client Registration \(DCR\)/)
assert.match(oauthDetails, /Client Identifier Metadata Document \(CIMD\)/)
assert.match(oauthDetails, /OAuth endpoints/)
assert.match(oauthDetails, /OpenID Connect/)

const mcpComposable = read('app/composables/useMcpServers.ts')
assert.match(mcpComposable, /oauth-discovery/)
assert.match(mcpComposable, /oauth-start/)
assert.match(dialog, /startOAuth/)

assert.match(dialog, /authChoice\.value === 'auto' && discoveryKey\.value !== config\.url/)
assert.match(dialog, /oauthDiscovery\.value = await discoverOAuth\(config\.url\)/)
assert.match(dialog, /startOAuth\(config\)/)
assert.match(dialog, /navigateTo\(result\.authorizationUrl, \{ external: true \}\)/)

const oauthDiscovery = read('server/api/mcp-servers/oauth-discovery.post.ts')
const oauthStart = read('server/api/mcp-servers/oauth-start.post.ts')
const oauthCallback = read('server/api/mcp-servers/oauth/callback.get.ts')
const oauthFlow = read('server/infrastructure/mcp/oauth-start.ts')
const oauthComplete = read('server/infrastructure/mcp/oauth-complete.ts')
const mcpDatabase = read('server/infrastructure/database/mcp-servers.ts')
const mcpClient = read('server/infrastructure/mcp/client.ts')
assert.match(oauthDiscovery, /discoverOAuth/)
assert.match(oauthStart, /application\.mcp\.startOAuth\(session\.user\.id, body, callbackUrl\)/)
assert.match(oauthStart, /OAuth client registration rejected the AI Code callback URL/)
assert.match(oauthCallback, /application\.mcp\.completeOAuth/)
assert.doesNotMatch(oauthCallback, /requireUserSession/)
assert.match(oauthCallback, /\/settings\/mcp\?oauth=success/)
assert.match(oauthFlow, /codeVerifier/)
assert.match(oauthFlow, /hashMcpOAuthState/)
assert.match(oauthFlow, /encryptSecret\(codeVerifier\)/)
assert.match(oauthFlow, /const authorizationServerUrl = new URL\(info\.authorizationServerUrl\)/)
assert.match(oauthFlow, /authorizationServer: authorizationServerUrl\.href/)
assert.match(oauthFlow, /token_endpoint_auth_method: 'client_secret_basic'/)
assert.doesNotMatch(oauthFlow, /token_endpoint_auth_method: 'none'/)
assert.match(oauthComplete, /exchangeAuthorization/)
assert.match(oauthComplete, /const authorizationServerUrl = new URL\(info\.authorizationServerUrl\)/)
assert.doesNotMatch(oauthComplete, /info\.authorizationServerUrl\.href/)
assert.match(oauthComplete, /createOAuthPendingMcpServer/)
assert.match(oauthComplete, /testMcpServer/)
assert.match(mcpDatabase, /oauthTokensEncrypted/)
assert.match(mcpClient, /StoredMcpOAuthProvider/)
assert.match(mcpClient, /updateMcpServerOAuthTokens/)
assert.match(mcpClient, /canTryModernOAuthHttp/)
assert.match(mcpClient, /refreshStoredMcpOAuthAccessToken/)
assert.match(mcpClient, /modernStatus !== 401 && modernStatus !== 403/)
assert.match(mcpClient, /new ModernHttpMcpClient/)
assert.match(mcpClient, /tokens\?\.access_token/)
assert.doesNotMatch(mcpClient, /code_400/)

const mcpManagement = read('server/infrastructure/mcp/server-management.ts')
assert.match(mcpManagement, /discoverOAuthServerInfo/)
assert.match(mcpManagement, /createSsrfSafeFetch/)
assert.match(mcpManagement, /client_id_metadata_document_supported/)

const newChat = read('app/pages/chat/index.vue')
const controls = read('app/components/chat/ChatConfigControls.vue')
assert.doesNotMatch(newChat, /ChatToolPicker|enabled-tool-ids/)
assert.doesNotMatch(controls, /ChatToolPicker|remoteToolsEnabled|useMcpServers/)
assert.match(controls, /agentAvailable: boolean/)
assert.match(controls, /mode === 'agent' && agentAvailable/)

const conversationChat = read('app/composables/useConversationChat.ts')
assert.match(conversationChat, /createConversationTransport\(\)/)
assert.doesNotMatch(conversationChat, /useRelayAgent|local_terminal|startSession|preAgentStop|createLocalToolController/)

const nativeTools = read('shared/utils/native-tools.ts')
assert.match(nativeTools, /NATIVE_LOCAL_TERMINAL_TOOL_ID = 'native\.local_terminal'/)
assert.match(nativeTools, /legacyNativeToolIds/)
assert.doesNotMatch(nativeTools, /pickerVisible|toolName: 'local_terminal'/)

const executeTurn = read('server/application/chat/execute-chat-turn.ts')
assert.match(executeTurn, /const agentTurn = conv\.mode === 'agent' && mcpExecution\.terminalAvailable/)
assert.match(executeTurn, /const readOnlyToolTurn = conv\.mode === 'chat' && mcpExecution\.terminalAvailable/)
assert.match(executeTurn, /deps\.buildMcpTools/)
assert.match(executeTurn, /\['workspace_read', 'git_read'\]/)
assert.doesNotMatch(executeTurn, /createLocalTerminalPolicy|local_terminal|deps\.localTerminal/)

const dependencies = read('server/application/chat/contracts.ts')
assert.doesNotMatch(dependencies, /LocalTerminalPort|localTerminal/)

for (const removed of [
  'app/composables/useRelayAgent.ts',
  'app/composables/chat/local-tool-controller.ts',
  'app/components/settings/SettingsLocalRelaySetup.vue',
  'shared/utils/local-relay.ts',
  'server/application/chat/local-terminal-policy.ts',
  'server/infrastructure/ai/local-terminal-tool.ts'
]) assert.equal(existsSync(resolve(root, removed)), false, `${removed} should be removed`)

console.log('unified MCP settings UX contract: pass')
