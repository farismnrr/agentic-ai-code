import {
  approvalForCapability,
  capabilityFactsForToolCall,
  classifyCapability,
  rememberedApprovalCanAutoAnswer
} from '../shared/utils/capability-policy.ts'
import { mcpModelToolName, resolveMcpToolFromModelName } from '../shared/utils/mcp-tool-identity.ts'

function expect(condition: unknown, message: string) {
  if (!condition) throw new Error(`capability policy acceptance failed: ${message}`)
}

const trustedRead = capabilityFactsForToolCall({
  toolId: 'relay.file_read', toolName: 'file_read', input: { path: 'src/main.ts' }, trustedProvenance: 'first-party-relay'
})
expect(classifyCapability(trustedRead).risk === 'low', 'trusted read is low risk')
for (const mode of ['plan', 'workspace', 'autonomous', 'manual'] as const) {
  expect(approvalForCapability(trustedRead, 'always', mode).outcome === 'approved', `${mode} remembered read`)
}
expect(approvalForCapability(trustedRead, 'never', 'workspace').outcome === 'denied', 'never is fail-closed')

const write = capabilityFactsForToolCall({
  toolId: 'relay.file_write', toolName: 'file_write', input: { path: 'src/new.ts' }, trustedProvenance: 'first-party-relay'
})
expect(approvalForCapability(write, undefined, 'workspace').outcome === 'approved', 'contained workspace mutation')
expect(approvalForCapability(write, undefined, 'plan').outcome === 'denied', 'plan denies mutation')

const terminal = capabilityFactsForToolCall({
  toolId: 'native.local_terminal', toolName: 'local_terminal', input: { command: 'cat', args: ['src/main.ts'] }, trustedProvenance: 'native'
})
expect(terminal.effects.includes('process_exec') && terminal.effects.includes('network_read') && terminal.effects.includes('external_mutation'), 'local terminal effects')
expect(classifyCapability(terminal).risk === 'high', 'local terminal is high risk')
expect(approvalForCapability(terminal, 'always', 'manual').outcome === 'user-approval', 'remembered terminal always narrows')
expect(!rememberedApprovalCanAutoAnswer(terminal, 'always', 'manual'), 'narrowed always remains visible')
expect(rememberedApprovalCanAutoAnswer(terminal, 'never', 'manual'), 'remembered never can answer')

const opaque = capabilityFactsForToolCall({
  toolId: 'native.local_terminal', toolName: 'local_terminal', input: { command: 'sh', args: ['-lc', 'cat src/main.ts'] }, trustedProvenance: 'native'
})
expect(classifyCapability(opaque).opaque === true, 'shell command is opaque')

const externalSafeName = capabilityFactsForToolCall({
  toolId: 'external.read_file', toolName: 'read_file', input: { path: 'src/main.ts' }, trustedProvenance: 'external'
})
expect(classifyCapability(externalSafeName).risk === 'high', 'external safe-looking tool is high risk')
expect(approvalForCapability(externalSafeName, 'always', 'autonomous').outcome === 'user-approval', 'external tool never auto-approves')

const structured = (path: string) => approvalForCapability(capabilityFactsForToolCall({
  toolId: 'relay.file_write', toolName: 'file_write', input: { path }, trustedProvenance: 'first-party-relay'
}), undefined, 'workspace').outcome
expect(structured('src/new.ts') === 'approved', 'safe structured mutation')
expect(structured('.npmrc') === 'denied', 'protected structured mutation')

const network = capabilityFactsForToolCall({
  toolId: 'relay.http_fetch', toolName: 'http_fetch', input: { url: 'https://example.test/a' }, trustedProvenance: 'first-party-relay'
})
expect(network.domain === 'example.test' && network.networkRequested === true, 'structured domain fact')
expect(approvalForCapability(network, undefined, 'autonomous').outcome === 'user-approval', 'network asks')

const mcpCatalog = [
  { id: 'server-a.read_file', serverId: 'server-a', name: 'read_file', description: '', sampleInput: {} },
  { id: 'server-b.read_file', serverId: 'server-b', name: 'read_file', description: '', sampleInput: {} }
]
const modelKey = mcpModelToolName('server-a', 'read_file')
const resolvedMcp = resolveMcpToolFromModelName(modelKey, mcpCatalog)
expect(resolvedMcp?.id === 'server-a.read_file', 'model key resolves exact MCP identity')
expect(resolvedMcp?.id === 'server-a.read_file', 'remembered approval uses canonical MCP id')
expect(resolveMcpToolFromModelName('read_file', mcpCatalog) === undefined, 'raw MCP name does not resolve')
expect(resolveMcpToolFromModelName('unknown_tool', mcpCatalog) === undefined, 'unknown MCP identity fails safe')
const staleMcp = capabilityFactsForToolCall({
  toolId: 'server-a.read_file', toolName: 'read_file', input: { path: '.ssh/id' }, trustedProvenance: 'external'
})
expect(classifyCapability(staleMcp).risk === 'high', 'external protected MCP call is high risk')
expect(!rememberedApprovalCanAutoAnswer(staleMcp, 'always', 'autonomous'), 'stale broad approval cannot suppress narrowed MCP approval')

console.log('capability policy behavioral acceptance: PASS')
