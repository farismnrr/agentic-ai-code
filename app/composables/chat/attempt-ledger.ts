const STORAGE_KEY = 'relay_agent_executed_tool_calls'
const MAX_TRACKED_CALLS = 200

export function createAttemptLedger() {
  function hasAttempted(toolCallId: string) {
    if (!import.meta.client) return false
    try {
      return (JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]') as string[]).includes(toolCallId)
    } catch {
      return false
    }
  }
  function markAttempted(toolCallId: string) {
    if (!import.meta.client) return
    try {
      const ids = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]') as string[]
      localStorage.setItem(STORAGE_KEY, JSON.stringify([...ids, toolCallId].slice(-MAX_TRACKED_CALLS)))
    } catch {
      // Storage failures must not prevent the approved tool from reporting an outcome.
    }
  }
  return { hasAttempted, markAttempted }
}
