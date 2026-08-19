import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  ORCHESTRATOR_CAPS,
  cancelCurrentOrchestratorGraph,
  cancelOrchestratorGraph,
  claimReadyNode,
  getOrchestratorGraph,
  invalidateRunningClaims,
  replaceOrchestratorGraph,
  resetOrchestratorStoresForTests,
  settleClaim
} from '../server/application/orchestration/task-graph.ts'

const owner = { userId: 'user-1', conversationId: 'conv-1', parentSessionId: 'conv-1' }
const policy = { availableTools: ['file_read'], availableEffects: ['workspace_read'] as Array<'workspace_read'> }
const node = (id: string, depends_on: string[] = [], role: 'planner' | 'researcher' | 'reviewer' | 'writer' | 'verifier' | 'general' = 'general') => ({
  id,
  role,
  objective: `Execute bounded objective ${id}`,
  depends_on,
  budget_class: 'medium' as const,
  required_tools: ['file_read'],
  required_effects: ['workspace_read'] as const
})

resetOrchestratorStoresForTests()

// Linear chain: only the root is ready, then readiness advances one node at a time.
let graph = replaceOrchestratorGraph({ ...owner, nodes: [node('a'), node('b', ['a']), node('c', ['b'])], now: 1000 })
assert.deepEqual(graph.ready, ['a'])
const claimA = claimReadyNode({ ...owner, ...policy, nodeId: 'a', owner: 'child-a', generation: graph.generation, now: 1001 })
assert.equal(claimA.node_id, 'a')
assert.deepEqual(claimA.required_effects, ['workspace_read'])
assert.throws(() => claimReadyNode({ ...owner, ...policy, nodeId: 'b', owner: 'child-b', generation: graph.generation, now: 1002 }), /not ready/)
graph = settleClaim({ ...owner, nodeId: 'a', generation: graph.generation, lease: claimA.lease, outcome: 'completed', resultRef: 'rr_a', evidenceRefs: ['git/status'], now: 1003 })
assert.deepEqual(graph.ready, ['b'])
const claimB = claimReadyNode({ ...owner, ...policy, nodeId: 'b', owner: 'child-b', generation: graph.generation, now: 1004 })
graph = settleClaim({ ...owner, nodeId: 'b', generation: graph.generation, lease: claimB.lease, outcome: 'completed', now: 1005 })
assert.deepEqual(graph.ready, ['c'])
const claimC = claimReadyNode({ ...owner, ...policy, nodeId: 'c', owner: 'child-c', generation: graph.generation, now: 1006 })
graph = settleClaim({ ...owner, nodeId: 'c', generation: graph.generation, lease: claimC.lease, outcome: 'completed', now: 1007 })
assert.equal(graph.status, 'completed')
assert.deepEqual(graph.ready, [])

// Fan-out/fan-in: both independent middle nodes become ready, final waits for both.
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('root'), node('left', ['root']), node('right', ['root']), node('join', ['left', 'right'])], now: 2000 })
const rootClaim = claimReadyNode({ ...owner, ...policy, nodeId: 'root', owner: 'root-child', generation: graph.generation, now: 2001 })
graph = settleClaim({ ...owner, nodeId: 'root', generation: graph.generation, lease: rootClaim.lease, outcome: 'completed', now: 2002 })
assert.deepEqual(graph.ready, ['left', 'right'])
const left = claimReadyNode({ ...owner, ...policy, nodeId: 'left', owner: 'left-child', generation: graph.generation, now: 2003 })
const right = claimReadyNode({ ...owner, ...policy, nodeId: 'right', owner: 'right-child', generation: graph.generation, now: 2004 })
graph = settleClaim({ ...owner, nodeId: 'left', generation: graph.generation, lease: left.lease, outcome: 'completed', now: 2005 })
assert.deepEqual(graph.ready, [])
graph = settleClaim({ ...owner, nodeId: 'right', generation: graph.generation, lease: right.lease, outcome: 'completed', now: 2006 })
assert.deepEqual(graph.ready, ['join'])

