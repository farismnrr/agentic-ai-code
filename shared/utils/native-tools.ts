export const NATIVE_LOCAL_TERMINAL_TOOL_ID = 'native.local_terminal'

// Keep recognizing this historical persisted ID so conversations created by
// the removed browser-local mode cannot accidentally turn it into a remote
// MCP tool. It is intentionally no longer part of a visible/native tool
// catalog and is never registered in the server-side Agent tool set.
const legacyNativeToolIds = new Set([NATIVE_LOCAL_TERMINAL_TOOL_ID])

export function isNativeToolId(toolId: string) {
  return legacyNativeToolIds.has(toolId)
}
