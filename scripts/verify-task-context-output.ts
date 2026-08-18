/* eslint-disable @stylistic/max-statements-per-line */
import { strict as assert } from 'node:assert'
import { classifyOutput, consumeContinuation, inspectContext, issueContinuation, putResultRef, getResultRef, resetTaskContextStoresForTests, taskLedgerFor, updateTaskLedger, TASK_CAPS } from '../server/application/task-context-output.ts'
import { composeAgentTools } from '../server/application/chat/tool-composition.ts'

resetTaskContextStoresForTests()
const base = { userId: 'u1', conversationId: 'c1', sessionId: 's1' }
assert.throws(() => updateTaskLedger({ ...base, tasks: Array.from({ length: 33 }, (_, i) => ({ id: `t${i}`, title: 'x', status: 'pending', depends_on: [] })) }))
assert.throws(() => updateTaskLedger({ ...base, tasks: [{ id: 'a', title: 'a', status: 'bad', depends_on: [] }] }))
assert.throws(() => updateTaskLedger({ ...base, tasks: [{ id: 'a', title: 'a', status: 'pending', depends_on: ['b'] }] }))
assert.throws(() => updateTaskLedger({ ...base, tasks: [{ id: 'a', title: 'a', status: 'pending', depends_on: ['b'] }, { id: 'b', title: 'b', status: 'pending', depends_on: ['a'] }] }))
assert.equal(updateTaskLedger({ ...base, tasks: [{ id: 'a', title: 'a', status: 'completed', depends_on: [], short_note: 'state only' }] }).tasks[0]?.status, 'completed')
assert.match(updateTaskLedger({ ...base, tasks: [{ id: 'a', title: 'a', status: 'completed', depends_on: [], short_note: 'state only' }] }).tasks[0]?.short_note ?? '', /state only/)
assert.equal(taskLedgerFor('u2', 'c1', 's1').tasks.length, 0)
updateTaskLedger({ ...base, tasks: [{ id: 'old', title: 'old', status: 'pending', depends_on: [] }], now: 0 })
assert.equal(taskLedgerFor('u1', 'c1', 's1', TASK_CAPS.ttlMs + 1).tasks.length, 0)
assert.equal(composeAgentTools({ task_update: 'internal' }, { task_update: 'mcp', external: 'mcp' }).task_update, 'internal')
const claims = { tool: 'text_search', query: '{"query":"needle"}', scope: '/repo', limit: 2, offset: 2, retrieved: 2, owner: 'u1/s1', snapshot: 'sha' }
const token = issueContinuation(claims)
assert.equal(consumeContinuation(token, { tool: claims.tool, query: claims.query, scope: claims.scope, limit: claims.limit, owner: claims.owner, snapshot: claims.snapshot }).offset, 2)
assert.throws(() => consumeContinuation(`${token}x`, { tool: claims.tool, query: claims.query, scope: claims.scope, limit: claims.limit, owner: claims.owner, snapshot: claims.snapshot }))
assert.throws(() => consumeContinuation(token, { ...claims, scope: '/other' }))
assert.throws(() => consumeContinuation(token, { ...claims, limit: 3 }))
assert.throws(() => consumeContinuation(token, { ...claims, tool: 'other' }))
assert.throws(() => consumeContinuation(token, { ...claims, owner: 'u2/s1' }))
assert.throws(() => consumeContinuation(token, { ...claims, snapshot: 'other' }))
assert.throws(() => consumeContinuation(issueContinuation({ ...claims, expiresAt: Date.now() + 1 }), { tool: claims.tool, query: claims.query, scope: claims.scope, limit: claims.limit, owner: claims.owner, snapshot: claims.snapshot }, Date.now() + 1))
const ref = putResultRef('u1/s1', 'bounded result'); assert.equal(getResultRef('u1/s1', ref), 'bounded result'); assert.equal(getResultRef('u2/s1', ref), undefined)
assert.equal(classifyOutput(100), 'inline_small'); assert.equal(classifyOutput(200_000), 'summarized_large'); assert.equal(classifyOutput(100, true), 'retained_failure')
const context = inspectContext({ contextWindow: 1000, usedTokens: 600, measuredAtBoundary: false, maxOutputTokens: 200, summary: '[private summary]', childCount: 99, backgroundCount: 99 })
assert.equal(context.usedTokensKind, 'estimated_from_provider_boundary'); assert.equal(context.headroom, 200); assert.equal(context.summaryPresent, true); assert.equal(context.activeChildren, 32)
assert.equal(inspectContext({ usedTokens: null }).usedTokensKind, 'unknown')
assert.equal(inspectContext({ usedTokens: 600, measuredAtBoundary: true }).usedTokensKind, 'provider_measured_boundary')
console.log('task/context/output behavioral acceptance: PASS')
