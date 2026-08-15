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

  return {
    port,
    isConnected,
    isConnecting,
    error,
    checkConnection,
    exec
  }
}
