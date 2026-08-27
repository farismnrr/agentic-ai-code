import { friendlyRelayErrorMessage } from '../utils/chat-errors'
import { LOCAL_RELAY_PORT } from '#shared/utils/local-relay'

const MCP_PROTOCOL_VERSION = '2026-07-28'
const MCP_CLIENT_INFO = { name: 'AI Code', version: 'local-tool-controller' }

export interface RelayExecResult {
  type: 'exec_result'
  id?: string
  success: boolean
  approvalRequired?: boolean
  approvalToken?: string
  error?: string
  stdout?: string
  stderr?: string
  exitCode?: number
}

export type RelayExecutionMode = 'sync' | 'async' | 'auto'

export interface RelayExecOptions {
  timeoutMs?: number
  executionMode?: RelayExecutionMode
  idempotencyKey?: string
}

interface RelayToolCallResult {
  resultType?: 'complete' | 'task'
  taskId?: string
  status?: string
  pollIntervalMs?: number
  isError?: boolean
  _meta?: { control?: { type?: string, reason?: string, token?: string } }
  content?: Array<{ type?: string, text?: string }>
  result?: RelayToolCallResult
  error?: { message?: string }
}

export interface RelaySessionStartResult {
  isError?: boolean
  context?: { repository_identity?: string }
  bounded?: boolean
}

export function useRelayAgent() {
  // Shared Nuxt state keeps the connection indicator consistent between chat,
  // the tool picker, and Settings → MCP. The relay remains browser-local.
  const port = useState<number>('relay-agent-port', () => LOCAL_RELAY_PORT)
  const isConnected = useState<boolean>('relay-agent-connected', () => false)
  const isConnecting = useState<boolean>('relay-agent-connecting', () => false)
  const error = useState<string | null>('relay-agent-error', () => null)

  async function checkConnection(): Promise<boolean> {
    isConnecting.value = true
    try {
      const discovery = await mcpRequest<{ supportedVersions?: string[] }>('server/discover', '', {})
      if (!discovery.supportedVersions?.includes(MCP_PROTOCOL_VERSION)) {
        throw new Error('Local relay uses an incompatible MCP protocol version')
      }
      isConnected.value = true
      error.value = null
      return true
    } catch (err: unknown) {
      isConnected.value = false
      error.value = friendlyRelayErrorMessage(err)
      return false
    } finally {
      isConnecting.value = false
    }
  }

  function toExecResult(result: RelayToolCallResult): RelayExecResult {
    if (result._meta?.control?.type === 'approval_required') {
      return { type: 'exec_result', success: false, approvalRequired: true, approvalToken: result._meta.control.token, error: 'Approval is required before this action can continue' }
    }
    if (result.isError) {
      return { type: 'exec_result', success: false, error: friendlyRelayErrorMessage(result) }
    }

    const textContent = result.content?.find(c => c.type === 'text')?.text || ''
    let stdout: string
    let stderr = ''
    let exitCode = 0
    try {
      const parsed = JSON.parse(textContent)
      stdout = parsed.stdout || ''
      stderr = parsed.stderr || ''
      exitCode = parsed.exit_code ?? 0
    } catch {
      stdout = textContent
    }

    return {
      type: 'exec_result',
      success: exitCode === 0,
      stdout,
      stderr,
      exitCode
    }
  }

  async function awaitTaskResult(initial: RelayToolCallResult, agentSession?: string): Promise<RelayExecResult> {
    if (!initial.taskId) throw new Error('Relay task response is missing a task ID')
    let task = initial
    let pollIntervalMs = Math.min(Math.max(task.pollIntervalMs ?? 1000, 100), 5000)
    for (;;) {
      if (task.status === 'input_required') throw new Error('Relay task requires additional input')
      if (task.status !== 'working') {
        if (task.status === 'cancelled') return { type: 'exec_result', success: false, error: 'Relay task was cancelled' }
        if (task.status === 'failed') return { type: 'exec_result', success: false, error: friendlyRelayErrorMessage(task.error ?? task) }
        if (task.result) return toExecResult(task.result)
        return { type: 'exec_result', success: false, error: 'Relay task completed without a result' }
      }
      await new Promise(resolve => setTimeout(resolve, pollIntervalMs))
      task = await mcpRequest<RelayToolCallResult>('tasks/get', '', { taskId: initial.taskId }, agentSession)
      pollIntervalMs = Math.min(Math.max(task.pollIntervalMs ?? pollIntervalMs, 100), 5000)
    }
  }

  async function exec(command: string, args: string[] = [], cwd?: string, agentSession?: string, hookApprovalToken?: string, options: RelayExecOptions = {}): Promise<RelayExecResult> {
    const connected = await checkConnection()
    if (!connected) throw new Error('Local relay agent is not connected')

    try {
      const result = await mcpRequest<RelayToolCallResult>('tools/call', 'terminal_exec', {
        name: 'terminal_exec',
        arguments: {
          command,
          args,
          cwd,
          ...(options.timeoutMs !== undefined ? { timeout_ms: options.timeoutMs } : {}),
          execution_mode: options.executionMode ?? 'auto',
          ...(options.idempotencyKey ? { idempotency_key: options.idempotencyKey } : {})
        },
        ...(hookApprovalToken ? { _meta: { 'io.modelcontextprotocol/hookApprovalToken': hookApprovalToken } } : {})
      }, agentSession)
      if (result.resultType === 'task') return awaitTaskResult(result, agentSession)
      return toExecResult(result)
    } catch (err: unknown) {
      throw new Error(friendlyRelayErrorMessage(err), { cause: err })
    }
  }

  async function startSession(agentSession: string): Promise<RelaySessionStartResult> {
    const result = await mcpRequest<RelaySessionStartResult>('agent/session_start', '', {}, agentSession)
    if (result.isError) throw new Error('Relay security session start failed')
    return result
  }

  async function preAgentStop(agentSession: string): Promise<{ completion?: string }> {
    return mcpRequest<{ completion?: string }>('agent/pre_stop', '', {}, agentSession)
  }

  async function mcpRequest<T>(method: string, name: string, params: Record<string, unknown>, agentSession?: string): Promise<T> {
    const requestMeta = params._meta && typeof params._meta === 'object' && !Array.isArray(params._meta)
      ? params._meta as Record<string, unknown>
      : {}
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'mcp-protocol-version': MCP_PROTOCOL_VERSION,
      'mcp-method': method
    }
    if (name) headers['mcp-name'] = name

    const response = await $fetch<{ result?: T, error?: { message?: string } }>(`http://127.0.0.1:${port.value}/mcp`, {
      method: 'POST',
      headers,
      body: { jsonrpc: '2.0', id: Math.random().toString(36).slice(2), method, params: { ...params, _meta: { 'io.modelcontextprotocol/protocolVersion': MCP_PROTOCOL_VERSION, 'io.modelcontextprotocol/clientCapabilities': { extensions: { 'io.modelcontextprotocol/tasks': {} } }, 'io.modelcontextprotocol/clientInfo': MCP_CLIENT_INFO, ...requestMeta, ...(agentSession ? { 'io.modelcontextprotocol/agentSession': agentSession } : {}) } } }
    })
    if (response.error) throw new Error(response.error.message || 'Relay request failed')
    return response.result as T
  }

  return {
    port,
    isConnected,
    isConnecting,
    error,
    checkConnection,
    exec,
    startSession,
    preAgentStop
  }
}
