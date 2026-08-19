import { randomUUID } from 'node:crypto'
import type { OrchestratorClaim, OrchestratorGraphSnapshot, OrchestratorNode, OrchestratorNodeInput } from '../../../shared/types/orchestration.ts'
import { ORCHESTRATOR_BUDGET_CLASSES, ORCHESTRATOR_ROLES } from '../../../shared/types/orchestration.ts'
import { SUBAGENT_PROFILES, type SubagentEffect } from '../../../shared/types/subagents.ts'

export const ORCHESTRATOR_CAPS = {
  graphs: 128,
  nodes: 24,
  dependencies: 8,
  depth: 8,
  objective: 512,
  id: 64,
  requiredTools: 32,
  requiredEffects: 9,
  evidenceRefs: 16,
  ref: 256,
  ttlMs: 30 * 60 * 1000
} as const

const EFFECTS = new Set<SubagentEffect>(['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge'])
const ROLES = new Set(ORCHESTRATOR_ROLES)
const BUDGETS = new Set(ORCHESTRATOR_BUDGET_CLASSES)
const PROFILES = new Set(SUBAGENT_PROFILES)

type GraphEntry = OrchestratorGraphSnapshot & { user_id: string, conversation_id: string }
const graphs = new Map<string, GraphEntry>()
const keyFor = (userId: string, conversationId: string) => `${userId}\0${conversationId}`
const strictString = (value: unknown, max: number): string => {
  if (typeof value !== 'string' || value.length > max || !value.trim()) throw new Error('malformed orchestrator string')
  return value.trim()
}
const strictStringList = (value: unknown, maxCount: number, maxBytes: number): string[] => {
  if (value === undefined) return []
  if (!Array.isArray(value) || value.length > maxCount) throw new Error('orchestrator list exceeds maximum')
  const parsed = value.map(item => strictString(item, maxBytes))
  if (new Set(parsed).size !== parsed.length) throw new Error('duplicate orchestrator list entry')
  return parsed
}

export function replaceOrchestratorGraph(input: {
  userId: string
  conversationId: string
  parentSessionId: string
  nodes: unknown
  now?: number
}): OrchestratorGraphSnapshot {
  const now = input.now ?? Date.now()
  evictGraphs(now)
  const nodes = parseNodes(input.nodes, now)
  validateDependencies(nodes)
  if (graphDepth(nodes) > ORCHESTRATOR_CAPS.depth) throw new Error('orchestrator graph exceeds maximum depth')
  const graph: GraphEntry = {
    graph_id: `og_${randomUUID().replaceAll('-', '')}`,
    generation: randomUUID(),
    parent_session_id: strictString(input.parentSessionId, 128),
    user_id: input.userId,
    conversation_id: input.conversationId,
    status: 'active',
    nodes,
    ready: [],
    updated_at: now
  }
  recompute(graph, now)
  graphs.set(keyFor(input.userId, input.conversationId), graph)
  evictGraphs(now)
  return publicGraph(graph)
}

export function getOrchestratorGraph(userId: string, conversationId: string, now = Date.now()): OrchestratorGraphSnapshot | undefined {
  evictGraphs(now)
  const graph = graphs.get(keyFor(userId, conversationId))
  return graph ? publicGraph(graph) : undefined
}

export function claimReadyNode(input: {
  userId: string
  conversationId: string
  nodeId: string
  owner: string
  generation: string
  availableTools: string[]
  availableEffects: SubagentEffect[]
  now?: number
}): OrchestratorClaim {
  const now = input.now ?? Date.now()
  const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
  if (graph.status !== 'active') throw new Error('orchestrator graph is not active')
  const node = graph.nodes.find(item => item.id === input.nodeId)
  if (!node || node.status !== 'ready') throw new Error('orchestrator node is not ready')
  const availableTools = new Set(input.availableTools)
  const availableEffects = new Set(input.availableEffects)
  if ((node.required_tools ?? []).some(required => !availableTools.has(required)) || (node.required_effects ?? []).some(required => !availableEffects.has(required))) throw new Error('orchestrator policy prerequisites are unsatisfied')
  const owner = strictString(input.owner, 128)
  node.status = 'running'
  node.owner = owner
  node.lease = randomUUID()
  node.attempt += 1
  node.updated_at = now
  graph.updated_at = now
  recompute(graph, now)
  return {
    graph_id: graph.graph_id,
    generation: graph.generation,
    node_id: node.id,
    lease: node.lease,
    role: node.role,
    objective: node.objective,
    budget_class: node.budget_class,
    profile: node.profile,
    required_tools: [...(node.required_tools ?? [])],
    required_effects: [...(node.required_effects ?? [])]
  }
}

export function blockReadyNode(input: { userId: string, conversationId: string, nodeId: string, generation: string, resultRef?: string, now?: number }): OrchestratorGraphSnapshot {
  const now = input.now ?? Date.now()
  const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
  const node = graph.nodes.find(item => item.id === input.nodeId)
  if (!node || node.status !== 'ready') throw new Error('orchestrator node is not ready')
  node.status = 'blocked'
  node.result_ref = input.resultRef === undefined ? undefined : strictString(input.resultRef, ORCHESTRATOR_CAPS.ref)
  node.updated_at = now
  graph.updated_at = now
  recompute(graph, now)
  return publicGraph(graph)
}

