import type {
  CreativeCanvasInvokeInput,
  CreativeCanvasInvokeResult
} from '../../application/creative-canvas'
import { findUserWorkspace } from '../database/workspaces'
import { resolveMcpExecutionContext } from '../mcp/capabilities'
import { loadEnabledMcpServers } from '../mcp/server-config'
import { createMcpClient, type McpClientLike } from '../mcp/client'

const GRAPH_ACTIONS = new Set([
  'validate',
  'execute',
  'partial_rerun',
  'rerun_selected',
  'rerun_subgraph',
  'rerun_all_dirty',
  'get',
  'template_save',
  'template_get',
  'template_list',
  'template_instantiate'
])

const PROJECT_ACTIONS = new Set([
  'get',
  'scene_get',
  'game_get',
  'audio_get',
  'qa_list'
])

function allowedAction(input: CreativeCanvasInvokeInput) {
  const action = typeof input.arguments.action === 'string' ? input.arguments.action : ''
  const allowed = input.tool === 'creative_graph' ? GRAPH_ACTIONS : PROJECT_ACTIONS
  if (!allowed.has(action)) {
    throw createError({ statusCode: 400, statusMessage: 'Creative Canvas action is not allowed' })
  }
}

async function firstPartyClientForTool(userId: string, toolName: CreativeCanvasInvokeInput['tool']) {
  const execution = await resolveMcpExecutionContext(userId)
  const suffix = `.${toolName}`
  const serverIds = execution.enabledToolIds
    .filter(id => id.endsWith(suffix))
    .map(id => id.slice(0, -suffix.length))
  const servers = await loadEnabledMcpServers(userId, [...new Set(serverIds)])

  const candidates: McpClientLike[] = []
  try {
    for (const server of servers) {
      const client = await createMcpClient(server)
      if (client.trustedProvenance !== 'first-party-relay') {
        await client.close().catch(() => {})
        continue
      }
      const listed = await client.listTools()
      if (!listed.tools.some(tool => tool.name === toolName)) {
        await client.close().catch(() => {})
        continue
      }
      candidates.push(client)
    }
    if (candidates.length !== 1) {
      throw createError({
        statusCode: candidates.length === 0 ? 409 : 500,
        statusMessage: candidates.length === 0
          ? 'Creative Canvas requires one enabled first-party Creative MCP relay'
          : 'Creative Canvas relay selection is ambiguous'
      })
    }
    return candidates[0]!
  } catch (error) {
    await Promise.all(candidates.map(client => client.close().catch(() => {})))
    throw error
  }
}

export async function invokeCreativeCanvas(
  userId: string,
  input: CreativeCanvasInvokeInput
): Promise<CreativeCanvasInvokeResult> {
  if (!input.workspaceId || !input.arguments || Array.isArray(input.arguments)) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid Creative Canvas request' })
  }
  allowedAction(input)
  if ('cwd' in input.arguments) {
    throw createError({ statusCode: 400, statusMessage: 'Creative Canvas cwd is server-owned' })
  }

  const workspace = await findUserWorkspace(userId, input.workspaceId)
  if (!workspace.pathConfirmed) {
    throw createError({ statusCode: 409, statusMessage: 'Workspace path is not confirmed' })
  }
  const client = await firstPartyClientForTool(userId, input.tool)
  try {
    const result = await client.callTool({
      name: input.tool,
      arguments: {
        ...input.arguments,
        cwd: workspace.path
      }
    })
    return {
      tool: input.tool,
      content: result.content,
      isError: result.isError === true
    }
  } finally {
    await client.close().catch(() => {})
  }
}
