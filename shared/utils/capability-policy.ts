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

export type CapabilityMode = 'plan' | 'bypass' | 'manual'
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
  protectedBoundary?: boolean
  invalidInput?: boolean
  trustedProvenance?: 'first-party-relay' | 'native' | 'external'
  requiresConcreteScope?: boolean
}

export interface CapabilityAnnotations {
  readOnlyHint?: boolean
  destructiveHint?: boolean
  openWorldHint?: boolean
}

export interface CapabilityAssessment extends CapabilityFacts {
  risk: RiskLevel
  opaque: boolean
  reason: string
}

const SAFE_READ_TOOLS = new Set([
  'directory_list', 'file_search', 'text_search', 'file_read',
  'git_status', 'git_diff', 'git_log', 'git_show', 'git_blame', 'git_branch_list', 'git_operation_status', 'git_remote_list',
  'change_request_list', 'change_request_get', 'change_request_checks',
  'issue_list', 'issue_get',
  'workflow_list', 'workflow_get', 'workflow_run_list', 'workflow_run_get', 'workflow_run_jobs', 'workflow_job_log_preview',
  'dependabot_alert_list', 'dependabot_alert_get', 'code_scanning_alert_list', 'code_scanning_alert_get', 'secret_scanning_alert_list', 'secret_scanning_alert_get', 'secret_scanning_alert_locations', 'workflow_dispatch', 'workflow_run_rerun', 'workflow_run_cancel',
  'code_symbols', 'code_definition', 'code_references', 'code_hover',
  'code_diagnostics', 'code_rename_preview', 'web_search'
])

const OPAQUE_COMMANDS = new Set(['sh', 'bash', 'zsh', 'fish', 'dash', 'eval', 'env', 'xargs'])
const READ_COMMANDS = new Map([
  ['cat', 'workspace_read'], ['head', 'workspace_read'], ['tail', 'workspace_read'],
  ['ls', 'workspace_read'], ['pwd', 'workspace_read'], ['rg', 'workspace_read'],
  ['grep', 'workspace_read'], ['find', 'workspace_read'], ['git', 'git_read']
] as const)

const REVIEWED_STRUCTURED_TOOLS = new Set([
  ...SAFE_READ_TOOLS,
  'file_write', 'file_edit', 'apply_patch',
  'git_branch_create', 'git_branch_switch', 'git_stage', 'git_unstage', 'git_commit',
  'git_merge_start', 'git_merge_continue', 'git_merge_abort', 'git_rebase_start', 'git_rebase_continue', 'git_rebase_abort', 'git_branch_delete',
  'git_remote_branch_get', 'git_fetch', 'git_push', 'git_remote_branch_delete',
  'change_request_create', 'change_request_update', 'change_request_merge',
  'issue_create', 'issue_update', 'issue_comment', 'issue_close', 'issue_reopen',
  'workflow_dispatch', 'workflow_run_rerun', 'workflow_run_cancel',
  'http_fetch'
])

function inputRecord(input: unknown): Record<string, unknown> {
  return typeof input === 'object' && input !== null && !Array.isArray(input)
    ? input as Record<string, unknown>
    : {}
}

function inputString(input: Record<string, unknown>, key: string) {
  return typeof input[key] === 'string' ? input[key] as string : undefined
}

function inputArgs(input: Record<string, unknown>) {
  return Array.isArray(input.args)
    ? input.args.filter((arg): arg is string => typeof arg === 'string')
    : undefined
}

function inputDomain(input: Record<string, unknown>) {
  const url = inputString(input, 'url')
  if (!url) return undefined
  try {
    return new URL(url).hostname.toLowerCase()
  } catch {
    return undefined
  }
}

function hasString(input: Record<string, unknown>, key: string) {
  return typeof input[key] === 'string' && input[key] !== ''
}

function hasRequiredStrings(input: Record<string, unknown>, keys: string[]) {
  return keys.every(key => hasString(input, key))
}

