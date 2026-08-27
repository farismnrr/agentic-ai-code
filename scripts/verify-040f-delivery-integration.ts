import { strict as assert } from 'node:assert'
import { safeInputSummary, toolCategory } from '../app/utils/tool-presentation.ts'
import { capabilityFactsForToolCall, approvalForCapability } from '../shared/utils/capability-policy.ts'

assert.equal(toolCategory('git_stage'), 'git')
assert.equal(toolCategory('git_push'), 'git')
assert.equal(toolCategory('change_request_get'), 'git')
assert.equal(toolCategory('change_request_merge'), 'git')

const approvalSummary = safeInputSummary({
  remote: 'origin',
  branch: 'feat/example',
  head_branch: 'feat/example',
  base_branch: 'main',
  number: 42,
  strategy: 'squash',
  title: 'private title text',
  body: 'token=private-body',
  expected_head_sha: '0123456789012345678901234567890123456789'
})
const serialized = JSON.stringify(approvalSummary)
for (const expected of ['origin', 'feat/example', 'main', '42', 'squash']) assert(serialized.includes(expected), expected)
for (const forbidden of ['private title text', 'private-body', '0123456789012345678901234567890123456789']) assert(!serialized.includes(forbidden), forbidden)

function facts(toolName: string, input: Record<string, unknown>, destructiveHint: boolean, openWorldHint: boolean) {
  return capabilityFactsForToolCall({
    toolId: `relay.${toolName}`,
    toolName,
    input,
    annotations: { readOnlyHint: !destructiveHint, destructiveHint, openWorldHint },
    trustedProvenance: 'first-party-relay'
  })
}

const remoteRead = facts('git_remote_branch_get', { branch: 'main' }, false, true)
assert.deepEqual(remoteRead.effects, ['git_read', 'network_read'])
assert.equal(remoteRead.networkRequested, true)
assert.equal(approvalForCapability(remoteRead, undefined, 'plan').outcome, 'denied')

const push = facts('git_push', { branch: 'feat/example', set_upstream: true }, true, true)
assert.deepEqual(push.effects, ['git_read', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge'])
assert.equal(approvalForCapability(push, undefined, 'bypass').outcome, 'approved')

const prRead = facts('change_request_get', { number: 42 }, false, true)
assert.deepEqual(prRead.effects, ['network_read', 'privileged_bridge'])
assert.equal(approvalForCapability(prRead, undefined, 'bypass').outcome, 'approved')

const merge = facts('change_request_merge', { number: 42, expected_head_sha: '0123456789012345678901234567890123456789', strategy: 'squash' }, true, true)
assert.deepEqual(merge.effects, ['network_read', 'network_write', 'external_mutation', 'privileged_bridge'])
assert.equal(approvalForCapability(merge, undefined, 'manual').outcome, 'user-approval')

const malformedMerge = facts('change_request_merge', { number: 42, strategy: 'squash' }, true, true)
assert.equal(malformedMerge.invalidInput, true)
assert.equal(approvalForCapability(malformedMerge, undefined, 'bypass').outcome, 'denied')

console.log('040F delivery integration acceptance: PASS')
