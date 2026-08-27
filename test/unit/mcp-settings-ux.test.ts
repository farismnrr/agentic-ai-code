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
assert.match(dialog, /Add remote MCP/)
assert.match(dialog, /Scan tools/)
assert.match(dialog, /Add MCP/)
assert.match(dialog, /scanFresh/)
assert.match(dialog, /connectionChanged/)
assert.doesNotMatch(dialog, /'stdio'|"stdio"|Command/)
assert.doesNotMatch(dialog, /useRelayAgent|SettingsLocalRelaySetup|localConfigured|localConnected/)
assert.match(dialog, /form="mcp-connection-form"/)

const picker = read('app/components/chat/ChatToolPicker.vue')
assert.match(picker, /Remote MCP tools/)
assert.match(picker, /to="\/settings\/mcp"/)
assert.doesNotMatch(picker, /useRelayAgent|NATIVE_LOCAL_TERMINAL_TOOL_ID|Local relay/)
assert.doesNotMatch(picker, /settings\/local-terminal/)

const controls = read('app/components/chat/ChatConfigControls.vue')
assert.match(controls, /remoteToolsEnabled/)
assert.match(controls, /server\.status === 'connected'/)
assert.doesNotMatch(controls, /useRelayAgent|NATIVE_LOCAL_TERMINAL_TOOL_ID/)

const conversationChat = read('app/composables/useConversationChat.ts')
assert.match(conversationChat, /createConversationTransport\(\)/)
assert.doesNotMatch(conversationChat, /useRelayAgent|local_terminal|startSession|preAgentStop|createLocalToolController/)

const nativeTools = read('shared/utils/native-tools.ts')
assert.match(nativeTools, /NATIVE_LOCAL_TERMINAL_TOOL_ID = 'native\.local_terminal'/)
assert.match(nativeTools, /legacyNativeToolIds/)
assert.doesNotMatch(nativeTools, /pickerVisible|toolName: 'local_terminal'/)

const executeTurn = read('server/application/chat/execute-chat-turn.ts')
assert.match(executeTurn, /const agentTurn = conv\.mode === 'agent' && enabledMcpToolIds\.length > 0/)
assert.match(executeTurn, /deps\.buildMcpTools/)
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
