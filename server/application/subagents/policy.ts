import { resolve, relative, sep } from 'node:path'
import { existsSync, realpathSync } from 'node:fs'
import type { SubagentAuthority, SubagentBudget, SubagentProfile, SubagentWorkingMode, SubagentEffect } from '../../../shared/types/subagents.ts'

const EFFECTS: SubagentEffect[] = ['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge']
const MODES: SubagentWorkingMode[] = ['read-only', 'workspace']

export const SUBAGENT_BUDGET_LIMITS: SubagentBudget = {
  max_turns: 20,
  max_tool_calls: 40,
  max_output_tokens: 4096,
  max_context_tokens: 8192,
  max_wall_time_ms: 180000,
  max_depth: 1
}

export function intersectSubagentAuthority(parent: SubagentAuthority, profile: SubagentProfile, operator: Partial<SubagentAuthority> = {}): SubagentAuthority {
  if (!parent.workspace_root || !parent.workspace_root.startsWith('/')) throw new Error('invalid workspace scope')
  const parentEffects = new Set(parent.effects)
  const operatorEffects = new Set(operator.effects ?? EFFECTS)
  const allowedProfileEffects = new Set(profile.effects.allow)
  const denied = new Set([...profile.effects.deny])
  const effects = EFFECTS.filter(effect => parentEffects.has(effect) && operatorEffects.has(effect) && allowedProfileEffects.has(effect) && !denied.has(effect))
  const parentTools = new Set(parent.tools)
  const operatorTools = new Set(operator.tools ?? parent.tools)
  const tools = [...parentTools].filter((tool) => {
    const canonical = tool.includes('.') ? tool.split('.').at(-1) ?? tool : tool
    return operatorTools.has(tool) && profile.tools.allow.includes(canonical) && !profile.tools.deny.includes(canonical)
  })
  const working_mode = parent.working_mode === 'read-only' || profile.working_mode === 'read-only' || operator.working_mode === 'read-only' ? 'read-only' : 'workspace'
  const model_policy = parent.model_policy === 'fast' || operator.model_policy === 'fast' ? 'fast' : profile.model_policy === 'strong' && parent.model_policy === 'strong' ? 'strong' : 'default'
  return { tools, effects, working_mode: MODES.includes(working_mode) ? working_mode : 'read-only', model_policy, workspace_root: parent.workspace_root }
}

export function narrowBudget(profile: SubagentProfile, requested: Partial<SubagentBudget> = {}): SubagentBudget {
  const configured = {
    max_turns: profile.max_turns,
    max_tool_calls: profile.max_tool_calls,
    max_output_tokens: profile.max_output_tokens,
    max_context_tokens: profile.max_context_tokens,
    max_wall_time_ms: profile.max_wall_time_ms,
    max_depth: profile.max_depth
  }
  return Object.fromEntries(Object.keys(configured).map((key) => {
    const name = key as keyof SubagentBudget
    const value = requested[name]
    if (value !== undefined && (!Number.isSafeInteger(value) || value < 0)) throw new Error('invalid subagent budget')
    return [name, Math.min(configured[name], SUBAGENT_BUDGET_LIMITS[name], value ?? configured[name])]
  })) as SubagentBudget
}

export function assertContainedChildPath(root: string, candidate: string): string {
  const resolvedRoot = existsSync(root) ? realpathSync(root) : resolve(root)
  const lexical = resolve(root, candidate)
  const resolved = existsSync(lexical) ? realpathSync(lexical) : lexical
  const outside = relative(resolvedRoot, resolved).startsWith('..') || relative(resolvedRoot, resolved).includes(`..${sep}`)
  if (outside) throw new Error('child workspace escapes execution root')
  return resolved
}
