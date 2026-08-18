export const SUBAGENT_PROFILES = ['explore', 'plan', 'review', 'verify', 'general-purpose'] as const
export type SubagentProfileName = typeof SUBAGENT_PROFILES[number]

export const SUBAGENT_STATUSES = ['completed', 'blocked', 'cancelled', 'budget_exhausted', 'failed', 'invalid'] as const
export type SubagentStatus = typeof SUBAGENT_STATUSES[number]

export type SubagentModelPolicy = 'fast' | 'default' | 'strong'
export type SubagentWorkingMode = 'read-only' | 'workspace'
export type SubagentEffect = 'workspace_read' | 'workspace_write' | 'workspace_delete' | 'git_read' | 'process_exec' | 'network_read' | 'network_write' | 'external_mutation' | 'privileged_bridge'

export interface SubagentProfile {
  name: SubagentProfileName
  description: string
  model_policy: SubagentModelPolicy
  tools: { allow: string[], deny: string[] }
  effects: { allow: SubagentEffect[], deny: SubagentEffect[] }
  max_turns: number
  max_tool_calls: number
  max_output_tokens: number
  max_context_tokens: number
  max_wall_time_ms: number
  max_depth: number
  working_mode: SubagentWorkingMode
  skills: string[]
  instructions: string
}

export interface SubagentBudget {
  max_turns: number
  max_tool_calls: number
  max_output_tokens: number
  max_context_tokens: number
  max_wall_time_ms: number
  max_depth: number
}

export interface SubagentUsage {
  turns: number
  tool_calls: number
  output_tokens: number
  context_tokens: number
  wall_time_ms: number
  depth: number
}

export interface SubagentAuthority {
  tools: string[]
  effects: SubagentEffect[]
  working_mode: SubagentWorkingMode
  model_policy: SubagentModelPolicy
  workspace_root: string
}

export interface SubagentContextPackage {
  task: string
  repository_identity: string
  workspace_root: string
  cwd: string
  references: string[]
  parent_summary?: string
  skill_instructions?: string[]
}

export interface SubagentEvidence { reference: string, detail: string }
export interface SubagentResult {
  status: SubagentStatus
  summary: string
  findings: string[]
  evidence: SubagentEvidence[]
  validation: string[]
  remaining_risks: string[]
  session_id: string
  profile: SubagentProfileName
  usage: SubagentUsage
}

export interface SubagentRequest {
  user_id: string
  parent_session_id: string
  parent_authority: SubagentAuthority
  profile: SubagentProfileName
  task: string
  cwd?: string
  context_refs?: string[]
  budget?: Partial<SubagentBudget>
  depth?: number
  abort_signal?: AbortSignal
  model?: unknown
  approvals?: Record<string, string>
  permission_mode?: 'plan' | 'workspace' | 'autonomous' | 'manual'
}
