const DEFAULT_MCP_REQUEST_TIMEOUT_MS = 45_000
const MIN_MCP_REQUEST_TIMEOUT_MS = 1_000
const MAX_MCP_REQUEST_TIMEOUT_MS = 120_000
const MIN_TASK_POLL_MS = 250
const MAX_TASK_POLL_MS = 5_000
const DEFAULT_TASK_POLL_MS = 500

export type McpTaskEnvelope = Record<string, unknown> & {
  resultType: 'task'
  taskId: string
}

export class McpRoundTripTimeoutError extends Error {
  constructor() {
    super('Remote MCP request timed out')
    this.name = 'TimeoutError'
  }
}

export function resolveMcpRequestTimeoutMs(value: unknown) {
  if (value === undefined || value === null || value === '') return DEFAULT_MCP_REQUEST_TIMEOUT_MS
  const parsed = typeof value === 'number' ? value : Number(value)
  if (!Number.isSafeInteger(parsed) || parsed < MIN_MCP_REQUEST_TIMEOUT_MS || parsed > MAX_MCP_REQUEST_TIMEOUT_MS) {
    throw new Error('Remote MCP request timeout configuration is invalid')
  }
  return parsed
}

export function asMcpTaskEnvelope(value: unknown): McpTaskEnvelope | undefined {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return undefined
  const task = value as Record<string, unknown>
  if (task.resultType !== 'task' || typeof task.taskId !== 'string' || task.taskId.length === 0 || task.taskId.length > 256) return undefined
  return task as McpTaskEnvelope
}

export function mcpRoutingName(method: string, params: Record<string, unknown>) {
  const value = method === 'tools/call'
    ? params.name
    : method.startsWith('tasks/')
      ? params.taskId
      : undefined
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

export function taskPollDelayMs(task: Record<string, unknown>, previousDelayMs?: number) {
  const hinted = task.pollIntervalMs
  if (typeof hinted === 'number' && Number.isFinite(hinted) && hinted > 0) {
    return Math.min(MAX_TASK_POLL_MS, Math.max(MIN_TASK_POLL_MS, Math.round(hinted)))
  }
  const previous = typeof previousDelayMs === 'number' && Number.isFinite(previousDelayMs) && previousDelayMs > 0
    ? previousDelayMs
    : DEFAULT_TASK_POLL_MS
  return Math.min(MAX_TASK_POLL_MS, Math.max(MIN_TASK_POLL_MS, Math.round(previous * 1.5)))
}

export async function fetchWithMcpDeadline(
  fetchImpl: typeof fetch,
  input: RequestInfo | URL,
  init: RequestInit,
  timeoutMs: number
) {
  const controller = new AbortController()
  const upstreamSignal = init.signal
  const abortFromUpstream = () => controller.abort(upstreamSignal?.reason)
  if (upstreamSignal?.aborted) abortFromUpstream()
  else upstreamSignal?.addEventListener('abort', abortFromUpstream, { once: true })

  let timedOut = false
  const timer = setTimeout(() => {
    timedOut = true
    controller.abort()
  }, timeoutMs)

  try {
    return await fetchImpl(input, { ...init, signal: controller.signal })
  } catch (error) {
    if (timedOut) throw new McpRoundTripTimeoutError()
    throw error
  } finally {
    clearTimeout(timer)
    upstreamSignal?.removeEventListener('abort', abortFromUpstream)
  }
}
