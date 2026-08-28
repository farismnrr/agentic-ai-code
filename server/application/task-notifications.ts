import type { OrchestratorGraphSnapshot } from '../../shared/types/orchestration.ts'

export const TASK_NOTIFICATION_CONTRACT_VERSION = '1' as const
export const TASK_NOTIFICATION_LIMITS = {
  taskId: 128,
  title: 160,
  summary: 2000,
  resultUrl: 2048,
  message: 4096
} as const

export type TaskCompletionSource = 'nuxt' | 'external-mcp'

export interface TaskCompletionInput {
  source: TaskCompletionSource
  taskId: string
  title: string
  summary: string
  completedAt?: string
  resultUrl?: string
}

export interface SanitizedTaskCompletion extends Omit<TaskCompletionInput, 'completedAt' | 'resultUrl'> {
  contractVersion: typeof TASK_NOTIFICATION_CONTRACT_VERSION
  completedAt: string
  resultUrl?: string
}

export interface TaskCompletionNotificationPort {
  enqueue(input: TaskCompletionInput): Promise<void>
}

export function taskCompletionInputForGraph(graph: OrchestratorGraphSnapshot): TaskCompletionInput {
  const completedNodes = graph.nodes.filter(node => node.status === 'completed').length
  return {
    source: 'nuxt',
    taskId: graph.graph_id,
    title: 'Implementation task completed',
    summary: `Completed the implementation plan with ${completedNodes} of ${graph.nodes.length} nodes settled successfully.`
  }
}

// ANSI/control bytes are intentionally matched here so notification text
// cannot carry terminal formatting or hidden control characters.
// eslint-disable-next-line no-control-regex
const ANSI_SEQUENCE = /\u001B(?:\][^\u0007]*(?:\u0007|\u001B\\)|\[[0-?]*[ -/]*[@-~])/g
// eslint-disable-next-line no-control-regex
const CONTROL_CHARACTER = /[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/g
const BEARER_CREDENTIAL = /\b(Bearer)\s+[^\s,;]+/gi
const CREDENTIAL_ASSIGNMENT = /\b(authorization|cookie|password|passwd|secret|token|api[_-]?key)\s*([:=])\s*(?:(Bearer)\s+)?[^\s,;]+/gi
const ENV_CREDENTIAL_ASSIGNMENT = /\b[A-Z][A-Z0-9_]*(?:TOKEN|KEY|SECRET|PASSWORD|CREDENTIAL)[A-Z0-9_]*\s*=\s*[^\s,;]+/g

function cleanText(value: string) {
  return value
    .replace(ANSI_SEQUENCE, '')
    .replace(CONTROL_CHARACTER, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

function redactText(value: string) {
  return cleanText(value)
    .replace(BEARER_CREDENTIAL, '$1 [REDACTED]')
    .replace(CREDENTIAL_ASSIGNMENT, (_match, key: string, delimiter: string, bearer?: string) => `${key}${delimiter}${bearer ? ` ${bearer} ` : ''}[REDACTED]`)
    .replace(ENV_CREDENTIAL_ASSIGNMENT, match => `${match.slice(0, match.indexOf('='))}=[REDACTED]`)
}

function requireBoundedText(field: 'taskId' | 'title' | 'summary', value: unknown, max: number) {
  if (typeof value !== 'string') throw new Error(`${field} is required`)
  const cleaned = redactText(value)
  if (!cleaned || cleaned.length > max) throw new Error(`${field} is invalid`)
  return cleaned
}

function normalizeTimestamp(value: unknown) {
  if (value === undefined) return new Date().toISOString()
  if (typeof value !== 'string' || !value.trim()) throw new Error('completedAt is invalid')
  const timestamp = new Date(value)
  if (Number.isNaN(timestamp.getTime())) throw new Error('completedAt is invalid')
  return timestamp.toISOString()
}

function normalizeResultUrl(value: unknown) {
  if (value === undefined) return undefined
  if (typeof value !== 'string' || value.length > TASK_NOTIFICATION_LIMITS.resultUrl) throw new Error('resultUrl is invalid')
  let url: URL
  try {
    url = new URL(value)
  } catch {
    throw new Error('resultUrl is invalid')
  }
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash || url.href !== value) throw new Error('resultUrl is invalid')
  return value
}

export function sanitizeTaskCompletion(input: TaskCompletionInput): SanitizedTaskCompletion {
  if (!input || typeof input !== 'object' || !['nuxt', 'external-mcp'].includes(input.source)) throw new Error('source is invalid')
  const resultUrl = normalizeResultUrl(input.resultUrl)
  return {
    contractVersion: TASK_NOTIFICATION_CONTRACT_VERSION,
    source: input.source,
    taskId: requireBoundedText('taskId', input.taskId, TASK_NOTIFICATION_LIMITS.taskId),
    title: requireBoundedText('title', input.title, TASK_NOTIFICATION_LIMITS.title),
    summary: requireBoundedText('summary', input.summary, TASK_NOTIFICATION_LIMITS.summary),
    completedAt: normalizeTimestamp(input.completedAt),
    ...(resultUrl ? { resultUrl } : {})
  }
}

function truncateUtf8(value: string, maxBytes: number) {
  if (new TextEncoder().encode(value).byteLength <= maxBytes) return value
  let result = value.slice(0, maxBytes)
  while (result && new TextEncoder().encode(result).byteLength > maxBytes) result = result.slice(0, -1)
  return result.trimEnd()
}

export function formatTaskCompletionMessage(event: SanitizedTaskCompletion) {
  const resultLine = event.resultUrl ? `\nResult: ${event.resultUrl}` : ''
  const prefix = `✅ ${event.title}\n`
  const available = TASK_NOTIFICATION_LIMITS.message - new TextEncoder().encode(`${prefix}${resultLine}`).byteLength
  const summary = truncateUtf8(event.summary, Math.max(0, available))
  let message = `${prefix}${summary}${resultLine}`
  if (new TextEncoder().encode(message).byteLength <= TASK_NOTIFICATION_LIMITS.message) return message
  message = `${prefix}${truncateUtf8(event.summary, TASK_NOTIFICATION_LIMITS.message - new TextEncoder().encode(prefix).byteLength)}`
  return truncateUtf8(message, TASK_NOTIFICATION_LIMITS.message)
}

export function completionTransitionWasNewlyReached(previous: OrchestratorGraphSnapshot['status'], next: OrchestratorGraphSnapshot['status']) {
  return previous === 'active' && next === 'completed'
}
