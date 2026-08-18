export type ToolRenderCategory = 'read' | 'git' | 'mutation' | 'execution' | 'network' | 'subagent' | 'policy' | 'diagnostics' | 'unknown'

const MAX_PREVIEW_CHARS = 6000
const MAX_SUMMARY_ITEMS = 12
const HIDDEN_KEY = /(token|secret|password|passwd|cookie|authorization|api[-_]?key|credential|content|body|patch|diff|source|prompt|message|args|headers?)/i

function record(value: unknown): Record<string, unknown> | undefined {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? value as Record<string, unknown> : undefined
}

function safePath(value: unknown): string | undefined {
  if (typeof value !== 'string' || value.length === 0) return undefined
  const normalized = value.replaceAll('\\', '/')
  if (normalized.startsWith('/')) return normalized.split('/').filter(Boolean).slice(-3).join('/') || '/'
  return normalized.replace(/^\.\//, '').slice(0, 256)
}

function safeDomain(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined
  try {
    return new URL(value).hostname.toLowerCase().slice(0, 253)
  } catch {
    return undefined
  }
}

function safeExecutable(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined
  const first = value.trim().split(/\s+/)[0]
  return first?.split(/[\\/]/).pop()?.slice(0, 128)
}

function scalar(value: unknown): string | number | boolean | undefined {
  if (typeof value === 'boolean' || typeof value === 'number') return value
  if (typeof value === 'string' && value.length <= 160) return value
  return undefined
}

export function toolCategory(toolName: string): ToolRenderCategory {
  if (toolName === 'delegate_task' || toolName.startsWith('background_') || toolName.startsWith('agent_task_')) return 'subagent'
  if (toolName.startsWith('git_')) return 'git'
  if (toolName.startsWith('code_')) return 'diagnostics'
  if (['file_write', 'file_edit', 'apply_patch'].includes(toolName)) return 'mutation'
  if (['terminal_exec', 'terminal_job_start', 'terminal_job_get', 'terminal_job_cancel', 'local_terminal'].includes(toolName)) return 'execution'
  if (['http_fetch', 'web_search'].includes(toolName)) return 'network'
  if (toolName.includes('hook') || toolName.includes('approval') || toolName.includes('policy')) return 'policy'
  if (['directory_list', 'file_search', 'text_search', 'file_read'].includes(toolName)) return 'read'
  return 'unknown'
}

export function categoryLabel(category: ToolRenderCategory): string {
  return ({ read: 'Read / search', git: 'Git / review', mutation: 'File change', execution: 'Execution', network: 'Network', subagent: 'Agent task', policy: 'Policy / hook', diagnostics: 'Diagnostics', unknown: 'Tool' })[category]
}

export interface SafeInputSummary {
  rows: Array<{ label: string, value: string }>
  hiddenFields: number
}

/** Approval-safe summary: deliberately excludes argument arrays, bodies, patches, source, prompts and secret-shaped fields. */
export function safeInputSummary(input: unknown): SafeInputSummary {
  const value = record(input)
  if (!value) return { rows: [], hiddenFields: input == null ? 0 : 1 }
  const rows: Array<{ label: string, value: string }> = []
  let hiddenFields = 0
  for (const [key, raw] of Object.entries(value)) {
    if (rows.length >= MAX_SUMMARY_ITEMS) {
      hiddenFields += 1
      continue
    }
    if (HIDDEN_KEY.test(key)) {
      hiddenFields += 1
      continue
    }
    let shown: string | number | boolean | undefined
    if (/^(path|cwd|file|directory|root|worktree_path)$/i.test(key)) shown = safePath(raw)
    else if (/^(url|uri)$/i.test(key)) shown = safeDomain(raw)
    else if (/^(command|executable)$/i.test(key)) shown = safeExecutable(raw)
    else if (Array.isArray(raw)) shown = `${raw.length} item${raw.length === 1 ? '' : 's'}`
    else if (record(raw)) shown = `${Object.keys(raw as Record<string, unknown>).length} fields`
    else shown = scalar(raw)
    if (shown === undefined) {
      hiddenFields += 1
      continue
    }
    rows.push({ label: key.replaceAll('_', ' '), value: String(shown) })
  }
  return { rows, hiddenFields }
}

function previewCandidate(output: unknown): { label: string, text: string } | undefined {
  if (typeof output === 'string') return { label: 'Result preview', text: output }
  const value = record(output)
  if (!value) return undefined
  for (const key of ['diff', 'patch', 'text', 'content', 'summary']) {
    if (typeof value[key] === 'string') return { label: key === 'diff' || key === 'patch' ? 'Diff preview' : 'Result preview', text: value[key] as string }
  }
  return undefined
}

export interface OutputPresentation {
  summary: string
  preview?: string
  previewLabel?: string
  truncated: boolean
  continuation: boolean
}

export function presentToolOutput(output: unknown): OutputPresentation | undefined {
  if (output === undefined || output === null) return undefined
  const value = record(output)
  const continuation = Boolean(value?.continuation)
  const declaredTruncated = value?.truncated === true
  const candidate = previewCandidate(output)
  if (candidate) {
    const truncated = declaredTruncated || candidate.text.length > MAX_PREVIEW_CHARS
    return {
      summary: `${candidate.text.length.toLocaleString()} characters${continuation ? ' · continuation available' : ''}`,
      preview: candidate.text.slice(0, MAX_PREVIEW_CHARS),
      previewLabel: candidate.label,
      truncated,
      continuation
    }
  }
  if (Array.isArray(output)) return { summary: `${output.length} result item${output.length === 1 ? '' : 's'}`, truncated: false, continuation: false }
  if (value) {
    const keys = Object.keys(value).filter(key => !HIDDEN_KEY.test(key)).slice(0, 8)
    return { summary: `${Object.keys(value).length} result fields${keys.length ? ` · ${keys.join(', ')}` : ''}${continuation ? ' · continuation available' : ''}`, truncated: declaredTruncated, continuation }
  }
  return { summary: String(output).slice(0, 256), truncated: false, continuation: false }
}
