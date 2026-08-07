import type { McpServer, McpTool } from '~/types/chat'
import { mcpServers as seedServers } from '~/utils/fixtures/mcp-servers'

/**
 * In-memory MCP server registry. Stands in for what an MCP client would
 * report; `useState` for the same SSR-safety reason as the conversation store.
 */
export function useMcpServers() {
  const servers = useState<McpServer[]>('mcp-servers', () => [...seedServers])

  /** Only connected, enabled servers can contribute tools to a conversation. */
  const availableTools = computed<McpTool[]>(() =>
    servers.value
      .filter(server => server.enabled && server.status === 'connected')
      .flatMap(server => server.tools)
  )

  const toolsById = computed<Record<string, McpTool>>(() =>
    Object.fromEntries(servers.value.flatMap(s => s.tools).map(t => [t.id, t]))
  )

  function setEnabled(id: string, enabled: boolean) {
    servers.value = servers.value.map(server =>
      server.id === id ? { ...server, enabled } : server
    )
  }

  function add(server: Omit<McpServer, 'id' | 'tools' | 'status'>) {
    const id = server.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')
      || `server-${Date.now().toString(36)}`
    servers.value = [
      ...servers.value,
      // A real client discovers tools after connecting; there's nothing to
      // discover here, so a new server starts connected with no tools.
      { ...server, id, status: 'connected', tools: [] }
    ]
  }

  function remove(id: string) {
    servers.value = servers.value.filter(server => server.id !== id)
  }

  return { servers, availableTools, toolsById, setEnabled, add, remove }
}
