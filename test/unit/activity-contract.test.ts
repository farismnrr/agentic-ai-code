import { strict as assert } from 'node:assert'
import { existsSync, readFileSync } from 'node:fs'

const read = (file: string) => readFileSync(file, 'utf8')
const hasKey = (value: unknown, key: string): boolean => {
  if (Array.isArray(value)) return value.some(item => hasKey(item, key))
  if (!value || typeof value !== 'object') return false
  return Object.entries(value).some(([entry, nested]) => entry === key || hasKey(nested, key))
}

const contract = JSON.parse(read('.agents/contracts/050-activity-event-v1.json'))
assert.equal(contract.properties.contract_version.const, 'activity.event.v1')
assert.equal(contract.additionalProperties, false)
assert.equal(contract.properties.activity_id.maxLength, 256)
assert.equal(contract.properties.source_sequence.minimum, 1)
for (const field of contract.forbidden) {
  assert.equal(hasKey(contract.properties, field), false, `forbidden field appears in contract properties: ${field}`)
}

const schema = read('server/database/schema.ts')
const database = read('server/infrastructure/database/activity.ts')
for (const table of ['relay_activity_sources', 'relay_activity_workspace_bindings', 'workspace_activity', 'workspace_activity_payloads']) {
  assert.ok(schema.includes(table), `missing activity table: ${table}`)
}
assert.ok(database.includes('encryptActivityPayload'))
assert.ok(database.includes('assertWorkspaceOwner'))
assert.ok(database.includes('clearThroughSequence'))
assert.ok(database.includes('if (!binding) {'))
assert.ok(database.includes('accepted.push(event.recordId)'))
assert.ok(database.includes('evidence.evidence = \'unavailable\''))
assert.ok(database.includes('if (event.payload && payloadSecret)'))

for (const file of [
  'server/api/activity/ingest.post.ts',
  'server/api/activity/sources/index.post.ts',
  'server/api/activity/sources/[id].delete.ts',
  'server/api/activity/bindings.post.ts',
  'server/api/mcp-servers/[id]/activity-bootstrap.post.ts',
  'server/api/workspaces/[id]/activity.get.ts',
  'server/api/workspaces/[id]/activity/[activityId].get.ts',
  'server/api/workspaces/[id]/activity/[activityId]/diff.get.ts',
  'server/api/workspaces/[id]/activity.delete.ts',
  'app/pages/workspaces/[id]/logs.vue'
]) {
  assert.ok(existsSync(file), `required activity route/page is missing: ${file}`)
}

const ingress = read('server/api/activity/ingest.post.ts')
assert.ok(ingress.includes('strictObject'))
assert.ok(ingress.includes('MAX_BATCH_BYTES'))
assert.ok(ingress.includes('readBoundedJson'))

const routeAuth = read('app/middleware/auth.global.ts')
assert.ok(routeAuth.includes('if (to.path.startsWith(\'/api/\')) return'), 'browser route auth must not redirect API callers before endpoint-owned authentication')

const bootstrap = read('server/infrastructure/activity/bootstrap.ts')
assert.ok(bootstrap.includes('serverUrl.hostname.toLowerCase() === `mcp.${operatorSuffix}`'))
assert.ok(bootstrap.includes('client.supportsActivityBootstrap?.()'))
assert.ok(bootstrap.includes('activityDatabase.enroll'))
assert.ok(bootstrap.includes('bindAllWorkspaces'))
assert.equal(read('server/api/mcp-servers/[id]/activity-bootstrap.post.ts').includes('sourceToken'), false, 'source token must never cross the browser API')

const modernMcpClient = read('server/infrastructure/mcp/modern-http-client.ts')
assert.ok(modernMcpClient.includes('\'io.masihawam/activity-bootstrap\''))
assert.ok(modernMcpClient.includes('this.request(\'server/activity_configure\''))

const ui = read('app/components/workspace/WorkspaceActivityView.vue')
assert.ok(ui.includes('Load historical diff'))
assert.ok(ui.includes('Load older activity'))
assert.ok(ui.includes('setInterval'))
assert.equal(ui.includes('v-html'), false)
