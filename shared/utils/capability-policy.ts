export type CapabilityEffect
  = | 'workspace_read'
    | 'workspace_write'
    | 'workspace_delete'
    | 'git_read'
    | 'process_exec'
    | 'network_read'
    | 'network_write'
    | 'external_mutation'
    | 'privileged_bridge'

export type CapabilityMode = 'plan' | 'workspace' | 'autonomous' | 'manual'
export type ApprovalOutcome = 'approved' | 'denied' | 'user-approval'
type CapabilityApprovalDecision = 'always' | 'never'
export type RiskLevel = 'low' | 'medium' | 'high'

export interface CapabilityFacts {
  toolId: string
  effects: CapabilityEffect[]
  path?: string
  domain?: string
  command?: string
  args?: string[]
  networkRequested?: boolean
  destructive?: boolean
  external?: boolean
  trustedProvenance?: 'first-party-relay' | 'native' | 'external'
  requiresConcreteScope?: boolean
}

export interface CapabilityAssessment extends CapabilityFacts {
  risk: RiskLevel
  opaque: boolean
  reason: string
}

const SAFE_READ_TOOLS = new Set([
  'directory_list', 'file_search', 'text_search', 'file_read',
  'git_status', 'git_diff', 'git_log', 'git_show', 'git_blame',
  'code_symbols', 'code_definition', 'code_references', 'code_hover',
  'code_diagnostics', 'code_rename_preview', 'web_search'
])

const OPAQUE_COMMANDS = new Set(['sh', 'bash', 'zsh', 'fish', 'dash', 'eval', 'env', 'xargs'])
const READ_COMMANDS = new Map([
  ['cat', 'workspace_read'], ['head', 'workspace_read'], ['tail', 'workspace_read'],
  ['ls', 'workspace_read'], ['pwd', 'workspace_read'], ['rg', 'workspace_read'],
  ['grep', 'workspace_read'], ['find', 'workspace_read'], ['git', 'git_read']
] as const)

export function classifyCapability(facts: CapabilityFacts): CapabilityAssessment {
  const command = facts.command?.trim().split(/\s+/)[0]?.toLowerCase()
  const opaque = Boolean(command && OPAQUE_COMMANDS.has(command))
  const network = facts.networkRequested === true || facts.effects.some(effect => effect === 'network_read' || effect === 'network_write')
  const destructive = facts.destructive === true || facts.effects.some(effect => effect === 'workspace_delete' || effect === 'network_write' || effect === 'external_mutation' || effect === 'privileged_bridge')
  const lowRisk = facts.effects.every(effect => effect === 'workspace_read' || effect === 'git_read') && !network && !destructive && !opaque
  const risk: RiskLevel = destructive || opaque || facts.external === true ? 'high' : network || facts.effects.includes('process_exec') || facts.effects.includes('workspace_write') ? 'medium' : 'low'
  let reason = opaque ? 'opaque shell or wrapper execution requires explicit review' : facts.external === true ? 'external tool provenance is untrusted and requires explicit review' : network ? 'network access is an independent capability' : destructive ? 'the operation can mutate or delete state' : facts.effects.includes('workspace_write') ? 'workspace mutation requires explicit review' : 'bounded read-only capability'

  if (command && READ_COMMANDS.has(command as 'cat' | 'head' | 'tail' | 'ls' | 'pwd' | 'rg' | 'grep' | 'find' | 'git') && facts.effects.length === 0) {
    reason = 'reviewed direct-argv read-only command'
  }

  return { ...facts, networkRequested: network, destructive, opaque, risk: lowRisk ? 'low' : risk, reason }
}

export function approvalForCapability(
  facts: CapabilityFacts,
  remembered: CapabilityApprovalDecision | undefined,
  mode: CapabilityMode = 'manual'
): { outcome: ApprovalOutcome, assessment: CapabilityAssessment } {
  const assessment = classifyCapability(facts)
  if (remembered === 'never') return { outcome: 'denied', assessment }
  if (mode === 'plan' && assessment.effects.some(effect => effect !== 'workspace_read' && effect !== 'git_read')) {
    return { outcome: 'denied', assessment: { ...assessment, reason: 'Plan mode permits read-only capabilities only' } }
  }
  const trusted = facts.trustedProvenance === 'first-party-relay' || facts.trustedProvenance === 'native'
  const containedWorkspaceMutation = trusted
    && facts.path !== undefined
    && assessment.effects.length === 1
    && assessment.effects[0] === 'workspace_write'
    && !assessment.destructive
    && !assessment.networkRequested
    && !assessment.opaque
  if (containedWorkspaceMutation && (mode === 'workspace' || mode === 'autonomous')) return { outcome: 'approved', assessment }
  if (remembered === 'always' && assessment.risk === 'low' && trusted && !assessment.opaque && !facts.requiresConcreteScope) return { outcome: 'approved', assessment }
  if (SAFE_READ_TOOLS.has(facts.toolId.split('.').pop() ?? '') && assessment.risk === 'low' && trusted) return { outcome: 'approved', assessment }
  return { outcome: 'user-approval', assessment }
}

export function toolEffects(toolName: string, annotations?: { readOnlyHint?: boolean, destructiveHint?: boolean, openWorldHint?: boolean }, trustedProvenance: CapabilityFacts['trustedProvenance'] = 'external'): CapabilityEffect[] {
  if (trustedProvenance === 'external') {
    if (annotations?.destructiveHint) return ['external_mutation']
    if (annotations?.openWorldHint) return ['network_read', 'external_mutation']
    return ['workspace_read']
  }
  if (annotations?.readOnlyHint && toolName !== 'web_search') return toolName.startsWith('git_') ? ['git_read'] : ['workspace_read']
  if (toolName === 'web_search') return ['network_read']
  if (toolName === 'http_fetch') return ['network_read', 'network_write', 'external_mutation']
  if (toolName === 'terminal_exec') return ['process_exec', 'workspace_write', 'network_read', 'external_mutation']
  if (toolName === 'file_write' || toolName === 'file_edit' || toolName === 'apply_patch') return ['workspace_write']
  return annotations?.destructiveHint ? ['workspace_write'] : ['workspace_read']
}
