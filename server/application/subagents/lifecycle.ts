export interface SubagentStopBridge {
  subagentStop?: (parentSessionId: string, childSessionId: string, status: string) => Promise<boolean>
}

export async function enforceSubagentStop(client: SubagentStopBridge | undefined, parentSessionId: string, childSessionId: string, status: string) {
  if (!client?.subagentStop) return false
  try {
    return (await client.subagentStop(parentSessionId, childSessionId, status)) === true
  } catch {
    return false
  }
}
