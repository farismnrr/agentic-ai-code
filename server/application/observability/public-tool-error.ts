import type { RequestTelemetryContext } from './contracts'

/** Stable MCP 200 tool-result adapter; diagnostics remain private telemetry. */
export function publicMcpToolFailure(telemetry: RequestTelemetryContext, name: string, cause: unknown) {
  telemetry.error('mcp.tool.call', 'mcp_tool_call_failed', cause, { 'tool.name': name })
  return { content: [{ type: 'text' as const, text: 'Tool execution failed' }], isError: true as const }
}