export function releaseClaim(input: { userId: string, conversationId: string, nodeId: string, generation: string, lease: string, now?: number }): OrchestratorGraphSnapshot {
  const now = input.now ?? Date.now()
  const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
  const node = graph.nodes.find(item => item.id === input.nodeId)
  if (!node || node.status !== 'running' || !node.lease || node.lease !== input.lease) throw new Error('stale orchestrator release')
  node.status = 'ready'
  node.owner = undefined
  node.lease = undefined
  node.updated_at = now
  graph.updated_at = now
  recompute(graph, now)
  return publicGraph(graph)
}

export function cancelOrchestratorNodes(input: { userId: string, conversationId: string, generation: string, nodeIds: string[], now?: number }): OrchestratorGraphSnapshot {
  const now = input.now ?? Date.now()
  const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
  const selected = new Set(input.nodeIds)
  if (selected.size === 0 || [...selected].some(id => !graph.nodes.some(node => node.id === id))) throw new Error('invalid orchestrator cancellation target')
  for (const node of graph.nodes) {
    if (!selected.has(node.id) || ['completed', 'failed', 'cancelled', 'invalid'].includes(node.status)) continue
    node.status = 'cancelled'
    node.owner = undefined
    node.lease = undefined
    node.blocked_by = []
    node.updated_at = now
  }
  recompute(graph, now)
  return publicGraph(graph)
}

export function settleClaim(input: {
  userId: string
  conversationId: string
  nodeId: string
  generation: string
  lease: string
  outcome: 'completed' | 'failed' | 'cancelled' | 'blocked'
  resultRef?: string
  evidenceRefs?: string[]
  now?: number
}): OrchestratorGraphSnapshot {
  const now = input.now ?? Date.now()
  const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
  const node = graph.nodes.find(item => item.id === input.nodeId)
  if (!node || node.status !== 'running' || !node.lease || node.lease !== input.lease) throw new Error('stale orchestrator completion')
  node.status = input.outcome
  node.owner = undefined
  node.lease = undefined
  node.result_ref = input.resultRef === undefined ? undefined : strictString(input.resultRef, ORCHESTRATOR_CAPS.ref)
  node.evidence_refs = strictStringList(input.evidenceRefs, ORCHESTRATOR_CAPS.evidenceRefs, ORCHESTRATOR_CAPS.ref)
  node.updated_at = now
  graph.updated_at = now
  recompute(graph, now)
  return publicGraph(graph)
}

export function cancelCurrentOrchestratorGraph(userId: string, conversationId: string, now = Date.now()): OrchestratorGraphSnapshot | undefined {
  evictGraphs(now)
  const graph = graphs.get(keyFor(userId, conversationId))
  if (!graph) return undefined
  return cancelGraphEntry(graph, now)
}

export function cancelOrchestratorGraph(input: { userId: string, conversationId: string, generation: string, now?: number }): OrchestratorGraphSnapshot {
  const now = input.now ?? Date.now()
  return cancelGraphEntry(requireGraph(input.userId, input.conversationId, input.generation, now), now)
}

export function invalidateRunningClaims(userId: string, conversationId: string, generation: string, now = Date.now()): OrchestratorGraphSnapshot {
  const graph = requireGraph(userId, conversationId, generation, now)
  for (const node of graph.nodes) {
    if (node.status !== 'running') continue
    node.status = 'invalid'
    node.owner = undefined
    node.lease = undefined
    node.updated_at = now
  }
  recompute(graph, now)
  return publicGraph(graph)
}

export function resetOrchestratorStoresForTests() {
  graphs.clear()
}

function parseNodes(raw: unknown, now: number): OrchestratorNode[] {
  if (!Array.isArray(raw) || raw.length === 0 || raw.length > ORCHESTRATOR_CAPS.nodes) throw new Error('invalid orchestrator node count')
  const ids = new Set<string>()
  return raw.map((item) => {
    if (!item || typeof item !== 'object') throw new Error('malformed orchestrator node')
    const value = item as Record<string, unknown>
    const id = strictString(value.id, ORCHESTRATOR_CAPS.id)
    const role = value.role
    const objective = strictString(value.objective, ORCHESTRATOR_CAPS.objective)
    const budgetClass = value.budget_class
    const profile = value.profile
    if (!id || ids.has(id) || typeof role !== 'string' || !ROLES.has(role as never) || !objective || typeof budgetClass !== 'string' || !BUDGETS.has(budgetClass as never)) throw new Error('malformed orchestrator node')
    if (profile !== undefined && (typeof profile !== 'string' || !PROFILES.has(profile as never))) throw new Error('malformed orchestrator profile')
    ids.add(id)
    const requiredEffects = strictStringList(value.required_effects, ORCHESTRATOR_CAPS.requiredEffects, 64)
    if (requiredEffects.some(effect => !EFFECTS.has(effect as SubagentEffect))) throw new Error('invalid orchestrator required effect')
    const node: OrchestratorNode = {
      id,
      role: role as OrchestratorNodeInput['role'],
      objective,
      depends_on: strictStringList(value.depends_on, ORCHESTRATOR_CAPS.dependencies, ORCHESTRATOR_CAPS.id),
      budget_class: budgetClass as OrchestratorNodeInput['budget_class'],
      profile: profile as OrchestratorNodeInput['profile'],
      required_tools: strictStringList(value.required_tools, ORCHESTRATOR_CAPS.requiredTools, 160),
      required_effects: requiredEffects as SubagentEffect[],
      status: 'pending',
      attempt: 0,
      evidence_refs: [],
      blocked_by: [],
      updated_at: now
    }
    if (node.depends_on.includes(id)) throw new Error('orchestrator dependency cycle')
    return node
  })
}

