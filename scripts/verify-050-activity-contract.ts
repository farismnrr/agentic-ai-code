import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'

const read = (file: string) => readFileSync(file, 'utf8')
const expect = (condition: boolean, message: string) => {
  if (!condition) throw new Error(`Plan 050 verification failed: ${message}`)
}
const has = (value: string, fragment: string) => value.includes(fragment)
const hasKey = (value: unknown, key: string): boolean => {
  if (Array.isArray(value)) return value.some(item => hasKey(item, key))
  if (!value || typeof value !== 'object') return false
  return Object.entries(value).some(([entry, nested]) => entry === key || hasKey(nested, key))
}
const contract = JSON.parse(read('.agents/contracts/050-activity-event-v1.json'))
const activity = read('packages/rust-tools/application/src/activity.rs')
const journal = read('packages/rust-tools/infrastructure/src/activity/journal.rs')
const runtime = read('packages/rust-tools/infrastructure/src/activity/mod.rs')
const transport = read('packages/rust-tools/infrastructure/src/transport/tools.rs')
const taskCalls = read('packages/rust-tools/infrastructure/src/transport/task_calls.rs')
const toolHelpers = read('packages/rust-tools/infrastructure/src/transport/tool_helpers.rs')
const mcpHttp = read('packages/rust-tools/infrastructure/src/transport/mcp_http.rs')
const catalog = read('packages/rust-tools/interfaces/src/mcp/catalog.rs')
const requests = read('packages/rust-tools/application/src/execution/requests.rs')
const mutation = read('packages/rust-tools/application/src/workspace/mutate.rs')
const patch = read('packages/rust-tools/application/src/workspace/patch.rs')
const database = read('server/infrastructure/database/activity.ts')
const schema = read('server/database/schema.ts')
const ui = read('app/components/workspace/WorkspaceActivityView.vue')
const localRelay = read('app/composables/useRelayAgent.ts')
const localController = read('app/composables/chat/local-tool-controller.ts')
const localTerminalTool = read('server/infrastructure/ai/local-terminal-tool.ts')
const telemetry = read('packages/rust-tools/infrastructure/src/telemetry.rs')

expect(contract.properties.contract_version.const === 'activity.event.v1', 'contract version is not frozen')
expect(contract.additionalProperties === false, 'contract must reject unknown fields')
expect(contract.properties.activity_id.maxLength === 256 && contract.properties.source_sequence.minimum === 1, 'identity bounds are missing')
expect(contract.properties.category.enum.length === 9 && contract.properties.effects.items.enum.length === 9, 'canonical vocabularies are incomplete')
for (const field of contract.forbidden) expect(!hasKey(contract.properties, field), `forbidden field appears in contract properties: ${field}`)
for (const field of ['raw_arguments', 'raw_result', 'prompt', 'auth', 'env', 'stdout', 'stderr']) expect(!has(activity, field), `raw field appears in activity implementation: ${field}`)
expect(has(activity, 'deny_unknown_fields') && has(activity, 'transition_allowed') && has(activity, 'External MCP client'), 'contract validation or truthful actor fallback is missing')
expect(has(activity, 'workspace_root_fingerprint') && has(activity, 'SHA256'), 'canonical root fingerprint is missing')
expect(has(activity, 'client_info') && has(activity, 'ClientInfo') && has(activity, 'actor: actor_or_external(None)') && !has(activity, 'ChatGPT'), 'caller identity must be metadata-only and vendor-neutral')