function isProtectedPath(path: string | undefined, cwd?: string) {
  if (!path) return false
  const normalized = [cwd, path].filter(Boolean).join('/').replaceAll('\\', '/')
  const segments = normalized.split('/').filter(Boolean)
  return segments.some(segment => ['.ssh', '.aws', '.docker', '.kube'].includes(segment))
    || segments.some((segment, index) => segment === '.config' && ['gcloud', 'gh'].includes(segments[index + 1] ?? ''))
    || segments.some(segment => ['.npmrc', '.netrc', '.pypirc', '.git-credentials'].includes(segment))
    || segments.some((segment, index) => segment === '.cargo' && ['credentials', 'credentials.toml'].includes(segments[index + 1] ?? ''))
    || segments.some(segment => segment === '.env' || (segment.startsWith('.env.') && segment !== '.env.example'))
}

/** Extract only reviewed, top-level call facts. Arbitrary shell syntax is
 * intentionally not parsed here; opaque commands remain conservative. */
export function capabilityFactsForToolCall({
  toolId,
  toolName,
  input,
  annotations,
  trustedProvenance = 'external'
}: {
  toolId: string
  toolName: string
  input?: unknown
  annotations?: CapabilityAnnotations
  trustedProvenance?: CapabilityFacts['trustedProvenance']
}): CapabilityFacts {
  const values = inputRecord(input)
  const malformedInput = typeof input !== 'object' || input === null || Array.isArray(input)
    || ('path' in values && typeof values.path !== 'string')
    || ('cwd' in values && typeof values.cwd !== 'string')
    || (['file_read', 'file_write', 'file_edit', 'git_blame'].includes(toolName) && !hasString(values, 'path'))
    || (toolName === 'file_write' && !hasRequiredStrings(values, ['path', 'content']))
    || (toolName === 'file_edit' && !hasRequiredStrings(values, ['path', 'old_text', 'new_text']))
    || (toolName === 'apply_patch' && !hasString(values, 'patch'))
    || (toolName === 'git_show' && !hasString(values, 'ref'))
    || (['git_branch_create', 'git_branch_switch', 'git_branch_delete'].includes(toolName) && !hasString(values, 'name'))
    || (toolName === 'git_branch_create' && 'start_point' in values && !hasString(values, 'start_point'))
    || (['git_stage', 'git_unstage'].includes(toolName) && (!Array.isArray(values.paths) || values.paths.length === 0 || values.paths.some(path => typeof path !== 'string' || path === '')))
    || (toolName === 'git_commit' && !hasString(values, 'message'))
    || (['git_merge_start', 'git_rebase_start'].includes(toolName) && !hasString(values, 'ref'))
    || (['git_remote_branch_get', 'git_fetch', 'git_push', 'git_remote_branch_delete'].includes(toolName) && !hasString(values, 'branch'))
    || (toolName === 'git_remote_branch_delete' && !hasString(values, 'expected_sha'))
    || (['git_remote_branch_get', 'git_fetch', 'git_push', 'git_remote_branch_delete'].includes(toolName) && 'remote' in values && !hasString(values, 'remote'))
    || (['change_request_get', 'change_request_update', 'change_request_checks', 'change_request_merge'].includes(toolName) && (!Number.isInteger(values.number) || Number(values.number) <= 0))
    || (toolName === 'change_request_create' && !hasRequiredStrings(values, ['head_branch', 'base_branch', 'title', 'body']))
    || (toolName === 'change_request_merge' && !hasString(values, 'expected_head_sha'))
    || (['change_request_list', 'change_request_get', 'change_request_create', 'change_request_update', 'change_request_checks', 'change_request_merge'].includes(toolName) && 'remote' in values && !hasString(values, 'remote'))
    || (['issue_get', 'issue_update', 'issue_comment', 'issue_close', 'issue_reopen'].includes(toolName) && (!Number.isInteger(values.number) || Number(values.number) <= 0))
    || (toolName === 'issue_create' && !hasString(values, 'title'))
    || (toolName === 'issue_comment' && !hasString(values, 'body'))
    || (toolName === 'issue_close' && (!hasString(values, 'reason') || !['completed', 'not_planned', 'duplicate'].includes(values.reason as string)))
    || (['issue_list', 'issue_get', 'issue_create', 'issue_update', 'issue_comment', 'issue_close', 'issue_reopen'].includes(toolName) && 'remote' in values && !hasString(values, 'remote'))
    || (toolName === 'http_fetch' && !hasString(values, 'url'))
    || (toolName === 'web_search' && !hasString(values, 'query'))
    || (toolName === 'terminal_exec' && !hasString(values, 'command'))
  const path = inputString(values, 'path')
  const cwd = inputString(values, 'cwd')
  const effects = toolEffects(toolName, annotations, trustedProvenance)
  return {
    toolId,
    effects,
    path,
    domain: inputDomain(values),
    command: inputString(values, 'command'),
    args: inputArgs(values),
    networkRequested: toolName === 'http_fetch' || toolName === 'web_search' || toolName.startsWith('git_remote_') || toolName === 'git_fetch' || toolName === 'git_push' || toolName.startsWith('change_request_') || toolName.startsWith('issue_') || toolName.startsWith('workflow_')
      ? true
      : undefined,
    destructive: annotations?.destructiveHint,
    external: trustedProvenance === 'external',
    protectedBoundary: isProtectedPath(path, cwd),
    invalidInput: malformedInput,
    trustedProvenance,
    requiresConcreteScope: !REVIEWED_STRUCTURED_TOOLS.has(toolName)
  }
}

