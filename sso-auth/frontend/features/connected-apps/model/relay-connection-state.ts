export type RelayConnectionState =
  | { status: 'not_connected' }
  | { status: 'connected'; connectionId: string }
  | { status: 'failed' }

const STORAGE_KEY = 'masih-awam.relay-connection'

export function loadRelayConnectionState(): RelayConnectionState {
  try {
    const raw = window.sessionStorage.getItem(STORAGE_KEY)
    if (!raw) return { status: 'not_connected' }

    const parsed = JSON.parse(raw) as Partial<RelayConnectionState>
    if (
      parsed.status === 'connected'
      && 'connectionId' in parsed
      && typeof parsed.connectionId === 'string'
      && parsed.connectionId
    ) {
      return { status: 'connected', connectionId: parsed.connectionId }
    }
  } catch {
    // Browser storage is optional; fall back to the disconnected UI state.
  }

  return { status: 'not_connected' }
}

export function saveRelayConnection(connectionId: string): RelayConnectionState {
  const state: RelayConnectionState = { status: 'connected', connectionId }
  try {
    window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state))
  } catch {
    // The current page can still show the verified connection without storage.
  }
  return state
}

export function clearRelayConnection(): RelayConnectionState {
  try {
    window.sessionStorage.removeItem(STORAGE_KEY)
  } catch {
    // Ignore storage failures and still update the in-memory UI state.
  }
  return { status: 'not_connected' }
}

export type RelayConnectionResult = {
  state: RelayConnectionState
  message: string
  error: string
}

export function consumeRelayConnectionResult(): RelayConnectionResult | null {
  const url = new URL(window.location.href)
  const result = url.searchParams.get('relay_connection')
  if (!result) return null

  const connectionId = url.searchParams.get('connection_id')
  let outcome: RelayConnectionResult | null = null

  if (result === 'connected' && connectionId) {
    outcome = {
      state: saveRelayConnection(connectionId),
      message: 'Relay connected successfully.',
      error: ''
    }
  } else if (result === 'failed') {
    clearRelayConnection()
    outcome = {
      state: { status: 'failed' },
      message: '',
      error: 'Relay connection failed. Try connecting again.'
    }
  }

  url.searchParams.delete('relay_connection')
  url.searchParams.delete('connection_id')
  window.history.replaceState({}, '', `${url.pathname}${url.search}${url.hash}`)
  return outcome
}
