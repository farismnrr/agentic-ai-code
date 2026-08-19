import { randomUUID } from 'node:crypto'
import type { BackgroundTaskMetadata, BackgroundTaskState, SubagentAuthority, SubagentBudget, SubagentEffect, SubagentProfileName } from '../../../shared/types/subagents.ts'
import type { OrchestratorGraphSnapshot, OrchestratorNode, OrchestratorRole } from '../../../shared/types/orchestration.ts'
import {
  blockReadyNode,
  cancelOrchestratorGraph,
  cancelOrchestratorNodes,
  claimReadyNode,
  getOrchestratorGraph,
  releaseClaim,
  settleClaim
} from './task-graph.ts'

export const ORCHESTRATOR_CONCURRENCY = { global: 4, perParent: 2 } as const

export const ORCHESTRATOR_ROLE_PROFILE: Record<OrchestratorRole, SubagentProfileName> = {
  planner: 'plan',
  researcher: 'explore',
  reviewer: 'review',
  writer: 'general-purpose',
  verifier: 'verify',
  general: 'explore'
}

export const ORCHESTRATOR_BUDGETS: Record<OrchestratorNode['budget_class'], Partial<SubagentBudget>> = {
  small: { max_turns: 6, max_tool_calls: 10, max_output_tokens: 1536, max_context_tokens: 4096, max_wall_time_ms: 60000 },
  medium: { max_turns: 12, max_tool_calls: 24, max_output_tokens: 3072, max_context_tokens: 6144, max_wall_time_ms: 120000 },
  large: { max_turns: 16, max_tool_calls: 32, max_output_tokens: 4096, max_context_tokens: 8192, max_wall_time_ms: 180000 }
}

export interface OrchestratorPreparedDispatch {
  profile: SubagentProfileName
  isolation: 'shared_read' | 'worktree'
  budget: Partial<SubagentBudget>
}

export interface OrchestratorChildPort {
  capacity(parentSessionId: string): { global: number, parent: number }
  prepare(node: OrchestratorNode, parentAuthority: SubagentAuthority): OrchestratorPreparedDispatch
  start(input: { taskId: string, node: OrchestratorNode, prepared: OrchestratorPreparedDispatch }): { task_id: string, state: BackgroundTaskState }
  get(taskId: string): BackgroundTaskMetadata | undefined
  cancel(taskId: string): boolean
}

export class OrchestratorScheduler {
  dispatchReady(input: {
    userId: string
    conversationId: string
    generation: string
    parentSessionId: string
    parentAuthority: SubagentAuthority
    port: OrchestratorChildPort
    now?: number
  }) {
    const now = input.now ?? Date.now()
    const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
    const capacity = input.port.capacity(input.parentSessionId)
    let slots = Math.min(ORCHESTRATOR_CONCURRENCY.global, ORCHESTRATOR_CONCURRENCY.perParent, capacity.global, capacity.parent)
    const started: string[] = []
    const denied: string[] = []
    const queued = [...graph.ready]

    for (const nodeId of queued) {
      if (slots <= 0) break
      const current = requireGraph(input.userId, input.conversationId, input.generation, now)
      const node = current.nodes.find(item => item.id === nodeId)
      if (!node || node.status !== 'ready') continue
      let prepared: OrchestratorPreparedDispatch
      try {
        prepared = input.port.prepare(node, input.parentAuthority)
      } catch {
        denied.push(node.id)
        blockReadyNode({ userId: input.userId, conversationId: input.conversationId, nodeId: node.id, generation: input.generation, resultRef: 'orchestrator/policy-denied', now })
        continue
      }
      const taskId = randomUUID()
      let claim
      try {
        claim = claimReadyNode({
          userId: input.userId,
          conversationId: input.conversationId,
          nodeId: node.id,
          owner: taskId,
          generation: input.generation,
          availableTools: input.parentAuthority.tools,
          availableEffects: input.parentAuthority.effects,
          now
        })
      } catch {
        denied.push(node.id)
        continue
      }
      const launch = input.port.start({ taskId, node, prepared })
      if (!launch.task_id || launch.state === 'rejected') {
        releaseClaim({ userId: input.userId, conversationId: input.conversationId, nodeId: node.id, generation: input.generation, lease: claim.lease, now })
        break
      }
      started.push(node.id)
      slots -= 1
    }

    return {
      graph: requireGraph(input.userId, input.conversationId, input.generation, now),
      started,
      denied,
      queued: requireGraph(input.userId, input.conversationId, input.generation, now).ready.filter(id => !started.includes(id))
    }
  }