export function classifyCapability(facts: CapabilityFacts): CapabilityAssessment {
  const command = facts.command?.trim().split(/\s+/)[0]?.toLowerCase()
  const opaque = Boolean(command && OPAQUE_COMMANDS.has(command))
  const network = facts.networkRequested === true || facts.effects.some(effect => effect === 'network_read' || effect === 'network_write')
  const destructive = facts.destructive === true || facts.effects.some(effect => effect === 'workspace_delete' || effect === 'network_write' || effect === 'external_mutation' || effect === 'privileged_bridge')
  const lowRisk = facts.effects.every(effect => effect === 'workspace_read' || effect === 'git_read') && !network && !destructive && !opaque && facts.external !== true && facts.protectedBoundary !== true && facts.invalidInput !== true
  const risk: RiskLevel = facts.invalidInput === true || destructive || opaque || facts.external === true || facts.protectedBoundary === true ? 'high' : network || facts.effects.includes('process_exec') || facts.effects.includes('workspace_write') ? 'medium' : 'low'
  let reason = facts.invalidInput ? 'malformed capability input requires explicit review' : facts.protectedBoundary ? 'protected credential boundary requires explicit review' : opaque ? 'opaque shell or wrapper execution requires explicit review' : facts.external === true ? 'external tool provenance is untrusted and requires explicit review' : network ? 'network access is an independent capability' : destructive ? 'the operation can mutate or delete state' : facts.effects.includes('workspace_write') ? 'workspace mutation requires explicit review' : 'bounded read-only capability'

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
  // Hard safety boundaries are invariant across every user-facing permission
  // mode. "Bypass" means bypass product approval prompts; it never bypasses
  // malformed-input or protected-credential enforcement.
  if (assessment.invalidInput) return { outcome: 'denied', assessment }
  if (assessment.protectedBoundary) return { outcome: 'denied', assessment }
  if (mode === 'plan' && assessment.effects.some(effect => effect !== 'workspace_read' && effect !== 'git_read')) {
    return { outcome: 'denied', assessment: { ...assessment, reason: 'Plan mode permits read-only capabilities only' } }
  }
  if (mode === 'bypass') return { outcome: 'approved', assessment }
  if (remembered === 'never') return { outcome: 'denied', assessment }

  const trusted = facts.trustedProvenance === 'first-party-relay' || facts.trustedProvenance === 'native'
  if (remembered === 'always' && assessment.risk === 'low' && trusted && !assessment.opaque && !facts.requiresConcreteScope) return { outcome: 'approved', assessment }
  if (SAFE_READ_TOOLS.has(facts.toolId.split('.').pop() ?? '') && assessment.risk === 'low' && trusted) return { outcome: 'approved', assessment }
  return { outcome: 'user-approval', assessment }
}