// Failed predecessor blocks dependents deterministically.
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('source'), node('dependent', ['source'])], now: 3000 })
const source = claimReadyNode({ ...owner, ...policy, nodeId: 'source', owner: 'source-child', generation: graph.generation, now: 3001 })
graph = settleClaim({ ...owner, nodeId: 'source', generation: graph.generation, lease: source.lease, outcome: 'failed', now: 3002 })
const dependent = graph.nodes.find(item => item.id === 'dependent')
assert.equal(dependent?.status, 'blocked')
assert.deepEqual(dependent?.blocked_by, ['source'])

// A child-reported blocked outcome remains terminal for this generation; retry requires replacement/reconciliation.
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('blocked-child')], now: 3200 })
const blockedClaim = claimReadyNode({ ...owner, ...policy, nodeId: 'blocked-child', owner: 'blocked-owner', generation: graph.generation, now: 3201 })
graph = settleClaim({ ...owner, nodeId: 'blocked-child', generation: graph.generation, lease: blockedClaim.lease, outcome: 'blocked', now: 3202 })
assert.equal(graph.nodes[0]?.status, 'blocked')
assert.deepEqual(graph.ready, [])
assert.throws(() => claimReadyNode({ ...owner, ...policy, nodeId: 'blocked-child', owner: 'retry-owner', generation: graph.generation, now: 3203 }), /not ready|not active/)

// Identity fields fail closed instead of truncating into potentially colliding ownership/session identities.
assert.throws(() => replaceOrchestratorGraph({ ...owner, parentSessionId: 's'.repeat(129), nodes: [node('identity')] }), /malformed orchestrator string/)
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('identity')], now: 3300 })
assert.throws(() => claimReadyNode({ ...owner, ...policy, nodeId: 'identity', owner: 'o'.repeat(129), generation: graph.generation, now: 3301 }), /malformed orchestrator string/)
assert.equal(getOrchestratorGraph(owner.userId, owner.conversationId, 3302)?.nodes[0]?.status, 'ready')

// Dependency-ready work still cannot start when parent/operator policy prerequisites are unavailable.
graph = replaceOrchestratorGraph({ ...owner, nodes: [{ ...node('policy'), required_effects: ['workspace_read', 'network_read'] }], now: 3500 })
assert.deepEqual(graph.ready, ['policy'])
assert.throws(() => claimReadyNode({ ...owner, ...policy, nodeId: 'policy', owner: 'policy-child', generation: graph.generation, now: 3501 }), /policy prerequisites/)
assert.equal(getOrchestratorGraph(owner.userId, owner.conversationId, 3502)?.nodes[0]?.status, 'ready')

// Cycles, missing dependencies, oversized graphs and excessive depth fail closed.
assert.throws(() => replaceOrchestratorGraph({ ...owner, nodes: [node('x', ['y']), node('y', ['x'])] }), /cycle/)
assert.throws(() => replaceOrchestratorGraph({ ...owner, nodes: [node('x', ['missing'])] }), /unknown orchestrator dependency/)
assert.throws(() => replaceOrchestratorGraph({ ...owner, nodes: Array.from({ length: ORCHESTRATOR_CAPS.nodes + 1 }, (_, index) => node(`n${index}`)) }), /node count/)
const deep = Array.from({ length: ORCHESTRATOR_CAPS.depth + 1 }, (_, index) => node(`d${index}`, index === 0 ? [] : [`d${index - 1}`]))
assert.throws(() => replaceOrchestratorGraph({ ...owner, nodes: deep }), /maximum depth/)

// Replacing a graph supersedes the old generation; stale child results cannot mutate it.
const oldGraph = replaceOrchestratorGraph({ ...owner, nodes: [node('old')], now: 4000 })
const oldClaim = claimReadyNode({ ...owner, ...policy, nodeId: 'old', owner: 'old-child', generation: oldGraph.generation, now: 4001 })
const newGraph = replaceOrchestratorGraph({ ...owner, nodes: [node('new')], now: 4002 })
assert.throws(() => settleClaim({ ...owner, nodeId: 'old', generation: oldGraph.generation, lease: oldClaim.lease, outcome: 'completed', now: 4003 }), /stale orchestrator generation/)
assert.deepEqual(getOrchestratorGraph(owner.userId, owner.conversationId, 4004)?.ready, ['new'])

