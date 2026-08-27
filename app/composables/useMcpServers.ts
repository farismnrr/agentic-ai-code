import type { McpOAuthDiscovery, McpOAuthStartResult, McpRemoteConfig, McpScanResult, McpServer, McpServerUpdateInput, McpTool } from '#shared/types/chat'

/** SSR-safe MCP connection registry for user-scoped remote servers. */
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

  function replaceServer(next: McpServer) {
    servers.value = servers.value.map(server => server.id === next.id ? next : server)
  }

  async function loadAll() {
    const fetch = import.meta.server ? useRequestFetch() : $fetch
    servers.value = await fetch<McpServer[]>('/api/mcp-servers')
  }

  async function discoverOAuth(url: string) {
    return $fetch<McpOAuthDiscovery>('/api/mcp-servers/oauth-discovery', {
      method: 'POST',
      body: { url }
    })
  }

  async function startOAuth(url: string) {
    return $fetch<McpOAuthStartResult>('/api/mcp-servers/oauth-start', {
      method: 'POST',
      body: { url }
    })
  }

  async function scan(config: McpRemoteConfig) {
    return $fetch<McpScanResult>('/api/mcp-servers/scan', {
      method: 'POST',
      body: config
    })
  }

  async function create(config: McpRemoteConfig) {
    const data = await $fetch<McpServer>('/api/mcp-servers', {
      method: 'POST',
      body: config
    })
    servers.value = [...servers.value, data]
    return data
  }

  async function update(id: string, updates: McpServerUpdateInput) {
    const data = await $fetch<McpServer>(`/api/mcp-servers/${id}`, {
      method: 'PUT',
      body: updates
    })
    replaceServer(data)
    return data
  }

  async function setEnabled(id: string, enabled: boolean) {
    return update(id, { enabled })
  }

  async function test(id: string) {
    try {
      const result = await $fetch<{ id: string, status: McpServer['status'], tools: McpTool[] }>(`/api/mcp-servers/${id}/test`, {
        method: 'POST'
      })
      servers.value = servers.value.map(server =>
        server.id === id
          ? { ...server, status: result.status, tools: result.tools }
          : server
      )
      return result
    } catch (error) {
      servers.value = servers.value.map(server =>
        server.id === id ? { ...server, status: 'error', tools: [] } : server
      )
      throw error
    }
  }

  async function remove(id: string) {
    await $fetch(`/api/mcp-servers/${id}`, { method: 'DELETE' })
    servers.value = servers.value.filter(server => server.id !== id)
  }

  return { servers, availableTools, toolsById, loadAll, discoverOAuth, startOAuth, scan, create, update, setEnabled, test, remove }
}
