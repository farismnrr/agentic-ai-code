export interface RelayExecResult {
  type: 'exec_result'
  id?: string
  success: boolean
  error?: string
  stdout?: string
  stderr?: string
  exitCode?: number
}

// `crypto.randomUUID()` only exists in "secure contexts" (HTTPS, or
// localhost) per the Web Crypto spec — confirmed breaking in production via
// Loki (`crypto.randomUUID is not a function`, repeated ~20x from
// `vue.errorHandler`) when this app was reached over plain `http://` on a
// non-localhost address (a Tailscale IP in the observed case, but any LAN
// IP over HTTP hits the same wall). `crypto.getRandomValues` has no such
// restriction, so build a UUID v4 from that instead of the convenience
// method — this is the one thing standing between a paired device ever
// getting a fingerprint at all and the whole pairing flow silently
// breaking depending on how the user reaches this page.
function generateDeviceFingerprint(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16))
  bytes[6] = (bytes[6]! & 0x0f) | 0x40 // version 4
  bytes[8] = (bytes[8]! & 0x3f) | 0x80 // variant 10
  const hex = Array.from(bytes, b => b.toString(16).padStart(2, '0')).join('')
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
}

export function useRelayAgent() {
  const sessionCredential = ref<string | null>(null)
  const deviceFingerprint = ref<string | null>(null)
  const port = ref<number>(47821)
  const isConnected = ref(false)
  const isConnecting = ref(false)
  const error = ref<string | null>(null)
  let ws: WebSocket | null = null

  const pendingExecs = new Map<string, { resolve: (res: RelayExecResult) => void, reject: (err: Error) => void }>()

  function rejectAllPending(reason: string) {
    for (const pending of pendingExecs.values()) {
      pending.reject(new Error(reason))
    }
    pendingExecs.clear()
  }

  // Properly initialize from localStorage on client-side mount
  onMounted(() => {
    if (import.meta.client) {
      const storedCred = localStorage.getItem('relay_agent_session_credential')
      if (storedCred) {
        sessionCredential.value = storedCred
      }
      let storedFingerprint = localStorage.getItem('relay_agent_device_fingerprint')
      if (!storedFingerprint) {
        storedFingerprint = generateDeviceFingerprint()
        localStorage.setItem('relay_agent_device_fingerprint', storedFingerprint)
      }
      deviceFingerprint.value = storedFingerprint
    }
  })

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

  async function checkServerRevocation(): Promise<boolean> {
    if (!deviceFingerprint.value) return false
    try {
      const devices = await $fetch<Array<{ fingerprint: string, revokedAt: string | null }>>('/api/devices')
      const matching = devices.find(d => d.fingerprint === deviceFingerprint.value)

      if (matching && matching.revokedAt) {
        // Device is revoked on the server
        setSessionCredential(null)
        disconnect()
        error.value = 'Device has been revoked on server'
        return true
      }
      return false
    } catch {
      // If server check fails or unauthenticated, ignore
      return false
    }
  }

  async function pair(token: string): Promise<boolean> {
    error.value = null
    try {
      // Note: Do NOT set manual 'Origin' header on client fetch calls (handled automatically by browser)
      const res = await $fetch<{ sessionCredential?: string, error?: string }>(`http://127.0.0.1:${port.value}/pair`, {
        method: 'POST',
        body: { token }
      })

      if (res.sessionCredential) {
        setSessionCredential(res.sessionCredential)

        // Ensure independent fingerprint exists
        if (!deviceFingerprint.value && import.meta.client) {
          deviceFingerprint.value = generateDeviceFingerprint()
          localStorage.setItem('relay_agent_device_fingerprint', deviceFingerprint.value)
        }

        // Register device metadata on Singapore server
        const deviceName = `${navigator.platform || 'Desktop'} Relay Agent (${port.value})`
        try {
          await $fetch('/api/devices', {
            method: 'POST',
            body: { name: deviceName, fingerprint: deviceFingerprint.value }
          })
        } catch (regErr: unknown) {
          error.value = `Paired locally, but failed to register device on server: ${(regErr as Error).message}`
          // Return false so user is aware registration failed
          return false
        }

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

    // Check revocation status from Singapore DB first
    const isRevoked = await checkServerRevocation()
    if (isRevoked) {
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
            // Ignore non-JSON
          }
        }

        socket.onerror = () => {
          isConnected.value = false
          isConnecting.value = false
          error.value = 'WebSocket connection error'
          rejectAllPending('WebSocket connection error')
          resolve(false)
        }

        socket.onclose = () => {
          isConnected.value = false
          isConnecting.value = false
          ws = null
          // Without this, a command sent right before the socket dropped
          // (laptop sleep, CLI killed, network hiccup) left its exec()
          // promise pending forever — for an AI-initiated call that hangs
          // the whole chat turn with no error ever surfacing. See plan 026
          // Phase 8/9.
          rejectAllPending('Local relay agent connection closed')
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

  async function unpair() {
    if (sessionCredential.value) {
      // Invalidate credential on CLI server
      try {
        await $fetch(`http://127.0.0.1:${port.value}/revoke`, {
          method: 'POST',
          body: { credential: sessionCredential.value }
        })
      } catch {
        // Ignore if CLI is unreachable
      }
    }
    setSessionCredential(null)
    disconnect()
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
      // Backstop for a socket that never fires close/error at all (a dead
      // connection some networks just go silent on) — matches the relay
      // agent's own longest command timeout (5 min, see
      // packages/relay-agent/src/server.ts's execTimeoutMs) plus margin, so
      // this never fires before a legitimate long-running command
      // (e.g. npm install) would have gotten its own answer first.
      const timeoutId = setTimeout(() => {
        pendingExecs.delete(id)
        reject(new Error('Local relay agent did not respond in time'))
      }, 310000)

      pendingExecs.set(id, {
        resolve: (res) => {
          clearTimeout(timeoutId)
          resolve(res)
        },
        reject: (err) => {
          clearTimeout(timeoutId)
          reject(err)
        }
      })
      ws!.send(JSON.stringify({ type: 'exec', id, command, args, cwd }))
    })
  }

  return {
    sessionCredential,
    deviceFingerprint,
    port,
    isConnected,
    isConnecting,
    error,
    pair,
    connect,
    disconnect,
    unpair,
    exec,
    setSessionCredential
  }
}
