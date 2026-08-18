import { SpanStatusCode } from '@opentelemetry/api'
import { logger } from '../observability/logger'
import { getTracer } from '../observability/otel'
import { recordSanitizedException } from '../observability/exception'
import { tool, jsonSchema, type ToolSet } from 'ai'
import { loadEnabledMcpServers } from './server-config'
import { createMcpClient, type McpClientLike } from './client'
import { approvalForCapability, capabilityFactsForToolCall, toolRequiresEffects } from '#shared/utils/capability-policy'
import { mcpModelToolName } from '#shared/utils/mcp-tool-identity'
import { enforceSubagentStop } from '../../application/subagents/lifecycle'

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
  | ((input: unknown) => 'approved' | 'denied' | 'user-approval' | Promise<'approved' | 'denied' | 'user-approval'>)

/**
 * OpenAI-shaped tool names must match /^[a-zA-Z0-9_-]{1,64}$/; the shared
 * identity helper is the reverse-lookup contract used by the approval UI.
 */
/**
 * Builds the `tools` + `toolApproval` options for `streamText`, from a
 * conversation's `enabledToolIds` (`McpTool['id']` values, i.e.
 * `${serverId}.${toolName}`) resolved against the user's stored
 * `mcp_servers` rows — per plan 012 Phase 2. Connections are opened here and
 * must be closed via the returned `close()` once the stream finishes.
 */
export async function buildMcpTools(userId: string, enabledToolIds: string[], approvals: Record<string, 'always' | 'never'>, permissionMode: 'plan' | 'workspace' | 'autonomous' | 'manual' = 'manual', options: { allowedEffects?: string[], maxToolCalls?: number, abortSignal?: AbortSignal } = {}) {
  const clients: McpClientLike[] = []
  const tools: ToolSet = {}
  const toolApproval: Record<string, ToolApprovalValue> = {}
  const modelToolOwners = new Map<string, string>()
  const allowedEffects = new Set(options.allowedEffects)
  let toolCalls = 0

  if (enabledToolIds.length === 0) {
    return { tools, toolApproval, close: async () => {}, toolCallCount: () => 0, subagentStop: async () => false }
  }

  const serverIds = [...new Set(enabledToolIds.map(id => id.split('.')[0]).filter((id): id is string => Boolean(id)))]
  const servers = await loadEnabledMcpServers(userId, serverIds)

  for (const server of servers) {
    let client: McpClientLike
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

      const modelName = mcpModelToolName(server.id, mcpTool.name)
      const trustedProvenance = client.trustedProvenance ?? 'external'
      const requiredEffects = toolRequiresEffects(mcpTool.name, mcpTool.annotations, trustedProvenance)
      if (requiredEffects.length === 0 || (options.allowedEffects && requiredEffects.some(effect => !allowedEffects.has(effect)))) continue
      const previousOwner = modelToolOwners.get(modelName)
      if (previousOwner && previousOwner !== mcpToolId) {
        logger.error('[mcp-tools] model tool identity collision; refusing ambiguous tool', { modelName })
        continue
      }
      modelToolOwners.set(modelName, mcpToolId)
      tools[modelName] = tool({
        description: mcpTool.description ?? '',
        inputSchema: jsonSchema(mcpTool.inputSchema),
        execute: async (input: unknown) => {
          if (options.maxToolCalls !== undefined && toolCalls >= options.maxToolCalls) throw new Error('subagent tool-call budget exhausted')
          toolCalls++
          const call = { name: mcpTool.name, arguments: input as Record<string, unknown> }
          const result = client.trustedProvenance === 'first-party-relay'
            ? await withMcpSpan('mcp.tools_call', {}, () => client.callTool(call, options.abortSignal))
            : await withMcpSpan('mcp.tools_call', {}, () => client.callTool(call))
          return result.content
        }
      })

      toolApproval[modelName] = (input: unknown) => approvalForCapability(
        capabilityFactsForToolCall({
          toolId: mcpToolId,
          toolName: mcpTool.name,
          input,
          annotations: mcpTool.annotations,
          trustedProvenance
        }),
        approvals[mcpToolId],
        permissionMode
      ).outcome
    }
  }

  return {
    tools,
    toolApproval,
    close: async () => {
      await Promise.all(clients.map(c => c.close().catch((err: unknown) => logger.error('[mcp-tools] error closing client', err))))
    },
    toolCallCount: () => toolCalls,
    subagentStop: async (parentSessionId: string, childSessionId: string, status: string) => {
      const relay = clients.find(client => client.trustedProvenance === 'first-party-relay' && client.subagentStop)
      return enforceSubagentStop(relay, parentSessionId, childSessionId, status)
    }
  }
}