// Wrong lease cannot settle a running claim.
const newClaim = claimReadyNode({ ...owner, ...policy, nodeId: 'new', owner: 'new-child', generation: newGraph.generation, now: 4005 })
assert.throws(() => settleClaim({ ...owner, nodeId: 'new', generation: newGraph.generation, lease: 'stale-lease', outcome: 'completed', now: 4006 }), /stale orchestrator completion/)

// Explicit process-recovery behavior invalidates in-flight ownership rather than resurrecting it.
graph = invalidateRunningClaims(owner.userId, owner.conversationId, newGraph.generation, 4007)
assert.equal(graph.status, 'invalid')
assert.equal(graph.nodes.find(item => item.id === 'new')?.status, 'invalid')
assert.throws(() => settleClaim({ ...owner, nodeId: 'new', generation: newGraph.generation, lease: newClaim.lease, outcome: 'completed', now: 4008 }), /stale orchestrator completion/)

// Process restart semantics are explicit invalidation-by-loss: process-local state is never silently resurrected.
const restartGraph = replaceOrchestratorGraph({ ...owner, nodes: [node('restart-root')], now: 4500 })
const restartClaim = claimReadyNode({ ...owner, ...policy, nodeId: 'restart-root', owner: 'restart-child', generation: restartGraph.generation, now: 4501 })
resetOrchestratorStoresForTests()
assert.equal(getOrchestratorGraph(owner.userId, owner.conversationId, 4502), undefined)
assert.throws(() => settleClaim({ ...owner, nodeId: 'restart-root', generation: restartGraph.generation, lease: restartClaim.lease, outcome: 'completed', now: 4503 }), /stale orchestrator generation/)

// Parent cancellation truthfully terminates all non-terminal nodes and clears ownership.
graph = replaceOrchestratorGraph({ ...owner, nodes: [node('cancel-root'), node('cancel-next', ['cancel-root'])], now: 5000 })
const cancelClaim = claimReadyNode({ ...owner, ...policy, nodeId: 'cancel-root', owner: 'cancel-child', generation: graph.generation, now: 5001 })
assert.ok(cancelClaim.lease)
graph = cancelOrchestratorGraph({ ...owner, generation: graph.generation, now: 5002 })
assert.equal(graph.status, 'cancelled')
assert.deepEqual(graph.ready, [])
assert(graph.nodes.every(item => item.status === 'cancelled'))
assert(graph.nodes.every(item => !item.owner && !item.lease))
assert.equal(cancelCurrentOrchestratorGraph(owner.userId, owner.conversationId, 5003)?.status, 'cancelled')

// Snapshot contribution remains bounded and contains no hidden-reasoning field.
graph = replaceOrchestratorGraph({ ...owner, nodes: Array.from({ length: ORCHESTRATOR_CAPS.nodes }, (_, index) => node(`bounded-${index}`)), now: 6000 })
const serialized = JSON.stringify(graph)
assert(Buffer.byteLength(serialized) < 64 * 1024)
assert(!serialized.includes('chain_of_thought'))
assert(!serialized.includes('reasoning'))

const graphSource = readFileSync(resolve(import.meta.dirname, '../server/application/orchestration/task-graph.ts'), 'utf8')
const chatSource = readFileSync(resolve(import.meta.dirname, '../server/application/chat/execute-chat-turn.ts'), 'utf8')
const subagentToolSource = readFileSync(resolve(import.meta.dirname, '../server/infrastructure/ai/subagent-tool.ts'), 'utf8')
assert(!graphSource.includes('SubagentRuntime'), '042A task-graph state must remain independent of child execution')
assert(chatSource.includes('orchestrator_plan: buildOrchestratorPlanTool'), 'parent agent must receive the bounded orchestration planning tool')
assert(chatSource.includes('buildOrchestration(subagentInput)'), 'parent agent must receive scheduler-aware orchestration tools')
assert(subagentToolSource.includes(`input.abortSignal.addEventListener('abort', cancelActiveRun`), 'parent cancellation must propagate through the scheduler into owned child tasks')

console.log('042A orchestrator state-machine acceptance: PASS')
