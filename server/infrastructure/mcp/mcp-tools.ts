import { SpanStatusCode } from '@opentelemetry/api'
import { logger } from '../observability/logger'
import { getTracer } from '../observability/otel'
import { recordSanitizedException } from '../observability/exception'
import { tool, jsonSchema, type ToolSet } from 'ai'
import type { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { loadEnabledMcpServers } from './server-config'
import { createMcpClient } from './client'

const TRACER_NAME = 'ai-code-server'

// Infrastructure-layer outbound MCP boundary span helper. `server/infrastructure/**`
// is allowed to touch OTel directly (unlike application/core); a full
// `RequestTelemetryContext` isn't threaded this deep into the chat
// dependency chain today, so this uses the same start/end/record-exception
// shape directly against the tracer, matching Plan 035 Phase 6 item 5.
async function withMcpSpan<T>(operation: string, attributes: Record<string, unknown>, fn: () => Promise<T>): Promise<T> {
  const tracer = getTracer(TRACER_NAME)
  return tracer.startActiveSpan(operation, { attributes: attributes as Record<string, string | number | boolean> }, async (span) => {
    try {
      const result = await fn()
      span.end()
      return result
    } catch (err) {
      recordSanitizedException(span, err)
      span.setStatus({ code: SpanStatusCode.ERROR })
      span.end()
      throw err
    }
  }) as Promise<T>
}

// Plain map (not `ToolApprovalConfiguration<ToolSet, never>`, despite this
// being exactly what `toolApproval` ends up passed as into `streamText`):
// that generic type is a union of "one function for every tool call" OR "an
// object literal keyed by the tool set's own literal key type" — assigning
// into it key-by-key (`toolApproval['terminal'] = ...`, done by
// server/api/chat.post.ts for tools this function doesn't know about,
// e.g. the native terminal/local_terminal tools) doesn't typecheck against
// that union. Every real value ever stored here is one of these two shapes;
// callers cast to `ToolApprovalConfiguration<ToolSet, never>` only at the
// `streamText`/`generateText` call site, once nothing further mutates it.
type ToolApprovalValue = 'approved' | 'denied' | 'user-approval'
  | ((input: never) => 'approved' | 'denied' | 'user-approval' | Promise<'approved' | 'denied' | 'user-approval'>)

/**
 * OpenAI-shaped tool names must match /^[a-zA-Z0-9_-]{1,64}$/ — MCP tool ids
 * are `${serverId}.${toolName}` (see shared/types/chat.ts McpTool.id), which
 * contains a dot, so the model-facing name is a sanitized, truncated form
 * with a reverse lookup back to the real MCP call.
 */
function sanitizeToolName(mcpToolId: string) {
  return mcpToolId.replace(/[^a-zA-Z0-9_-]/g, '_').slice(0, 64)
}

/**
 * Builds the `tools` + `toolApproval` options for `streamText`, from a
 * conversation's `enabledToolIds` (`McpTool['id']` values, i.e.
 * `${serverId}.${toolName}`) resolved against the user's stored
 * `mcp_servers` rows — per plan 012 Phase 2. Connections are opened here and
 * must be closed via the returned `close()` once the stream finishes.
 */
export async function buildMcpTools(userId: string, enabledToolIds: string[], approvals: Record<string, 'always' | 'never'>) {
  const clients: Client[] = []
  const tools: ToolSet = {}
  const toolApproval: Record<string, ToolApprovalValue> = {}

  if (enabledToolIds.length === 0) {
    return { tools, toolApproval, close: async () => {} }
  }

  const serverIds = [...new Set(enabledToolIds.map(id => id.split('.')[0]).filter((id): id is string => Boolean(id)))]
  const servers = await loadEnabledMcpServers(userId, serverIds)

  for (const server of servers) {
    let client: Client
    try {
      client = await createMcpClient(server)
    } catch (err) {
      logger.error('[mcp-tools] failed to connect to configured server', err)
      continue
    }
    clients.push(client)

    let listed
    try {
      listed = await withMcpSpan('mcp.tools_list', {}, () => client.listTools())
    } catch (err) {
      logger.error('[mcp-tools] failed to list tools from configured server', err)
      continue
    }

    for (const mcpTool of listed.tools) {
      const mcpToolId = `${server.id}.${mcpTool.name}`
      if (!enabledToolIds.includes(mcpToolId)) continue

      const modelName = sanitizeToolName(mcpToolId)
      tools[modelName] = tool({
        description: mcpTool.description ?? '',
        inputSchema: jsonSchema(mcpTool.inputSchema as Record<string, unknown>),
        execute: async (input: unknown) => {
          const result = await withMcpSpan('mcp.tools_call', {}, () => client.callTool({ name: mcpTool.name, arguments: input as Record<string, unknown> }))
          return result.content
        }
      })

      const decision = approvals[mcpToolId]
      toolApproval[modelName] = decision === 'always' ? 'approved' : decision === 'never' ? 'denied' : 'user-approval'
    }
  }

  return {
    tools,
    toolApproval,
    close: async () => {
      await Promise.all(clients.map(c => c.close().catch((err: unknown) => logger.error('[mcp-tools] error closing client', err))))
    }
  }
}
