import type { McpServer, McpTool } from '#shared/types/chat'

/**
 * Stand-in for what an MCP client would report after connecting. Shapes follow
 * the real protocol loosely — server identity, transport, and a tool list — so
 * the UI built against this survives being wired to an actual client.
 */
function tool(
  serverId: string,
  name: string,
  description: string,
  sampleInput: Record<string, unknown>
): McpTool {
  return { id: `${serverId}.${name}`, serverId, name, description, sampleInput }
}

export const mcpServers: McpServer[] = [
  {
    id: 'filesystem',
    name: 'Filesystem',
    description: 'Read and search files in allowed directories.',
    transport: 'stdio',
    command: 'npx -y @modelcontextprotocol/server-filesystem ~/Projects',
    status: 'connected',
    enabled: true,
    tools: [
      tool('filesystem', 'read_file', 'Read the contents of a file.', { path: 'app/app.vue' }),
      tool('filesystem', 'list_directory', 'List entries in a directory.', { path: 'app' }),
      tool('filesystem', 'search_files', 'Search for files matching a pattern.', {
        path: 'app',
        pattern: '*.vue'
      })
    ]
  },
  {
    id: 'github',
    name: 'GitHub',
    description: 'Search repositories, read issues and pull requests.',
    transport: 'http',
    url: 'https://api.githubcopilot.com/mcp',
    status: 'connected',
    enabled: true,
    tools: [
      tool('github', 'search_repositories', 'Search public repositories.', { query: 'nuxt ui' }),
      tool('github', 'get_issue', 'Fetch a single issue by number.', {
        owner: 'nuxt',
        repo: 'ui',
        issue_number: 1
      }),
      tool('github', 'list_pull_requests', 'List pull requests on a repository.', {
        owner: 'nuxt',
        repo: 'ui',
        state: 'open'
      })
    ]
  },
  {
    id: 'nuxt-ui',
    name: 'Nuxt UI',
    description: 'Component APIs, examples and icon search for Nuxt UI.',
    transport: 'http',
    url: 'https://ui.nuxt.com/mcp',
    status: 'connected',
    enabled: true,
    tools: [
      tool('nuxt-ui', 'search_components', 'Find components by name or description.', {
        search: 'chat'
      }),
      tool('nuxt-ui', 'get_component', 'Full documentation for one component.', {
        componentName: 'ChatPrompt'
      }),
      tool('nuxt-ui', 'search_icons', 'Search Iconify icons.', { query: 'server' })
    ]
  },
  {
    id: 'postgres',
    name: 'Postgres',
    description: 'Query a read-only replica.',
    transport: 'stdio',
    command: 'npx -y @modelcontextprotocol/server-postgres $DATABASE_URL',
    status: 'error',
    enabled: false,
    tools: [
      tool('postgres', 'query', 'Run a read-only SQL query.', {
        sql: 'select count(*) from users'
      })
    ]
  }
]

/** Flat lookup across every server, including disabled ones. */
export const mcpToolsById: Record<string, McpTool> = Object.fromEntries(
  mcpServers.flatMap(server => server.tools).map(t => [t.id, t])
)

/** Tool ids switched on by default in a new conversation: every connected server. */
export const defaultEnabledToolIds: string[] = mcpServers
  .filter(server => server.enabled && server.status === 'connected')
  .flatMap(server => server.tools.map(t => t.id))