export function rememberedApprovalCanAutoAnswer(
  facts: CapabilityFacts,
  decision: CapabilityApprovalDecision,
  mode: CapabilityMode = 'manual'
) {
  if (mode === 'bypass') return approvalForCapability(facts, decision, mode).outcome === 'approved'
  return decision === 'never' || approvalForCapability(facts, decision, mode).outcome === 'approved'
}

export function toolEffects(toolName: string, annotations?: CapabilityAnnotations, trustedProvenance: CapabilityFacts['trustedProvenance'] = 'external'): CapabilityEffect[] {
  if (toolName === 'terminal_exec') return ['process_exec', 'workspace_write', 'network_read', 'external_mutation']
  if (toolName === 'web_search') return ['network_read']
  if (toolName === 'http_fetch') return ['network_read', 'network_write', 'external_mutation']
  if (toolName === 'file_write' || toolName === 'file_edit' || toolName === 'apply_patch') return ['workspace_write']
  if (['git_branch_create', 'git_branch_switch', 'git_stage', 'git_unstage', 'git_commit', 'git_merge_start', 'git_merge_continue'].includes(toolName)) return ['workspace_write']
  if (['git_merge_abort', 'git_rebase_start', 'git_rebase_continue', 'git_rebase_abort', 'git_branch_delete'].includes(toolName)) return ['workspace_write', 'workspace_delete']
  if (toolName === 'git_remote_branch_get') return ['git_read', 'network_read']
  if (toolName === 'git_fetch') return ['git_read', 'workspace_write', 'network_read']
  if (toolName === 'git_push') return ['git_read', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge']
  if (toolName === 'git_remote_branch_delete') return ['git_read', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge']
  if (['change_request_list', 'change_request_get', 'change_request_checks', 'issue_list', 'issue_get', 'workflow_list', 'workflow_get', 'workflow_run_list', 'workflow_run_get', 'workflow_run_jobs', 'workflow_job_log_preview', 'dependabot_alert_list', 'dependabot_alert_get', 'code_scanning_alert_list', 'code_scanning_alert_get', 'secret_scanning_alert_list', 'secret_scanning_alert_get', 'secret_scanning_alert_locations'].includes(toolName)) return ['network_read', 'privileged_bridge']
  if (['change_request_create', 'change_request_update', 'change_request_merge', 'issue_create', 'issue_update', 'issue_comment', 'issue_close', 'issue_reopen', 'workflow_dispatch', 'workflow_run_rerun', 'workflow_run_cancel'].includes(toolName)) return ['network_read', 'network_write', 'external_mutation', 'privileged_bridge']
  if (SAFE_READ_TOOLS.has(toolName)) return toolName.startsWith('git_') ? ['git_read'] : ['workspace_read']
  if (trustedProvenance === 'external') {
    if (annotations?.destructiveHint) return ['external_mutation']
    if (annotations?.openWorldHint) return ['network_read', 'external_mutation']
    if (!REVIEWED_STRUCTURED_TOOLS.has(toolName) && annotations?.readOnlyHint !== true) return ['privileged_bridge']
    return ['workspace_read']
  }
  if (annotations?.readOnlyHint && toolName !== 'web_search') return toolName.startsWith('git_') ? ['git_read'] : ['workspace_read']
  return annotations?.destructiveHint ? ['workspace_write'] : ['privileged_bridge']
}

export function toolRequiresEffects(toolName: string, annotations?: CapabilityAnnotations, trustedProvenance: CapabilityFacts['trustedProvenance'] = 'external') {
  return toolEffects(toolName, annotations, trustedProvenance)
}
