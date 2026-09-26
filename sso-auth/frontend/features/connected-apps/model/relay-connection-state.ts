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
