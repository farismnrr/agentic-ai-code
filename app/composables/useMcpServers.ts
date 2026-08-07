import type { McpServer, McpTool } from '#shared/types/chat'

/**
 * In-memory MCP server registry. Stands in for what an MCP client would
 * report; `useState` for the same SSR-safety reason as the conversation store.
 */
export function useMcpServers() {
  const servers = useState<McpServer[]>('mcp-servers', () => [])

  /** Only connected, enabled servers can contribute tools to a conversation. */
  const availableTools = computed<McpTool[]>(() =>
    servers.value
      .filter(server => server.enabled && server.status === 'connected')
      .flatMap(server => server.tools)
  )

  const toolsById = computed<Record<string, McpTool>>(() =>
    Object.fromEntries(servers.value.flatMap(s => s.tools).map(t => [t.id, t]))
  )

  async function loadAll() {
    const fetch = import.meta.server ? useRequestFetch() : $fetch
    const data = await fetch<McpServer[]>('/api/mcp-servers')
    servers.value = data
  }

  async function setEnabled(id: string, enabled: boolean) {
    servers.value = servers.value.map(server =>
      server.id === id ? { ...server, enabled } : server
    )
    await $fetch(`/api/mcp-servers/${id}`, {
      method: 'PUT',
      body: { enabled }
    })
  }

  async function add(server: Omit<McpServer, 'id' | 'tools' | 'status'>) {
    const data = await $fetch<McpServer>('/api/mcp-servers', {
      method: 'POST',
      body: server
    })
    servers.value = [...servers.value, data]
  }

  async function remove(id: string) {
    servers.value = servers.value.filter(server => server.id !== id)
    await $fetch(`/api/mcp-servers/${id}`, { method: 'DELETE' })
  }

  return { servers, availableTools, toolsById, loadAll, setEnabled, add, remove }
}
