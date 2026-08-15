import { friendlyRelayErrorMessage } from '../utils/chat-errors'

export interface RelayExecResult {
  type: 'exec_result'
  id?: string
  success: boolean
  error?: string
  stdout?: string
  stderr?: string
  exitCode?: number
}

export interface RelayJobSnapshot {
  taskId: string
  status: string
  output?: { stdout?: string, stderr?: string, omittedBytes?: number, exitCode?: number }
  result?: { content?: Array<{ type?: string, text?: string }>, isError?: boolean }
}

export function useRelayAgent() {
  const port = ref<number>(47821)
  const isConnected = ref(false)
  const isConnecting = ref(false)
  const error = ref<string | null>(null)

  async function checkConnection(): Promise<boolean> {
    isConnecting.value = true
    try {
      await $fetch(`http://127.0.0.1:${port.value}/health`)
      isConnected.value = true
      error.value = null
      isConnecting.value = false
      return true
    } catch (err: unknown) {
      isConnected.value = false
      error.value = friendlyRelayErrorMessage(err)
      isConnecting.value = false
      return false
    }
  }

  async function exec(command: string, args: string[] = [], cwd?: string): Promise<RelayExecResult> {
    const connected = await checkConnection()
    if (!connected) {
      throw new Error('Local relay agent is not connected')
    }

    try {
      const payload = {
        jsonrpc: '2.0',
        id: Math.random().toString(36).substring(2, 9),
        method: 'tools/call',
        params: {
          name: 'terminal_exec',
          arguments: {
            command,
            args,
            cwd
          },
          _meta: {
            'io.modelcontextprotocol/protocolVersion': '2026-07-28',
            'io.modelcontextprotocol/clientCapabilities': {}
          }
        }
      }

      interface McpResponse {
        error?: { message?: string }
        result?: {
          isError?: boolean
          content?: Array<{ type?: string, text?: string }>
        }
      }

      const res = await $fetch<McpResponse>(`http://127.0.0.1:${port.value}/mcp`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'mcp-protocol-version': '2026-07-28',
          'mcp-method': 'tools/call',
          'mcp-name': 'terminal_exec'
        },
        body: payload
      })

      if (res.error) {
        return {
          type: 'exec_result',
          success: false,
          error: friendlyRelayErrorMessage(res.error)
        }
      }

      if (res.result && res.result.isError) {
        return {
          type: 'exec_result',
          success: false,
          error: friendlyRelayErrorMessage(res.result)
        }
      }

      const textContent = res.result?.content?.find(c => c.type === 'text')?.text || ''
      let stdout = ''
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
    } catch (err: unknown) {
      throw new Error(friendlyRelayErrorMessage(err), { cause: err })
    }
  }

  async function mcpRequest<T>(method: string, name: string, params: Record<string, unknown>): Promise<T> {
    const response = await $fetch<{ result?: T, error?: { message?: string } }>(`http://127.0.0.1:${port.value}/mcp`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'mcp-protocol-version': '2026-07-28', 'mcp-method': method, 'mcp-name': name },
      body: { jsonrpc: '2.0', id: Math.random().toString(36).slice(2), method, params: { ...params, _meta: { 'io.modelcontextprotocol/protocolVersion': '2026-07-28', 'io.modelcontextprotocol/clientCapabilities': { extensions: { 'io.modelcontextprotocol/tasks': {} } } } } }
    })
    if (response.error) throw new Error(response.error.message || 'Relay request failed')
    return response.result as T
  }

  async function fallbackJobCall(name: 'terminal_job_start' | 'terminal_job_get' | 'terminal_job_cancel', arguments_: Record<string, unknown>): Promise<RelayJobSnapshot> {
    const result = await mcpRequest<{ content?: Array<{ type?: string, text?: string }>, isError?: boolean }>('tools/call', name, { name, arguments: arguments_ })
    if (result.isError) throw new Error('Relay job request failed')
    const text = result.content?.find(item => item.type === 'text')?.text
    if (!text) throw new Error('Relay job response is missing content')
    return JSON.parse(text) as RelayJobSnapshot
  }

  async function startJob(command: string, args: string[] = [], cwd?: string): Promise<string> {
    await checkConnection()
    const result = await fallbackJobCall('terminal_job_start', { command, args, cwd })
    if (!result.taskId) throw new Error('Relay did not return a task ID')
    return result.taskId
  }

  async function getJob(taskId: string): Promise<RelayJobSnapshot> {
    return fallbackJobCall('terminal_job_get', { taskId })
  }

  async function cancelJob(taskId: string): Promise<void> {
    await fallbackJobCall('terminal_job_cancel', { taskId })
  }

  return {
    port,
    isConnected,
    isConnecting,
    error,
    checkConnection,
    exec,
    startJob,
    getJob,
    cancelJob
  }
}
