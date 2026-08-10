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
  const sessionCredential = useState<string | null>('relay-agent-session-credential', () => {
    if (import.meta.client) {
      return localStorage.getItem('relay_agent_session_credential')
    }
    return null
  })

  const port = useState<number>('relay-agent-port', () => 47821)
  const isConnected = ref(false)
  const isConnecting = ref(false)
  const error = ref<string | null>(null)
  let ws: WebSocket | null = null

  const pendingExecs = new Map<string, { resolve: (res: RelayExecResult) => void, reject: (err: Error) => void }>()

  function setSessionCredential(cred: string | null) {
    sessionCredential.value = cred
    if (import.meta.client) {
      if (cred) {
        localStorage.setItem('relay_agent_session_credential', cred)
      } else {
        localStorage.removeItem('relay_agent_session_credential')
      }
    }
  }

  async function pair(token: string): Promise<boolean> {
    error.value = null
    try {
      const res = await $fetch<{ sessionCredential?: string, error?: string }>(`http://127.0.0.1:${port.value}/pair`, {
        method: 'POST',
        body: { token }
      })

      if (res.sessionCredential) {
        setSessionCredential(res.sessionCredential)
        return await connect()
      } else {
        error.value = res.error || 'Pairing failed'
        return false
      }
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to reach local relay agent'
      return false
    }
  }

  async function connect(): Promise<boolean> {
    if (!sessionCredential.value) {
      error.value = 'Not paired with local relay agent'
      return false
    }

    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
      return true
    }

    isConnecting.value = true
    error.value = null

    return new Promise((resolve) => {
      try {
        const socket = new WebSocket(`ws://127.0.0.1:${port.value}?credential=${sessionCredential.value}`)

        socket.onopen = () => {
          ws = socket
          isConnected.value = true
          isConnecting.value = false
          resolve(true)
        }

        socket.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data) as RelayExecResult
            if (data.type === 'exec_result' && data.id && pendingExecs.has(data.id)) {
              const pending = pendingExecs.get(data.id)!
              pendingExecs.delete(data.id)
              pending.resolve(data)
            }
          } catch {
            // Ignore non-JSON or unhandled messages
          }
        }

        socket.onerror = () => {
          isConnected.value = false
          isConnecting.value = false
          error.value = 'WebSocket connection error'
          resolve(false)
        }

        socket.onclose = () => {
          isConnected.value = false
          isConnecting.value = false
          ws = null
        }
      } catch (err: unknown) {
        isConnecting.value = false
        error.value = (err as Error).message
        resolve(false)
      }
    })
  }

  function disconnect() {
    if (ws) {
      ws.close()
      ws = null
    }
    isConnected.value = false
  }

  async function exec(command: string, args: string[] = [], cwd?: string): Promise<RelayExecResult> {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      const connected = await connect()
      if (!connected || !ws) {
        throw new Error('Local relay agent is not connected')
      }
    }

    const id = Math.random().toString(36).substring(2, 9)

    return new Promise((resolve, reject) => {
      pendingExecs.set(id, { resolve, reject })
      ws!.send(JSON.stringify({ type: 'exec', id, command, args, cwd }))
    })
  }

  return {
    sessionCredential,
    port,
    isConnected,
    isConnecting,
    error,
    pair,
    connect,
    disconnect,
    exec,
    setSessionCredential
  }
}
