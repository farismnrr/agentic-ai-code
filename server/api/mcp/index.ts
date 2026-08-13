import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import { SSEServerTransport } from '@modelcontextprotocol/sdk/server/sse.js'
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js'
import { verifyApiKey } from '../../utils/api-key'
import { getSettings, updateSettings, listWorkspaces, createWorkspace, updateWorkspace, deleteWorkspace, listMcpServers, createMcpServer, updateMcpServer, deleteMcpServer, listConversationMessages, sendMessage } from '../../application/features'

// We store active transports keyed by their SDK-generated session ID
// Note: This in-memory map means connections won't survive across Nitro workers
// in a multi-process deployment. This is acceptable for single-operator deployments
// (the current target), but should be moved to Redis/DB if scaling horizontally.
const transports = new Map<string, SSEServerTransport>()

export default defineEventHandler(async (event) => {
  const userId = await verifyApiKey(event)
  const method = event.method

  if (method === 'GET') {
    const transport = new SSEServerTransport('/api/mcp', event.node.res)
    const server = new Server({ name: 'ai-code', version: '1.0.0' }, { capabilities: { tools: {} } })

    server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: [
          { name: 'get_settings', description: 'Get user settings', inputSchema: { type: 'object', properties: {} } },
          { name: 'update_settings', description: 'Update user settings', inputSchema: { type: 'object', properties: { language: { type: 'string' }, streaming: { type: 'boolean' }, sendOnEnter: { type: 'boolean' }, defaultModelId: { type: 'string' }, temperature: { type: 'number' }, systemPrompt: { type: 'string' }, displayName: { type: 'string' }, email: { type: 'string' } } } },
          { name: 'list_workspaces', description: 'List workspaces', inputSchema: { type: 'object', properties: {} } },
          { name: 'create_workspace', description: 'Create workspace', inputSchema: { type: 'object', properties: { name: { type: 'string' }, path: { type: 'string' } }, required: ['name', 'path'] } },
          { name: 'update_workspace', description: 'Update workspace', inputSchema: { type: 'object', properties: { id: { type: 'string' }, name: { type: 'string' }, path: { type: 'string' } }, required: ['id', 'name'] } },
          { name: 'delete_workspace', description: 'Delete workspace', inputSchema: { type: 'object', properties: { id: { type: 'string' } }, required: ['id'] } },
          { name: 'list_mcp_servers', description: 'List connected MCP servers', inputSchema: { type: 'object', properties: {} } },
          { name: 'create_mcp_server', description: 'Create MCP server', inputSchema: { type: 'object', properties: { name: { type: 'string' }, description: { type: 'string' }, transport: { type: 'string' }, url: { type: 'string' }, command: { type: 'string' } }, required: ['name', 'transport'] } },
          { name: 'update_mcp_server', description: 'Update MCP server', inputSchema: { type: 'object', properties: { id: { type: 'string' }, name: { type: 'string' }, description: { type: 'string' }, transport: { type: 'string' }, url: { type: 'string' }, command: { type: 'string' }, status: { type: 'string' }, enabled: { type: 'boolean' } }, required: ['id'] } },
          { name: 'delete_mcp_server', description: 'Delete MCP server', inputSchema: { type: 'object', properties: { id: { type: 'string' } }, required: ['id'] } },
          { name: 'send_message', description: 'Send a message to a conversation', inputSchema: { type: 'object', properties: { conversationId: { type: 'string' }, text: { type: 'string' } }, required: ['conversationId', 'text'] } },
          { name: 'list_messages', description: 'List messages in a conversation', inputSchema: { type: 'object', properties: { conversationId: { type: 'string' } }, required: ['conversationId'] } }
        ]
      }
    })

    server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const name = request.params.name
      const args = request.params.arguments || {}

      try {
        if (name === 'get_settings') {
          const res = await getSettings(userId)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'update_settings') {
          const res = await updateSettings(userId, args)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'list_workspaces') {
          const res = await listWorkspaces(userId)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'create_workspace') {
          const res = await createWorkspace(userId, args.name as string, args.path as string)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'update_workspace') {
          const res = await updateWorkspace(userId, args.id as string, args.name as string, args.path as string)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'delete_workspace') {
          const res = await deleteWorkspace(userId, args.id as string)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'list_mcp_servers') {
          const res = await listMcpServers(userId)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'create_mcp_server') {
          const res = await createMcpServer(userId, args as { name: string, description?: string, transport: 'http' | 'stdio' | 'sse', url?: string, command?: string })
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'update_mcp_server') {
          const res = await updateMcpServer(userId, args.id as string, args)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'delete_mcp_server') {
          const res = await deleteMcpServer(userId, args.id as string)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'send_message') {
          const res = await sendMessage(userId, args.conversationId as string, args.text as string)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        } else if (name === 'list_messages') {
          const res = await listConversationMessages(userId, args.conversationId as string)
          return { content: [{ type: 'text', text: JSON.stringify(res, null, 2) }] }
        }

        throw new Error(`Unknown tool: ${name}`)
      } catch (err: unknown) {
        return { content: [{ type: 'text', text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true }
      }
    })

    await server.connect(transport)
    transports.set(transport.sessionId, transport)

    event.node.req.on('close', () => {
      transports.delete(transport.sessionId)
    })

    event._handled = true
    return
  } else if (method === 'POST') {
    const sessionId = getQuery(event).sessionId as string
    if (!sessionId) throw badRequest('Missing sessionId')
    const transport = transports.get(sessionId)
    if (!transport) throw notFound('Session not found')

    await transport.handlePostMessage(event.node.req, event.node.res)
    event._handled = true
    return
  }

  throw createError({ statusCode: 405, message: 'Method not allowed' })
})