function validateDependencies(nodes: OrchestratorNode[]) {
  const ids = new Set(nodes.map(node => node.id))
  for (const node of nodes) if (node.depends_on.some(dep => !ids.has(dep))) throw new Error('unknown orchestrator dependency')
  const visiting = new Set<string>(), visited = new Set<string>()
  const byId = new Map(nodes.map(node => [node.id, node]))
  const visit = (id: string): void => {
    if (visiting.has(id)) throw new Error('orchestrator dependency cycle')
    if (visited.has(id)) return
    visiting.add(id)
    for (const dep of byId.get(id)?.depends_on ?? []) visit(dep)
    visiting.delete(id)
    visited.add(id)
  }
  for (const node of nodes) visit(node.id)
}

function graphDepth(nodes: OrchestratorNode[]): number {
  const byId = new Map(nodes.map(node => [node.id, node]))
  const memo = new Map<string, number>()
  const depth = (id: string): number => {
    const cached = memo.get(id)
    if (cached) return cached
    const value = 1 + Math.max(0, ...(byId.get(id)?.depends_on ?? []).map(depth))
    memo.set(id, value)
    return value
  }
  return Math.max(...nodes.map(node => depth(node.id)))
}

function requireGraph(userId: string, conversationId: string, generation: string, now: number): GraphEntry {
  evictGraphs(now)
  const graph = graphs.get(keyFor(userId, conversationId))
  if (!graph || graph.generation !== generation) throw new Error('stale orchestrator generation')
  return graph
}

function cancelGraphEntry(graph: GraphEntry, now: number): OrchestratorGraphSnapshot {
  for (const node of graph.nodes) {
    if (node.status === 'completed' || node.status === 'failed' || node.status === 'cancelled' || node.status === 'invalid') continue
    node.status = 'cancelled'
    node.owner = undefined
    node.lease = undefined
    node.blocked_by = []
    node.updated_at = now
  }
  graph.status = 'cancelled'
  graph.ready = []
  graph.updated_at = now
  return publicGraph(graph)
}

function recompute(graph: GraphEntry, now: number) {
  const byId = new Map(graph.nodes.map(node => [node.id, node]))
  for (const node of graph.nodes) {
    if (node.status === 'running' || node.status === 'completed' || node.status === 'failed' || node.status === 'blocked' || node.status === 'cancelled' || node.status === 'invalid') continue
    const deps = node.depends_on.map(dep => byId.get(dep)).filter(Boolean) as OrchestratorNode[]
    const failed = deps.filter(dep => ['failed', 'cancelled', 'blocked', 'invalid'].includes(dep.status)).map(dep => dep.id)
    if (failed.length) {
      node.status = 'blocked'
      node.blocked_by = failed
    } else if (deps.every(dep => dep.status === 'completed')) {
      node.status = 'ready'
      node.blocked_by = []
    } else {
      node.status = 'pending'
      node.blocked_by = []
    }
    node.updated_at = now
  }
  graph.ready = graph.nodes.filter(node => node.status === 'ready').map(node => node.id)
  if (graph.status !== 'cancelled') {
    if (graph.nodes.every(node => node.status === 'completed')) graph.status = 'completed'
    else if (graph.nodes.some(node => node.status === 'invalid')) graph.status = 'invalid'
    else if (graph.nodes.every(node => ['completed', 'failed', 'blocked', 'cancelled'].includes(node.status))) graph.status = 'blocked'
    else graph.status = 'active'
  }
  graph.updated_at = now
}

function evictGraphs(now: number) {
  for (const [key, graph] of graphs) if (now - graph.updated_at > ORCHESTRATOR_CAPS.ttlMs) graphs.delete(key)
  if (graphs.size <= ORCHESTRATOR_CAPS.graphs) return
  const oldest = [...graphs.entries()].sort((a, b) => a[1].updated_at - b[1].updated_at).slice(0, graphs.size - ORCHESTRATOR_CAPS.graphs)
  for (const [key] of oldest) graphs.delete(key)
}

function publicGraph(graph: GraphEntry): OrchestratorGraphSnapshot {
  const { user_id: _, conversation_id: __, ...safe } = graph
  return structuredClone(safe)
}
