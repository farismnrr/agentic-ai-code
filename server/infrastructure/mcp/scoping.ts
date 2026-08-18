import type { ToolSet } from 'ai'

export type ToolApprovalValue = 'approved' | 'denied' | 'user-approval'
  | ((input: unknown) => 'approved' | 'denied' | 'user-approval' | Promise<'approved' | 'denied' | 'user-approval'>)

export type McpToolApprovalMap = Record<string, ToolApprovalValue>

export interface McpToolComposition {
  tools: ToolSet
  toolApproval: McpToolApprovalMap
  toolOwners: ReadonlyMap<string, string>
}

/** Claim a model-facing key only for one exact stable MCP tool owner. */
export function claimMcpToolOwner(owners: Map<string, string>, modelName: string, stableToolId: string): boolean {
  const previousOwner = owners.get(modelName)
  if (previousOwner !== undefined && previousOwner !== stableToolId) return false
  owners.set(modelName, stableToolId)
  return true
}

/** Scope the already-admitted MCP pair by stable IDs; never infer identity from model keys. */
export function scopeMcpTools(composition: McpToolComposition, allowedStableIds: ReadonlySet<string>): McpToolComposition {
  const retainedKeys = [...composition.toolOwners.entries()]
    .filter(([, stableId]) => allowedStableIds.has(stableId))
    .map(([modelName]) => modelName)
  const retained = new Set(retainedKeys)
  return {
    tools: Object.fromEntries(retainedKeys.map(name => [name, composition.tools[name]])) as ToolSet,
    toolApproval: Object.fromEntries(Object.entries(composition.toolApproval).filter(([name]) => retained.has(name))),
    toolOwners: new Map(retainedKeys.map(name => [name, composition.toolOwners.get(name) as string]))
  }
}
