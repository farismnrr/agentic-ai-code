import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { test } from 'node:test'
import { resolveNewChatModelId } from '../../app/utils/chat-model-selection.ts'

const root = resolve(import.meta.dirname, '../..')
const read = (path: string) => readFileSync(resolve(root, path), 'utf8')

test('new chat resolves the newest valid model in the active workspace', () => {
  const conversations = [
    { workspaceId: 'a', modelId: 'old', updatedAt: 10 },
    { workspaceId: 'a', modelId: 'new', updatedAt: 20 },
    { workspaceId: 'b', modelId: 'other', updatedAt: 30 }
  ]
  assert.equal(resolveNewChatModelId({ workspaceId: 'a', conversations, validModelIds: ['old', 'new', 'other'], defaultModelId: 'other' }), 'new')
})

test('new chat falls back to global default only when it is valid', () => {
  assert.equal(resolveNewChatModelId({ workspaceId: 'a', conversations: [], validModelIds: ['default'], defaultModelId: 'default' }), 'default')
  assert.equal(resolveNewChatModelId({ workspaceId: 'a', conversations: [], validModelIds: ['available'], defaultModelId: 'removed' }), undefined)
})

test('new chat has an explicit empty-model UX and no MCP tool picker', () => {
  const page = read('app/pages/chat/index.vue')
  const controls = read('app/components/chat/ChatConfigControls.vue')
  assert.match(page, /Choose a model to get started/)
  assert.match(page, /:disabled="!modelId"/)
  assert.match(controls, /placeholder="Choose a model"/)
  assert.doesNotMatch(page, /enabled-tool-ids|ChatToolPicker/)
  assert.doesNotMatch(controls, /enabledToolIds|ChatToolPicker|useMcpServers/)
})

test('Agent availability is terminal capability, not MCP tool volume', () => {
  const controls = read('app/components/chat/ChatConfigControls.vue')
  const newChat = read('app/pages/chat/index.vue')
  const existingChat = read('app/pages/chat/[id].vue')
  assert.match(controls, /agentAvailable: boolean/)
  assert.match(controls, /item\.value !== 'agent' \|\| props\.agentAvailable/)
  assert.match(newChat, /capabilities\.value\.terminal\.available/)
  assert.match(existingChat, /capabilities\.value\.terminal\.available/)
  assert.doesNotMatch(controls, /remoteToolsEnabled|server\.tools|availableTools/)
})

test('backend derives tools from Settings and fails stale Agent state closed', () => {
  const capabilities = read('server/infrastructure/mcp/capabilities.ts')
  const executeTurn = read('server/application/chat/execute-chat-turn.ts')
  assert.match(capabilities, /eq\(mcpServers\.enabled, true\)/)
  assert.match(capabilities, /eq\(mcpServers\.status, 'connected'\)/)
  assert.match(capabilities, /oauthTokensEncrypted/)
  assert.match(capabilities, /trustedProvenance === 'first-party-relay'/)
  assert.match(capabilities, /enabledToolIds/)
  assert.match(executeTurn, /const agentTurn = conv\.mode === 'agent' && mcpExecution\.terminalAvailable/)
  assert.match(executeTurn, /const readOnlyToolTurn = conv\.mode === 'chat' && mcpExecution\.terminalAvailable/)
  assert.match(executeTurn, /\['workspace_read', 'git_read'\]/)
  assert.match(executeTurn, /if \(!toolTurn\)/)
  assert.doesNotMatch(executeTurn, /conv\.enabledToolIds\.filter/)
})