  poll(input: { userId: string, conversationId: string, generation: string, port: OrchestratorChildPort, now?: number }): OrchestratorGraphSnapshot {
    const now = input.now ?? Date.now()
    let graph = requireGraph(input.userId, input.conversationId, input.generation, now)
    for (const node of graph.nodes) {
      if (node.status !== 'running' || !node.owner || !node.lease) continue
      const child = input.port.get(node.owner)
      if (!child) {
        graph = settleClaim({ userId: input.userId, conversationId: input.conversationId, nodeId: node.id, generation: input.generation, lease: node.lease, outcome: 'failed', resultRef: 'orchestrator/child-missing', now })
        continue
      }
      const outcome = terminalOutcome(child.state)
      if (!outcome) continue
      graph = settleClaim({
        userId: input.userId,
        conversationId: input.conversationId,
        nodeId: node.id,
        generation: input.generation,
        lease: node.lease,
        outcome,
        resultRef: child.result?.summary_ref,
        evidenceRefs: child.result?.evidence.map(item => item.reference),
        now
      })
    }
    return graph
  }

  cancelNode(input: { userId: string, conversationId: string, generation: string, nodeId: string, subtree?: boolean, port: OrchestratorChildPort, now?: number }) {
    const now = input.now ?? Date.now()
    const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
    const selected = input.subtree ? descendants(graph, input.nodeId) : [input.nodeId]
    for (const node of graph.nodes) if (selected.includes(node.id) && node.status === 'running' && node.owner) input.port.cancel(node.owner)
    return cancelOrchestratorNodes({ userId: input.userId, conversationId: input.conversationId, generation: input.generation, nodeIds: selected, now })
  }

  cancelRun(input: { userId: string, conversationId: string, generation: string, port: OrchestratorChildPort, now?: number }) {
    const now = input.now ?? Date.now()
    const graph = requireGraph(input.userId, input.conversationId, input.generation, now)
    for (const node of graph.nodes) if (node.status === 'running' && node.owner) input.port.cancel(node.owner)
    return cancelOrchestratorGraph({ userId: input.userId, conversationId: input.conversationId, generation: input.generation, now })
  }
}

function requireGraph(userId: string, conversationId: string, generation: string, now: number) {
  const graph = getOrchestratorGraph(userId, conversationId, now)
  if (!graph || graph.generation !== generation) throw new Error('stale orchestrator generation')
  return graph
}

function terminalOutcome(state: BackgroundTaskState): 'completed' | 'failed' | 'cancelled' | 'blocked' | undefined {
  if (state === 'completed') return 'completed'
  if (state === 'cancelled') return 'cancelled'
  if (state === 'blocked' || state === 'rejected') return 'blocked'
  if (state === 'failed' || state === 'budget_exhausted') return 'failed'
  return undefined
}

function descendants(graph: OrchestratorGraphSnapshot, rootId: string) {
  if (!graph.nodes.some(node => node.id === rootId)) throw new Error('invalid orchestrator cancellation target')
  const selected = new Set([rootId])
  let changed = true
  while (changed) {
    changed = false
    for (const node of graph.nodes) {
      if (selected.has(node.id) || !node.depends_on.some(dep => selected.has(dep))) continue
      selected.add(node.id)
      changed = true
    }
  }
  return [...selected]
}

export function requirementsFitAuthority(node: OrchestratorNode, authority: SubagentAuthority) {
  const tools = new Set(authority.tools)
  const effects = new Set<SubagentEffect>(authority.effects)
  return (node.required_tools ?? []).every(tool => tools.has(tool)) && (node.required_effects ?? []).every(effect => effects.has(effect))
}
