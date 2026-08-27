import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
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
assert.match(page, /v-else-if="!hasConnections"/)
assert.match(page, /No MCP connections yet/)
assert.match(page, /v-if="localConfigured"/)
assert.match(page, /This device/)
assert.match(page, /v-if="servers\.length"/)
assert.match(page, /Remote servers/)
assert.match(page, /name="Local relay"/)
assert.match(page, /label="Add MCP"/)
assert.match(page, /localMenuItems/)
assert.match(page, /Remove Local relay/)
assert.match(page, /Unsupported legacy transport/)
assert.match(page, /Remove MCP connection/)
assert.match(page, /More actions for/)
assert.doesNotMatch(page, /onMounted\([\s\S]*checkConnection/)

const dialog = read('app/components/settings/SettingsMcpConnectionDialog.vue')
assert.match(dialog, /Local relay/)
assert.match(dialog, /Remote MCP server/)
assert.match(dialog, /Scan tools/)
assert.match(dialog, /Add MCP/)
assert.match(dialog, /finishLocal/)
assert.match(dialog, /:disabled="localConfigured"/)
assert.match(dialog, /!localConfigured && !localConnected/)
assert.match(dialog, /scanFresh/)
assert.match(dialog, /connectionChanged/)
assert.doesNotMatch(dialog, /'stdio'|"stdio"|Command/)
assert.match(dialog, /focus-visible:ring-2/)

const picker = read('app/components/chat/ChatToolPicker.vue')
assert.match(picker, /isConfigured/)
assert.match(picker, /tool\.id !== NATIVE_LOCAL_TERMINAL_TOOL_ID \|\| isConfigured\.value/)
assert.match(picker, /label="MCP settings"/)
assert.match(picker, /to="\/settings\/mcp"/)
assert.doesNotMatch(picker, /settings\/local-terminal/)

const controls = read('app/components/chat/ChatConfigControls.vue')
assert.match(controls, /isConfigured\.value && enabledToolIds\.value\.includes/)

const conversationChat = read('app/composables/useConversationChat.ts')
assert.match(conversationChat, /relayAgent\.isConfigured\.value && conversation\.value\?\.mode === 'agent'/)
assert.match(conversationChat, /if \(!relayAgent\.isConfigured\.value\) return/)

const nativeTools = read('shared/utils/native-tools.ts')
assert.match(nativeTools, /NATIVE_LOCAL_TERMINAL_TOOL_ID = 'native\.local_terminal'/)
assert.match(nativeTools, /name: 'Local relay'/)

console.log('unified MCP settings UX contract: pass')