expect(has(journal, 'synchronous=FULL') && has(journal, 'journal_mode=WAL'), 'journal durability pragmas are missing')
expect(has(journal, 'crypto::seal') && has(journal, 'checksum') && has(journal, 'acknowledge'), 'journal confidentiality/integrity/ack behavior is missing')
expect(has(journal, 'recover_stale') && has(journal, 'Status::Interrupted') && has(journal, 'JournalError::Full') && has(journal, 'activity_id = ?2'), 'restart, quota, and lifecycle-safe ack behavior is missing')
expect(has(runtime, 'outcome could not be durably recorded') && has(runtime, 'map_journal_error'), 'terminal outcome failure does not leave a durable interrupted fallback')
expect(has(runtime, 'MAX_EXPORT_BYTES') && has(runtime, 'bearer_auth') && has(runtime, 'UNAUTHORIZED') && has(runtime, 'FORBIDDEN') && has(runtime, 'RETRY_AFTER'), 'bounded authenticated exporter behavior is missing')
expect(has(transport, 'state.activity.record_start') && transport.indexOf('record_start') < transport.indexOf('dispatch_tool_call'), 'required-mode admission is not before execution')
expect(has(toolHelpers, 'extract_activity_evidence') && has(taskCalls, 'task_lifecycle::observe'), 'all execution classes do not share the recorder boundary')
expect(!catalog.includes('agent_delegate') && !transport.includes('agent_delegate') && !requests.includes('primary terminal_exec timeout_ms'), 'removed provider delegation or the Primary-only timeout ceiling remains in the current relay')
expect((catalog.match(/execution_mode/g) ?? []).length >= 3 && transport.includes('async execution requires MCP Tasks capability'), 'sync/async/auto execution contract is not exposed and enforced')
expect(catalog.includes('Stable key for one logical async command') && catalog.includes('Choose a realistic value for the operation'), 'adaptive execution schema does not guide agents on timeout/idempotency behavior')
expect(mcpHttp.includes('io.modelcontextprotocol/tasks') && !mcpHttp.includes('ToolProfile::Full'), 'Tasks capability is not truthfully advertised for both profiles')
expect(localTerminalTool.includes('execution_mode') && localTerminalTool.includes('timeout_ms'), 'local terminal model schema does not expose adaptive execution controls')
expect(localRelay.includes(`extensions: { 'io.modelcontextprotocol/tasks': {} }`) && localRelay.includes(`result.resultType === 'task'`) && localRelay.includes(`'tasks/get'`), 'paired local relay does not advertise and poll MCP Tasks')
expect(localController.includes('idempotencyKey: part.toolCallId') && localController.includes('execution_mode'), 'local terminal does not bind async execution to the stable tool call identity')

expect(has(mutation, 'rename = "_activity"') && has(mutation, 'before_bytes') && has(mutation, 'atomic_replace') && has(mutation, 'no_change'), 'file mutation evidence is not tied to committed before/after state')
expect(has(patch, 'rename = "_activity"') && has(patch, 'activity_evidence') && has(patch, 'preview'), 'patch evidence or dry-run truthfulness is missing')
expect(!has(telemetry, 'activity_evidence') && !has(telemetry, 'activity_payload'), 'activity payload crossed the telemetry module boundary')

for (const table of ['relay_activity_sources', 'relay_activity_workspace_bindings', 'workspace_activity', 'workspace_activity_payloads']) expect(has(schema, table), `missing activity table: ${table}`)
expect(has(schema, 'sourceKey') && has(schema, 'actorSource') && has(schema, 'sourceSequence'), 'source identity or ordering columns are missing')
expect(has(schema, 'clientInfoName') && has(schema, 'clientInfoVersion') && has(database, 'clientInfoName'), 'client-reported metadata is not persisted separately from actor attribution')
expect(has(database, 'hash(token)') && has(database, 'revokedAt') && has(database, 'fs.realpath'), 'source hashing/revocation/canonical binding is missing')
expect(has(database, 'encryptActivityPayload') && has(database, 'assertWorkspaceOwner') && has(database, 'clearThroughSequence'), 'encrypted owned read model is incomplete')
expect(has(database, 'transitionAllowed') && has(database, 'sourceSequence'), 'idempotent lifecycle persistence is incomplete')

for (const file of [
  'server/api/activity/ingest.post.ts',
  'server/api/activity/sources/index.post.ts',
  'server/api/activity/sources/[id].delete.ts',
  'server/api/activity/bindings.post.ts',
  'server/api/workspaces/[id]/activity.get.ts',
  'server/api/workspaces/[id]/activity/[activityId].get.ts',
  'server/api/workspaces/[id]/activity/[activityId]/diff.get.ts',
  'server/api/workspaces/[id]/activity.delete.ts',
  'app/pages/workspaces/[id]/logs.vue'
]) expect(existsSync(file), `required activity route/page is missing: ${file}`)
const ingress = read('server/api/activity/ingest.post.ts')
expect(has(ingress, 'strictObject') && has(ingress, 'BASE64_REGEX'), 'ingress must reject unknown fields and invalid payload encoding')
expect(has(ingress, 'MAX_BATCH_BYTES') && has(ingress, 'readBoundedJson') && has(ingress, 'for await'), 'activity ingestion body is not bounded for chunked requests')
expect(has(ui, 'Load historical diff') && has(ui, 'Load older activity') && has(ui, 'setInterval') && !has(ui, 'v-html'), 'Logs UI lacks safe lazy review or resumable refresh')

execFileSync('cargo', ['run', '-p', 'relay-infrastructure', '--example', 'plan050_activity_acceptance', '--locked'], { stdio: 'inherit' })
console.log('Plan 050 composed activity contract/journal/capture/persistence/UI verification: PASS')
