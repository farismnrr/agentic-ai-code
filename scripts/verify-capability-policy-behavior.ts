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
for (const mode of ['plan', 'bypass', 'manual'] as const) {
  expect(approvalForCapability(trustedRead, 'always', mode).outcome === 'approved', `${mode} remembered read`)
}
expect(approvalForCapability(trustedRead, 'never', 'manual').outcome === 'denied', 'never is fail-closed')

const write = capabilityFactsForToolCall({
  toolId: 'relay.file_write', toolName: 'file_write', input: { path: 'src/new.ts', content: 'new' }, trustedProvenance: 'first-party-relay'
})
expect(approvalForCapability(write, undefined, 'manual').outcome === 'user-approval', 'manual asks for workspace mutation')
expect(approvalForCapability(write, undefined, 'bypass').outcome === 'approved', 'bypass skips workspace mutation prompt')
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
expect(approvalForCapability(externalSafeName, 'always', 'manual').outcome === 'user-approval', 'manual external tool still asks')
expect(approvalForCapability(externalSafeName, 'never', 'bypass').outcome === 'approved', 'bypass mode overrides remembered prompt answers')

const structured = (path: string) => approvalForCapability(capabilityFactsForToolCall({
  toolId: 'relay.file_write', toolName: 'file_write', input: { path, content: 'new' }, trustedProvenance: 'first-party-relay'
}), undefined, 'bypass').outcome
expect(structured('src/new.ts') === 'approved', 'safe structured mutation')
expect(structured('.npmrc') === 'denied', 'protected structured mutation')

const protectedPaths = [
  '.env', '.env.local', '.env.production', 'nested/.env.production',
  '.npmrc', '.netrc', '.pypirc', '.git-credentials',
  '.cargo/credentials', '.cargo/credentials.toml', '.ssh/id',
  '.config/gh/hosts.yml', '.config/gcloud/application_default_credentials.json'
]
for (const path of protectedPaths) {
  const facts = capabilityFactsForToolCall({
    toolId: 'relay.file_read', toolName: 'file_read', input: { path }, trustedProvenance: 'first-party-relay'
  })
  expect(facts.protectedBoundary === true, `protected path parity: ${path}`)
  expect(approvalForCapability(facts, undefined, 'manual').outcome === 'denied', `protected path denied: ${path}`)
}
for (const input of [
  { path: '.env.example' },
  { path: 'nested/.env.example' },
  { path: '.npmrc.bak' },
  { cwd: 'nested', path: '.env.example' },
  { cwd: '.config', path: 'gh.example/hosts.yml' }
]) {
  const facts = capabilityFactsForToolCall({
    toolId: 'relay.file_read', toolName: 'file_read', input, trustedProvenance: 'first-party-relay'
  })
  expect(facts.protectedBoundary !== true, `safe near-miss remains safe: ${JSON.stringify(input)}`)
}
const malformed = capabilityFactsForToolCall({
  toolId: 'relay.file_read', toolName: 'file_read', input: { path: { nested: '.env' } }, trustedProvenance: 'first-party-relay'
})
expect(malformed.invalidInput === true && classifyCapability(malformed).risk === 'high' && approvalForCapability(malformed, 'always').outcome === 'denied', 'malformed path fails closed')
const missingPath = capabilityFactsForToolCall({
  toolId: 'relay.file_read', toolName: 'file_read', input: {}, trustedProvenance: 'first-party-relay'
})
expect(missingPath.invalidInput === true && approvalForCapability(missingPath, 'always').outcome === 'denied', 'missing required path fails closed')
for (const [toolName, input] of [
  ['file_write', { path: 'src/new.ts' }],
  ['file_edit', { path: 'src/main.ts', old_text: 'old' }],
  ['apply_patch', {}]
] as const) {
  const malformedMutation = capabilityFactsForToolCall({
    toolId: `relay.${toolName}`, toolName, input, trustedProvenance: 'first-party-relay'
  })
  expect(malformedMutation.invalidInput === true && approvalForCapability(malformedMutation, undefined, 'bypass').outcome === 'denied', `malformed ${toolName} fails closed`)
}
const cwdProtected = capabilityFactsForToolCall({
  toolId: 'relay.file_read', toolName: 'file_read', input: { cwd: '.config', path: 'gh/hosts.yml' }, trustedProvenance: 'first-party-relay'
})
expect(cwdProtected.protectedBoundary === true, 'cwd plus relative protected path parity')

const network = capabilityFactsForToolCall({
  toolId: 'relay.http_fetch', toolName: 'http_fetch', input: { url: 'https://example.test/a' }, trustedProvenance: 'first-party-relay'
})
expect(network.domain === 'example.test' && network.networkRequested === true, 'structured domain fact')
expect(approvalForCapability(network, undefined, 'manual').outcome === 'user-approval', 'manual network asks')
expect(approvalForCapability(network, undefined, 'bypass').outcome === 'approved', 'bypass network skips prompt')

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
expect(!rememberedApprovalCanAutoAnswer(staleMcp, 'always', 'manual'), 'stale broad approval cannot suppress narrowed MCP approval')

console.log('capability policy behavioral acceptance: PASS')
