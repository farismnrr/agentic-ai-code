import type { SubagentEffect, SubagentProfileName } from './subagents.ts'

export const ORCHESTRATOR_ROLES = ['planner', 'researcher', 'reviewer', 'writer', 'verifier', 'general'] as const
export type OrchestratorRole = typeof ORCHESTRATOR_ROLES[number]

export const ORCHESTRATOR_BUDGET_CLASSES = ['small', 'medium', 'large'] as const
export type OrchestratorBudgetClass = typeof ORCHESTRATOR_BUDGET_CLASSES[number]

export const ORCHESTRATOR_NODE_STATUSES = ['pending', 'ready', 'running', 'blocked', 'completed', 'failed', 'cancelled', 'invalid'] as const
export type OrchestratorNodeStatus = typeof ORCHESTRATOR_NODE_STATUSES[number]

export interface OrchestratorNodeInput {
  id: string
  role: OrchestratorRole
  objective: string
  depends_on: string[]
  budget_class: OrchestratorBudgetClass
  profile?: SubagentProfileName
  required_tools?: string[]
  required_effects?: SubagentEffect[]
}

export interface OrchestratorNode extends OrchestratorNodeInput {
  status: OrchestratorNodeStatus
  owner?: string
  lease?: string
  attempt: number
  evidence_refs: string[]
  result_ref?: string
  blocked_by: string[]
  updated_at: number
}

export interface OrchestratorGraphSnapshot {
  graph_id: string
  generation: string
  parent_session_id: string
  status: 'active' | 'completed' | 'blocked' | 'cancelled' | 'invalid'
  nodes: OrchestratorNode[]
  ready: string[]
  updated_at: number
}

export interface OrchestratorClaim {
  graph_id: string
  generation: string
  node_id: string
  lease: string
  role: OrchestratorRole
  objective: string
  budget_class: OrchestratorBudgetClass
  profile?: SubagentProfileName
  required_tools: string[]
  required_effects: SubagentEffect[]
}
