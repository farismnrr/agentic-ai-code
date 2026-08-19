import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { classifyRawCause } from '../server/core/errors/classify.ts'
import { sanitizeAttributes } from '../server/infrastructure/observability/sanitize.ts'

const timeout = new Error('token=must-not-appear /home/alice/private/file')
timeout.name = 'TimeoutError'
assert.equal(classifyRawCause(timeout), 'timeout')

const aborted = new Error('Bearer should-not-appear')
aborted.name = 'AbortError'
assert.equal(classifyRawCause(aborted), 'aborted')

const coded = Object.assign(new Error('password=should-not-appear'), { code: 'ECONNRESET' })
assert.equal(classifyRawCause(coded), 'ECONNRESET')
assert.equal(classifyRawCause(new Error('provider free text secret=must-not-appear')), 'unclassified')

const sanitized = sanitizeAttributes({
  'request.id': 'req-123',
  'operation': 'chat.tool.policy',
  'outcome': 'denied',
  'tool.name': 'git_push',
  'tool.id': 'relay.git_push',
  'tool.effects': 'git_read,network_read,network_write,external_mutation,privileged_bridge',
  'policy.outcome': 'denied',
  'policy.source': 'runtime-policy',
  'result.classification': 'timeout',
  'error.message': 'Bearer abc.def token=hello /home/alice/private/file',
  'raw.input': 'super-secret',
  'provider.response': 'private-provider-body'
})
assert.equal(sanitized['policy.outcome'], 'denied')
assert.equal(sanitized['result.classification'], 'timeout')
assert(!('raw.input' in sanitized))
assert(!('provider.response' in sanitized))
const serialized = JSON.stringify(sanitized)
for (const forbidden of ['abc.def', 'token=hello', '/home/alice/private/file', 'super-secret', 'private-provider-body']) {
  assert(!serialized.includes(forbidden), `telemetry leaked ${forbidden}`)
}

const requestContext = readFileSync(resolve(import.meta.dirname, '../server/infrastructure/observability/request-context.ts'), 'utf8')
assert(requestContext.includes(`attributes: sanitizeAttributes({ 'request.id': requestId, ...safeAttributes })`), 'request spans must use the shared sanitizer')

const mcpTools = readFileSync(resolve(import.meta.dirname, '../server/infrastructure/mcp/mcp-tools.ts'), 'utf8')
assert(mcpTools.includes(`logger.info('chat.tool.policy'`), 'approval decisions need a bounded policy event even when execution is denied')
assert(mcpTools.includes(`'result.classification': options.abortSignal?.aborted ? 'cancelled' : classifyRawCause(err)`), 'tool failures need bounded cause classification')
assert(!mcpTools.includes(`'result.classification': 'error'`), 'generic error classification must not erase timeout/provider distinctions')
assert(!mcpTools.includes(`'tool.input'`))
assert(!mcpTools.includes(`'tool.arguments'`))

console.log('041C observability/debugging acceptance: PASS')
