import type { McpTool } from '../types/chat'

/** The one model-facing name transform used by MCP registration and UI lookup. */
export function mcpModelToolName(serverId: string, toolName: string) {
  return `${serverId}.${toolName}`.replace(/[^a-zA-Z0-9_-]/g, '_').slice(0, 64)
}

/** Resolve a model ToolSet key only when exactly one catalog entry owns it. */
export function resolveMcpToolFromModelName(modelName: string, tools: McpTool[]) {
  const matches = tools.filter(tool => mcpModelToolName(tool.serverId, tool.name) === modelName)
  return matches.length === 1 ? matches[0] : undefined
}
