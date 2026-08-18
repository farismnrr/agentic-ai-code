import { parse } from 'yaml'
import { SUBAGENT_PROFILES, type SubagentEffect, type SubagentProfile, type SubagentProfileName } from '../../../shared/types/subagents.ts'
import { SUBAGENT_BUDGET_LIMITS } from './policy.ts'

const MAX_PROFILE_BYTES = 64 * 1024
const MAX_LIST = 32
const TOOL_NAMES = new Set(['directory_list', 'file_search', 'text_search', 'file_read', 'git_status', 'git_diff', 'git_log', 'git_show', 'git_blame', 'code_symbols', 'code_definition', 'code_references', 'code_hover', 'code_diagnostics', 'code_rename_preview', 'web_search', 'http_fetch', 'terminal_exec', 'local_terminal', 'file_write', 'file_edit', 'apply_patch'])
const EFFECT_NAMES = new Set<SubagentEffect>(['workspace_read', 'workspace_write', 'workspace_delete', 'git_read', 'process_exec', 'network_read', 'network_write', 'external_mutation', 'privileged_bridge'])
const NAME_RE = /^[a-z][a-z0-9-]{0,31}$/

function assertBoundedString(value: unknown, field: string, max: number): asserts value is string {
  if (typeof value !== 'string' || value.length === 0 || value.length > max) throw new Error(`invalid profile ${field}`)
}

export function parseAgentProfile(source: string): SubagentProfile {
  if (Buffer.byteLength(source, 'utf8') > MAX_PROFILE_BYTES) throw new Error('profile exceeds size limit')
  if (!source.startsWith('---\n')) throw new Error('profile frontmatter is required')
  const end = source.indexOf('\n---', 4)
  if (end < 0) throw new Error('profile frontmatter is malformed')
  const raw = parse(source.slice(4, end)) as Record<string, unknown>
  const instructions = source.slice(end + 4).trim()
  assertBoundedString(raw.name, 'name', 32)
  if (!NAME_RE.test(raw.name) || !SUBAGENT_PROFILES.includes(raw.name as SubagentProfileName)) throw new Error('profile name is not allowed')
  assertBoundedString(raw.description, 'description', 512)
  if (!['fast', 'default', 'strong'].includes(raw.model_policy as string)) throw new Error('invalid model policy')
  if (!['read-only', 'workspace'].includes(raw.working_mode as string)) throw new Error('invalid working mode')
  if (instructions.length === 0 || instructions.length > 16 * 1024) throw new Error('invalid profile instructions')
  const tools = parseLists(raw.tools, 'tools', TOOL_NAMES)
  const effects = parseLists(raw.effects, 'effects', EFFECT_NAMES)
  const skills = parseStringList(raw.skills, 'skills')
  const budget = (field: keyof typeof SUBAGENT_BUDGET_LIMITS) => {
    const value = raw[field]
    if (!Number.isSafeInteger(value) || (value as number) < 1 || (value as number) > SUBAGENT_BUDGET_LIMITS[field]) throw new Error(`invalid profile ${field}`)
    return value as number
  }
  return { name: raw.name as SubagentProfileName, description: raw.description as string, model_policy: raw.model_policy as SubagentProfile['model_policy'], tools, effects, max_turns: budget('max_turns'), max_tool_calls: budget('max_tool_calls'), max_output_tokens: budget('max_output_tokens'), max_context_tokens: budget('max_context_tokens'), max_wall_time_ms: budget('max_wall_time_ms'), max_depth: budget('max_depth'), working_mode: raw.working_mode as SubagentProfile['working_mode'], skills, instructions }
}

function parseStringList(value: unknown, field: string): string[] {
  if (value === undefined) return []
  if (!Array.isArray(value) || value.length > MAX_LIST || value.some(item => typeof item !== 'string' || item.length > 128)) throw new Error(`invalid profile ${field}`)
  return [...new Set(value as string[])]
}

function parseLists(value: unknown, field: string, known: Set<string>): { allow: string[], deny: string[] } {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`invalid profile ${field}`)
  const record = value as Record<string, unknown>
  const allow = parseStringList(record.allow, `${field}.allow`)
  const deny = parseStringList(record.deny, `${field}.deny`)
  if ([...allow, ...deny].some(item => !known.has(item))) throw new Error(`unknown profile ${field}`)
  if (allow.some(item => deny.includes(item))) throw new Error(`conflicting profile ${field}`)
  return { allow, deny }
}

export function loadAgentProfile(name: string, readFile: (name: string) => string): SubagentProfile {
  if (!SUBAGENT_PROFILES.includes(name as SubagentProfileName)) throw new Error('unknown agent profile')
  return parseAgentProfile(readFile(name))
}

export function toolMatchesProfile(toolName: string, profile: SubagentProfile): boolean {
  const canonical = toolName.includes('.') ? toolName.split('.').at(-1) ?? toolName : toolName
  return profile.tools.allow.includes(canonical) && !profile.tools.deny.includes(canonical)
}
